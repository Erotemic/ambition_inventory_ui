//! The ONE canonical bevy_lunex 3D-cube inventory renderer (#31).
//!
//! Generic over the host's `PageId`/`Action`: the host publishes the ordered
//! faces via [`ActiveMenuPages`], and [`CubeMenuPlugin`] spawns a pause
//! `Camera3d` + a ring of bevy_lunex faces, rebuilds them when the pages change,
//! rotates the ring so the active face turns to the camera, and folds the faces
//! open/closed in the OoT "subscreen" style.
//!
//! This module is the consolidation of what used to be two drifted copies (the
//! `ambition_mock_demo` private cube and an earlier lib re-port). The demo's
//! look/fold/rotation/button-layout is the visual reference and is reproduced
//! here faithfully, generalized over N pages.
//!
//! ## Tuning seam
//! All geometry/speeds/visual knobs live in [`CubeMenuConfig`] (a `Resource`).
//! The plugin inserts a default if the host has not; the host (or demo) may
//! insert its own before adding the plugin to match its exact values.

use std::marker::PhantomData;
use std::sync::Arc;

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy_lunex::prelude::*;

use crate::{
    ActiveMenuPages, AmbitionMenuControl, AmbitionMenuPage, AmbitionMenuRoot, MenuColor,
    MenuControlKind, MenuCubeGeometry, MenuFocusKey, MenuNode, MenuOpenCloseStyle, MenuPageModel,
    MenuRect, MenuTextAlign, MenuVisualState,
};

// Depth bands on each Lunex face (more negative = closer to the pause camera).
// Ported verbatim from the demo's `app.rs` so the layered look matches.
const DEPTH_BACKGROUND: f32 = -0.04;
const DEPTH_LARGE_PANEL: f32 = -0.16;
const DEPTH_CARD: f32 = -0.32;
const DEPTH_ACTION: f32 = -0.50;
const DEPTH_EDGE: f32 = -0.68;
const DEPTH_TEXT_TOP: f32 = -0.96;
const DEPTH_SELECTION: f32 = -1.12;
const FONT_FAMILY: &str = "DejaVu Sans";

/// Marks the rotating ring root that holds the cube faces.
#[derive(Component)]
pub struct MenuRing;

/// Marks the dedicated pause camera that frames the cube.
#[derive(Component)]
pub struct CubePauseCamera;

/// Non-generic marker on each cube face plus the face's base ring placement.
///
/// Stored at build time so the per-frame OoT page-fold can recompute each face's
/// transform from its (immutable) base without corrupting it. A non-generic
/// component lets the fold/animation systems query faces without being generic
/// over the host's `PageId`.
#[derive(Component)]
pub struct CubeFace {
    /// Index of this face on the ring.
    pub index: usize,
    /// The face's ring angle (radians), source of the position-derived fold axis.
    pub angle: f32,
    /// The face's base translation on the ring (no fold applied).
    pub base_translation: Vec3,
    /// The face's base rotation on the ring (no fold applied).
    pub base_rotation: Quat,
    /// The face's base scale (carries the inside-of-cube X flip).
    pub base_scale: Vec3,
    /// Half-height of the face, for the bottom-edge hinge.
    pub half_height: f32,
}

/// Eased open amount for the cube menu (0 = folded shut, 1 = laid flat/open).
///
/// The host sets [`CubeOpenState::target`]; [`animate_cube_ring`] eases `amount`
/// toward it each frame and folds the faces accordingly. The host also reads
/// `amount` to drive camera/visibility so the close animation is visible.
#[derive(Resource, Default)]
pub struct CubeOpenState {
    pub amount: f32,
    pub target: f32,
}

/// All tuning knobs for the canonical cube, shared by the demo and the game.
///
/// The plugin inserts [`CubeMenuConfig::default`] if absent. A host that wants
/// the demo's exact look (e.g. the mock demo itself) inserts its own values
/// before adding the plugin.
#[derive(Resource, Clone, Debug)]
pub struct CubeMenuConfig {
    /// Cube/page geometry (radius, face size, camera placement).
    pub geometry: MenuCubeGeometry,
    /// How far a face folds away from the ring when fully closed (radians).
    pub fold_radians: f32,
    /// Ease speed for the open/close fold.
    pub open_close_speed: f32,
    /// Ease speed for the active-page ring rotation snap.
    pub page_rotate_speed: f32,
    /// Open/close presentation: page-fold (OoT) or a simple scale.
    pub open_close_style: MenuOpenCloseStyle,
    /// Inside-of-cube horizontal flip so face content reads correctly (-1.0).
    pub inside_x_flip: f32,
    /// Minimum ring scale at fully-closed when using [`MenuOpenCloseStyle::SmoothScale`].
    pub min_open_scale: f32,
    /// Draw the bright cube-edge frame around each face (demo look).
    pub draw_edge_frame: bool,
    /// Draw white selection corner-brackets around the selected control (demo look).
    pub draw_selection_corners: bool,
    /// Draw the left/right page-navigation affordance buttons on each face (the
    /// L/R "switch subscreen" arrows). Decorative-only in the lib (the host owns
    /// the actual page cycling via input); they communicate the affordance and
    /// match the demo's look. Default `true` so both the demo and the game get them.
    pub draw_nav_arrows: bool,
    /// Camera `order` for the cube's `Camera3d`.
    pub camera_order: isize,
    /// Whether the cube camera clears the screen (game overlay wants `None`).
    pub camera_clears: bool,
    /// Whether the cube camera starts active. The game gates this off and toggles
    /// it itself; a standalone demo can start it on.
    pub camera_starts_active: bool,
    /// Whether the ring starts visible. The game gates this off; the demo shows it.
    pub ring_starts_visible: bool,
}

impl Default for CubeMenuConfig {
    fn default() -> Self {
        Self {
            geometry: MenuCubeGeometry::default(),
            fold_radians: 1.60,
            open_close_speed: 8.0,
            page_rotate_speed: 5.2,
            open_close_style: MenuOpenCloseStyle::OotPageFold,
            inside_x_flip: -1.0,
            min_open_scale: 0.64,
            draw_edge_frame: true,
            draw_selection_corners: true,
            draw_nav_arrows: true,
            // Game-overlay defaults (see module docs in `oot_cube_app.rs`): the
            // cube camera must NOT clear, must NOT start active, and the ring must
            // start hidden — the host gates them on when the menu opens.
            camera_order: 8,
            camera_clears: false,
            camera_starts_active: false,
            ring_starts_visible: false,
        }
    }
}

/// Plugin: spawns the cube camera + ring and rebuilds faces from
/// `ActiveMenuPages<PageId, Action>`. Add once with the host's page/action types.
pub struct CubeMenuPlugin<PageId, Action> {
    _marker: PhantomData<fn() -> (PageId, Action)>,
}

impl<PageId, Action> Default for CubeMenuPlugin<PageId, Action> {
    fn default() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<PageId, Action> Plugin for CubeMenuPlugin<PageId, Action>
where
    PageId: Clone + PartialEq + Send + Sync + 'static,
    Action: Clone + Send + Sync + 'static,
{
    fn build(&self, app: &mut App) {
        app.add_plugins(UiLunexPlugins)
            .init_resource::<CubeOpenState>();
        if !app.world().contains_resource::<CubeMenuConfig>() {
            app.insert_resource(CubeMenuConfig::default());
        }
        app.add_systems(Startup, setup_cube)
            .add_systems(
                Update,
                (
                    rebuild_cube_faces::<PageId, Action>,
                    animate_cube_ring::<PageId, Action>,
                ),
            );
    }
}

fn setup_cube(mut commands: Commands, config: Res<CubeMenuConfig>) {
    let geo = config.geometry;
    commands.spawn((
        Name::new("Cube pause camera"),
        CubePauseCamera,
        Camera3d::default(),
        Camera {
            order: config.camera_order,
            // Host-gated by default: OFF until the host activates the menu. An
            // active higher-order camera otherwise clears the whole screen every
            // frame, hiding the lower-order game cameras.
            is_active: config.camera_starts_active,
            // Transparent clear (Option 1 overlay) keeps the live game world
            // visible behind the cube. A standalone demo flips `camera_clears` on.
            clear_color: if config.camera_clears {
                ClearColorConfig::default()
            } else {
                ClearColorConfig::None
            },
            ..default()
        },
        RenderLayers::layer(0),
        // NO explicit Msaa: a Camera3d overlaying a Camera2d on the same window must
        // share its sample count or it renders its clear but drops all geometry. The
        // host's Camera2d uses the default (Msaa::Sample4); omitting Msaa here
        // inherits that same default so they match.
        Transform::from_translation(Vec3::new(0.0, geo.camera_y, -geo.camera_distance))
            .looking_at(Vec3::new(0.0, geo.look_y, 0.0), Vec3::Y),
    ));
    commands.spawn((
        Name::new("Cube menu ring"),
        AmbitionMenuRoot,
        MenuRing,
        UiRoot3d,
        Transform::default(),
        if config.ring_starts_visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        },
        RenderLayers::layer(0),
    ));
}

/// Rebuild the ring's faces whenever the host's published pages change.
fn rebuild_cube_faces<PageId, Action>(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<CubeMenuConfig>,
    pages: Option<Res<ActiveMenuPages<PageId, Action>>>,
    ring_query: Query<Entity, With<MenuRing>>,
    faces: Query<Entity, With<AmbitionMenuPage<PageId>>>,
    mut last_version: Local<Option<u64>>,
    mut dirty: Local<bool>,
) where
    PageId: Clone + PartialEq + Send + Sync + 'static,
    Action: Clone + Send + Sync + 'static,
{
    let Some(pages) = pages else {
        return;
    };
    // Rebuild on version bump (host republish) or first run. Cheap: page models
    // are small and rebuilt only when the host changes them.
    if !pages.is_changed() && !*dirty && *last_version == Some(pages.version) {
        return;
    }
    *dirty = false;
    *last_version = Some(pages.version);

    for face in &faces {
        commands.entity(face).despawn();
    }
    let Ok(ring) = ring_query.single() else {
        warn!("cube: ring entity not found yet — deferring face rebuild");
        *dirty = true;
        return;
    };
    info!(
        "cube: rebuilding {} face(s) (active page present: {})",
        pages.pages.len(),
        pages.active.is_some()
    );
    let geo = config.geometry;
    let n = pages.pages.len().max(1) as f32;
    let flip = config.inside_x_flip;
    commands.entity(ring).with_children(|ring| {
        for (i, model) in pages.pages.iter().enumerate() {
            let active = pages.active.as_ref() == Some(&model.id);
            let angle = (i as f32) * std::f32::consts::TAU / n;
            let pos = Vec3::new(angle.sin() * geo.page_radius, 0.0, angle.cos() * geo.page_radius);
            let rot = Quat::from_rotation_y(angle);
            let scale = Vec3::new(flip, 1.0, 1.0);
            let mut face = ring.spawn((
                Name::new("Cube face"),
                AmbitionMenuPage {
                    id: model.id.clone(),
                    active,
                },
                CubeFace {
                    index: i,
                    angle,
                    base_translation: pos,
                    base_rotation: rot,
                    base_scale: scale,
                    half_height: geo.page_height * 0.5,
                },
                UiRoot3d,
                // bevy_lunex needs a layout root + a Dimension on each face for the
                // child UiLayout::window() planes to resolve their Rl/Rh sizes.
                UiLayoutRoot::new_3d(),
                Dimension::from((geo.page_width, geo.page_height)),
                Transform::from_translation(pos)
                    .with_rotation(rot)
                    .with_scale(scale),
                Visibility::Visible,
                RenderLayers::layer(0),
            ));
            face.with_children(|ui| render_page_model(ui, &mut materials, &config, model, active));
        }
    });
}

/// Drive the whole ring per frame: ease the open amount, snap the ring rotation
/// to the active face, apply the open/close presentation, and (in OoT style)
/// fold every face about its bottom edge.
///
/// Ported from the demo's `animate_menu_ring` + `apply_oot_open_fold`,
/// generalized over N pages.
fn animate_cube_ring<PageId, Action>(
    time: Res<Time>,
    config: Res<CubeMenuConfig>,
    mut state: ResMut<CubeOpenState>,
    pages: Option<Res<ActiveMenuPages<PageId, Action>>>,
    mut ring: Query<&mut Transform, (With<MenuRing>, Without<CubeFace>)>,
    mut faces: Query<(&CubeFace, &mut Transform), Without<MenuRing>>,
) where
    PageId: PartialEq + Send + Sync + 'static,
    Action: Send + Sync + 'static,
{
    let Ok(mut ring_t) = ring.single_mut() else {
        return;
    };
    let Some(pages) = pages else {
        return;
    };
    let n = pages.pages.len().max(1) as f32;

    // Ease the open amount toward the host's target (demo's exp ease).
    let open_step = 1.0 - (-config.open_close_speed * time.delta_secs()).exp();
    state.amount += (state.target - state.amount) * open_step;
    if (state.amount - state.target).abs() < 0.002 {
        state.amount = state.target;
    }
    let open = smoothstep(state.amount.clamp(0.0, 1.0));

    // Snap the ring so the active face turns to the camera (OoT page turn).
    let active_idx = pages
        .active
        .as_ref()
        .and_then(|a| pages.pages.iter().position(|p| &p.id == a))
        .unwrap_or(0) as f32;
    let target = Quat::from_rotation_y(-active_idx * std::f32::consts::TAU / n);
    let rotate_step = (time.delta_secs() * config.page_rotate_speed).clamp(0.0, 1.0);
    let spin = ring_t.rotation.slerp(target, rotate_step);

    match config.open_close_style {
        MenuOpenCloseStyle::SmoothScale => {
            let scale = config.min_open_scale + (1.0 - config.min_open_scale) * open;
            ring_t.rotation = spin;
            ring_t.scale = Vec3::splat(scale);
            ring_t.translation = Vec3::new(0.0, -0.05 * (1.0 - open), -0.42 * (1.0 - open));
            for (face, mut t) in &mut faces {
                reset_face_transform(face, &mut t);
            }
        }
        MenuOpenCloseStyle::OotPageFold => {
            ring_t.rotation = spin;
            ring_t.scale = Vec3::ONE;
            ring_t.translation = Vec3::new(0.0, -0.10 * (1.0 - open), 0.0);
            let fold = config.fold_radians * (1.0 - open);
            for (face, mut t) in &mut faces {
                apply_face_fold(face, fold, &mut t);
            }
        }
    }
}

/// Restore a face to its unfolded base placement (used by the scale style).
fn reset_face_transform(face: &CubeFace, transform: &mut Transform) {
    transform.translation = face.base_translation;
    transform.rotation = face.base_rotation;
    transform.scale = face.base_scale;
}

/// Generalized port of the demo's `apply_oot_open_fold`.
///
/// The demo's n=4 cardinal mapping folds each face about a horizontal axis in
/// *ring space* (the parent frame), pinning the face's bottom edge as a hinge:
///
/// | page   | ring angle θ | demo fold axis | `(cosθ, 0, -sinθ)` |
/// |--------|--------------|----------------|---------------------|
/// | Items  | 0°           | +X             | (1, 0, 0)           |
/// | Map    | 90°          | -Z             | (0, 0, -1)          |
/// | Quest  | 180°         | -X             | (-1, 0, 0)          |
/// | System | 270°         | +Z             | (0, 0, 1)           |
///
/// So the fold axis is exactly the ring-space tangent `(cosθ, 0, -sinθ)` — the
/// horizontal direction along the bottom edge of the face — with a single
/// positive `fold`. This reproduces the demo for n=4 AND generalizes to any N
/// (the axis is derived from the face's own ring angle, not a hardcoded enum).
/// The fold is pre-multiplied (`fold_rotation * base_rotation`) so it acts in
/// ring space, exactly like the demo.
fn apply_face_fold(face: &CubeFace, fold: f32, transform: &mut Transform) {
    let axis = Vec3::new(face.angle.cos(), 0.0, -face.angle.sin());
    let fold_rotation = Quat::from_axis_angle(axis, fold);
    let rotation = fold_rotation * face.base_rotation;
    // Pin the bottom edge of the page (hinge), exactly like the demo.
    let hinge_local = Vec3::new(0.0, -face.half_height, 0.0);
    let hinge_world = face.base_translation + face.base_rotation * hinge_local;
    let translation = hinge_world - rotation * hinge_local;
    transform.translation = translation;
    transform.rotation = rotation;
    transform.scale = face.base_scale;
}

/// OoT-style smoothstep ease (matches the demo's `smoothstep`).
fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn render_page_model<PageId, Action>(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    config: &CubeMenuConfig,
    model: &MenuPageModel<PageId, Action>,
    active: bool,
) where
    Action: Clone + Send + Sync + 'static,
{
    // One full-page background at the dedicated background depth.
    spawn_panel(
        ui,
        materials,
        MenuRect::new(0.0, 0.0, 100.0, 100.0),
        menu_color(model.background),
        None::<Action>,
        active,
    );
    if config.draw_edge_frame {
        spawn_cube_edge_frame(ui, materials, active);
    }
    if config.draw_nav_arrows {
        spawn_nav_arrows(ui, materials, active);
    }
    for node in &model.nodes {
        match node {
            MenuNode::Panel { rect, color, action } => {
                spawn_panel(ui, materials, *rect, menu_color(*color), action.clone(), active)
            }
            MenuNode::Text {
                x,
                y,
                size,
                text,
                align,
                color,
            } => spawn_text(
                ui,
                materials,
                *x,
                *y,
                *size,
                text,
                menu_align(*align),
                menu_srgba(*color),
                active,
            ),
            MenuNode::Control {
                rect,
                kind,
                label,
                detail,
                selected,
                important,
                action,
                ..
            } => spawn_control(
                ui,
                materials,
                config,
                *rect,
                *kind,
                label,
                detail.as_deref(),
                *selected,
                *important,
                action.clone(),
                active,
            ),
        }
    }
}

fn spawn_panel<Action>(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    rect: MenuRect,
    color: Color,
    action: Option<Action>,
    active: bool,
) where
    Action: Clone + Send + Sync + 'static,
{
    spawn_panel_at_depth(
        ui,
        materials,
        rect,
        color,
        action.clone(),
        panel_depth(rect, action.is_some()),
        active,
    );
}

fn spawn_panel_at_depth<Action>(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    rect: MenuRect,
    color: Color,
    action: Option<Action>,
    depth: f32,
    active: bool,
) where
    Action: Clone + Send + Sync + 'static,
{
    let material = materials.add(StandardMaterial {
        base_color: opaque_color(color),
        alpha_mode: AlphaMode::Opaque,
        cull_mode: None,
        unlit: true,
        ..default()
    });
    let mut entity = ui.spawn((
        Name::new("panel"),
        UiLayout::window()
            .x(Rl(rect.x))
            .y(Rl(rect.y))
            .width(Rl(rect.w))
            .height(Rh(rect.h))
            .anchor(Anchor::TOP_LEFT)
            .pack(),
        UiDepth::Set(page_depth(depth, active)),
        UiMeshPlane3d,
        MeshMaterial3d(material),
    ));
    if let Some(action) = action {
        entity.insert((
            AmbitionMenuControl {
                kind: MenuControlKind::Action,
                action: Some(action),
                focus: MenuFocusKey::default(),
            },
            MenuVisualState::default(),
        ));
    } else {
        entity.insert(Pickable::IGNORE);
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_text(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    x: f32,
    y: f32,
    size: f32,
    text: &str,
    align: TextAlign,
    color: Srgba,
    active: bool,
) {
    let material = materials.add(StandardMaterial {
        base_color_texture: Some(TextAtlas::DEFAULT_IMAGE),
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        unlit: true,
        ..default()
    });
    ui.spawn((
        Name::new("text"),
        UiLayout::window().x(Rl(x)).y(Rl(y)).anchor(Anchor::CENTER).pack(),
        UiDepth::Set(page_depth(text_depth(y), active)),
        UiTextSize::from(Rh(size)),
        Text3d::new(text.to_string()),
        Text3dStyling {
            size: 64.0,
            color,
            align,
            font: Arc::from(FONT_FAMILY),
            weight: Weight::BOLD,
            ..Default::default()
        },
        MeshMaterial3d(material),
        Mesh3d::default(),
        Pickable::IGNORE,
    ));
}

#[allow(clippy::too_many_arguments)]
fn spawn_control<Action>(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    config: &CubeMenuConfig,
    rect: MenuRect,
    kind: MenuControlKind,
    label: &str,
    detail: Option<&str>,
    selected: bool,
    important: bool,
    action: Option<Action>,
    active: bool,
) where
    Action: Clone + Send + Sync + 'static,
{
    let disabled = action.is_none();
    let color = if disabled {
        disabled_control_color()
    } else {
        control_color(kind, selected, important)
    };
    let material = materials.add(StandardMaterial {
        base_color: opaque_color(color),
        alpha_mode: AlphaMode::Opaque,
        cull_mode: None,
        unlit: true,
        ..default()
    });
    let focus = MenuFocusKey {
        row: (rect.y * 10.0).round() as i32,
        col: (rect.x * 10.0).round() as i32,
        order: (rect.y * 100.0 + rect.x).round() as i32,
    };
    let mut entity = ui.spawn((
        Name::new("control"),
        UiLayout::window()
            .x(Rl(rect.x))
            .y(Rl(rect.y))
            .width(Rl(rect.w))
            .height(Rh(rect.h))
            .anchor(Anchor::TOP_LEFT)
            .pack(),
        UiDepth::Set(page_depth(panel_depth(rect, action.is_some()), active)),
        UiMeshPlane3d,
        MeshMaterial3d(material),
        AmbitionMenuControl {
            kind,
            action,
            focus,
        },
        MenuVisualState {
            focused: selected,
            selected,
            disabled,
            ..Default::default()
        },
    ));
    if disabled {
        entity.insert(Pickable::IGNORE);
    }
    let draw_corners = config.draw_selection_corners;
    entity.with_children(|children| {
        if selected && draw_corners {
            spawn_selection_corners(children, materials, active);
        }
        let main_size = if matches!(kind, MenuControlKind::Item) { 20.0 } else { 22.0 };
        spawn_text(
            children,
            materials,
            50.0,
            44.0,
            main_size,
            label,
            TextAlign::Center,
            Srgba::rgb_u8(242, 234, 200),
            active,
        );
        if let Some(detail) = detail {
            spawn_text(
                children,
                materials,
                50.0,
                76.0,
                10.5,
                detail,
                TextAlign::Center,
                Srgba::rgb_u8(185, 196, 210),
                active,
            );
        }
    });
}

fn spawn_selection_corners(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    active: bool,
) {
    let color = Color::WHITE;
    let l = 23.0;
    let t = 6.0;
    spawn_corner_piece(ui, materials, 0.0, 0.0, l, t, color, active);
    spawn_corner_piece(ui, materials, 0.0, 0.0, t, l, color, active);
    spawn_corner_piece(ui, materials, 100.0 - l, 0.0, l, t, color, active);
    spawn_corner_piece(ui, materials, 100.0 - t, 0.0, t, l, color, active);
    spawn_corner_piece(ui, materials, 0.0, 100.0 - t, l, t, color, active);
    spawn_corner_piece(ui, materials, 0.0, 100.0 - l, t, l, color, active);
    spawn_corner_piece(ui, materials, 100.0 - l, 100.0 - t, l, t, color, active);
    spawn_corner_piece(ui, materials, 100.0 - t, 100.0 - l, t, l, color, active);
}

#[allow(clippy::too_many_arguments)]
fn spawn_corner_piece(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: Color,
    active: bool,
) {
    let material = materials.add(StandardMaterial {
        base_color: opaque_color(color),
        alpha_mode: AlphaMode::Opaque,
        cull_mode: None,
        unlit: true,
        ..default()
    });
    ui.spawn((
        Name::new("selection corner"),
        UiLayout::window()
            .x(Rl(x))
            .y(Rl(y))
            .width(Rl(w))
            .height(Rh(h))
            .anchor(Anchor::TOP_LEFT)
            .pack(),
        UiDepth::Set(page_depth(DEPTH_SELECTION, active)),
        UiMeshPlane3d,
        MeshMaterial3d(material),
        Pickable::IGNORE,
    ));
}

/// Draw the left/right page-navigation affordance buttons on a face (the L/R
/// "switch subscreen" arrows). Ported from the demo's per-face `add_edge_buttons`
/// (same rects/look), but decorative here: the lib is generic over the host's
/// `Action`, and the host already owns page cycling via input. They render the
/// affordance from ONE place so both the demo and the game show them.
fn spawn_nav_arrows(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    active: bool,
) {
    // Match the demo's edge-button placement and the unselected Action color.
    let bg = control_color(MenuControlKind::Action, false, false);
    let left = MenuRect::new(1.8, 43.5, 7.5, 13.0);
    let right = MenuRect::new(90.7, 43.5, 7.5, 13.0);
    spawn_panel_at_depth(ui, materials, left, bg, None::<Action0>, DEPTH_ACTION, active);
    spawn_panel_at_depth(ui, materials, right, bg, None::<Action0>, DEPTH_ACTION, active);
    let glyph = Srgba::rgb_u8(242, 234, 200);
    spawn_text(ui, materials, left.x + left.w * 0.5, left.y + left.h * 0.5, 5.0, "<", TextAlign::Center, glyph, active);
    spawn_text(ui, materials, right.x + right.w * 0.5, right.y + right.h * 0.5, 5.0, ">", TextAlign::Center, glyph, active);
}

fn spawn_cube_edge_frame(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    active: bool,
) {
    let color = Color::srgba(0.80, 0.92, 1.0, 0.62);
    // Cube borders sit in their own deterministic depth band so they do not
    // shimmer against the page/panel edges while the cube rotates.
    spawn_panel_at_depth(ui, materials, MenuRect::new(0.0, 0.0, 100.0, 0.7), color, None::<Action0>, DEPTH_EDGE, active);
    spawn_panel_at_depth(ui, materials, MenuRect::new(0.0, 99.3, 100.0, 0.7), color, None::<Action0>, DEPTH_EDGE, active);
    spawn_panel_at_depth(ui, materials, MenuRect::new(0.0, 0.0, 0.7, 100.0), color, None::<Action0>, DEPTH_EDGE, active);
    spawn_panel_at_depth(ui, materials, MenuRect::new(99.3, 0.0, 0.7, 100.0), color, None::<Action0>, DEPTH_EDGE, active);
}

/// Zero-sized stand-in `Action` for non-interactive decoration spawns (edges).
#[derive(Clone)]
enum Action0 {}

fn page_depth(depth: f32, active: bool) -> f32 {
    if active {
        depth
    } else {
        depth * 0.28
    }
}

fn text_depth(y: f32) -> f32 {
    DEPTH_TEXT_TOP - (y.round() % 37.0) * 0.0008
}

fn opaque_color(color: Color) -> Color {
    let s = color.to_srgba();
    Color::srgb(s.red, s.green, s.blue)
}

fn panel_depth(rect: MenuRect, actionable: bool) -> f32 {
    if actionable {
        return DEPTH_ACTION;
    }
    let near_full_page = rect.w > 98.0 && rect.h > 98.0;
    let edge_bar = rect.w < 1.5 || rect.h < 1.5;
    if near_full_page {
        DEPTH_BACKGROUND
    } else if edge_bar {
        DEPTH_EDGE
    } else if rect.w > 40.0 || rect.h > 35.0 {
        DEPTH_LARGE_PANEL
    } else {
        DEPTH_CARD
    }
}

fn control_color(kind: MenuControlKind, selected: bool, important: bool) -> Color {
    if selected {
        Color::srgba(0.98, 0.76, 0.26, 0.96)
    } else if important {
        Color::srgba(0.13, 0.34, 0.28, 0.96)
    } else {
        match kind {
            MenuControlKind::Item => Color::srgba(0.055, 0.074, 0.155, 0.96),
            MenuControlKind::Scrollbar => Color::srgba(0.42, 0.32, 0.08, 0.92),
            MenuControlKind::Action => Color::srgba(0.09, 0.12, 0.26, 0.96),
            _ => Color::srgba(0.055, 0.070, 0.145, 0.96),
        }
    }
}

fn disabled_control_color() -> Color {
    Color::srgba(0.040, 0.045, 0.075, 0.72)
}

fn menu_color(color: MenuColor) -> Color {
    Color::srgba(color.r, color.g, color.b, color.a)
}

fn menu_srgba(color: MenuColor) -> Srgba {
    Srgba::new(color.r, color.g, color.b, color.a)
}

fn menu_align(align: MenuTextAlign) -> TextAlign {
    match align {
        MenuTextAlign::Left => TextAlign::Left,
        MenuTextAlign::Center => TextAlign::Center,
        MenuTextAlign::Right => TextAlign::Right,
    }
}
