//! Reusable bevy_lunex 3D-cube renderer for `MenuPageModel`s (#31), promoted from
//! `ambition_mock_demo`. Generic over the host's `PageId`/`Action`: the host
//! publishes the ordered faces via [`ActiveMenuPages`], and [`CubeMenuPlugin`]
//! spawns a pause `Camera3d` + a ring of bevy_lunex faces and rebuilds them when
//! the pages change. Each clickable plane carries the host's `Action` via
//! [`AmbitionMenuControl`], so navigation/selection stay host-driven.
//!
//! NOTE: first blind port — visual depth/scale constants come straight from the
//! demo; tune in-game.

use std::marker::PhantomData;
use std::sync::Arc;

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy_lunex::prelude::*;

use crate::{
    ActiveMenuPages, AmbitionMenuControl, AmbitionMenuPage, AmbitionMenuRoot, MenuColor,
    MenuControlKind, MenuCubeGeometry, MenuFocusKey, MenuNode, MenuPageModel, MenuRect,
    MenuTextAlign, MenuVisualState,
};

// Depth bands on each Lunex face (more negative = closer to the pause camera).
const DEPTH_BACKGROUND: f32 = -0.04;
const DEPTH_LARGE_PANEL: f32 = -0.16;
const DEPTH_CARD: f32 = -0.32;
const DEPTH_ACTION: f32 = -0.50;
const DEPTH_EDGE: f32 = -0.68;
const DEPTH_TEXT_TOP: f32 = -0.96;
const FONT_FAMILY: &str = "DejaVu Sans";

/// Matches the mock demo's `OOT_PAGE_FOLD_RADIANS` (`app.rs`): how far a page
/// folds away from the ring when the menu is fully closed.
const OOT_PAGE_FOLD_RADIANS: f32 = 1.60;
/// Ease speed for the open/close fold, matching the demo's `open_close_speed`.
const CUBE_OPEN_SPEED: f32 = 8.0;

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
/// component lets [`animate_cube_open`] query faces without being generic over the
/// host's `PageId`.
#[derive(Component)]
pub struct CubeFace {
    /// Index of this face on the ring, used to derive the fold axis/sign.
    pub index: usize,
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
/// The host sets [`CubeOpenState::target`]; [`animate_cube_open`] eases
/// `amount` toward it each frame and folds the faces accordingly. The host also
/// reads `amount` to drive camera/visibility so the close animation is visible.
#[derive(Resource, Default)]
pub struct CubeOpenState {
    pub amount: f32,
    pub target: f32,
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
            .init_resource::<CubeOpenState>()
            .add_systems(Startup, setup_cube)
            .add_systems(
                Update,
                (
                    rebuild_cube_faces::<PageId, Action>,
                    animate_cube_ring::<PageId, Action>,
                    animate_cube_open,
                ),
            );
    }
}

fn setup_cube(mut commands: Commands) {
    let geo = MenuCubeGeometry::default();
    commands.spawn((
        Name::new("Cube pause camera"),
        CubePauseCamera,
        Camera3d::default(),
        Camera {
            order: 8,
            // Host-gated: OFF until the host activates the menu (e.g. on pause). This
            // order-8 camera otherwise clears the whole screen to black every frame,
            // hiding everything the lower-order game cameras drew.
            is_active: false,
            // Dark backdrop behind the cube — an OoT-style pause room, matching the
            // demo's near-black clear.
            clear_color: ClearColorConfig::Custom(Color::srgb(0.008, 0.009, 0.020)),
            ..default()
        },
        RenderLayers::layer(0),
        // NO explicit Msaa: a Camera3d overlaying a 2D camera on the SAME window must
        // share its sample count, or it renders its clear but drops all geometry. The
        // host's Camera2d uses the default (Msaa::Sample4); omitting Msaa here inherits
        // that same default so they match. (The demo forced Msaa::Off, but it has no
        // 2D camera to mismatch against.)
        Transform::from_translation(Vec3::new(0.0, geo.camera_y, -geo.camera_distance))
            .looking_at(Vec3::new(0.0, geo.look_y, 0.0), Vec3::Y),
    ));
    commands.spawn((
        Name::new("Cube menu ring"),
        AmbitionMenuRoot,
        MenuRing,
        UiRoot3d,
        Transform::default(),
        // Host-gated alongside the camera; the host shows it when opening the menu.
        Visibility::Hidden,
        RenderLayers::layer(0),
    ));
}

/// Rebuild the ring's faces whenever the host's published pages change.
fn rebuild_cube_faces<PageId, Action>(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    pages: Option<Res<ActiveMenuPages<PageId, Action>>>,
    ring_query: Query<Entity, With<MenuRing>>,
    faces: Query<Entity, With<AmbitionMenuPage<PageId>>>,
    mut last_len: Local<usize>,
    mut dirty: Local<bool>,
) where
    PageId: Clone + PartialEq + Send + Sync + 'static,
    Action: Clone + Send + Sync + 'static,
{
    let Some(pages) = pages else {
        return;
    };
    // Rebuild on add/change or first run; cheap heuristic since page models are
    // small and rebuilt only when the host republishes.
    if !pages.is_changed() && !*dirty && *last_len == pages.pages.len() {
        return;
    }
    *dirty = false;
    *last_len = pages.pages.len();

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
    let geo = MenuCubeGeometry::default();
    let n = pages.pages.len().max(1) as f32;
    commands.entity(ring).with_children(|ring| {
        for (i, model) in pages.pages.iter().enumerate() {
            let active = pages.active.as_ref() == Some(&model.id);
            let angle = (i as f32) * std::f32::consts::TAU / n;
            let pos = Vec3::new(angle.sin() * geo.page_radius, 0.0, angle.cos() * geo.page_radius);
            let rot = Quat::from_rotation_y(angle);
            let scale = Vec3::new(-1.0, 1.0, 1.0);
            let mut face = ring.spawn((
                Name::new("Cube face"),
                AmbitionMenuPage {
                    id: model.id.clone(),
                    active,
                },
                // Non-generic marker carrying the immutable base placement so the
                // per-frame OoT page-fold ([`animate_cube_open`]) can recompute the
                // face transform from its base without permanently corrupting it.
                CubeFace {
                    index: i,
                    base_translation: pos,
                    base_rotation: rot,
                    base_scale: scale,
                    half_height: geo.page_height * 0.5,
                },
                UiRoot3d,
                // bevy_lunex needs a layout root + a Dimension on each face for the
                // child UiLayout::window() planes to resolve their Rl/Rh sizes.
                // Without these the planes get zero size and the cube renders black.
                UiLayoutRoot::new_3d(),
                Dimension::from((geo.page_width, geo.page_height)),
                Transform::from_translation(pos)
                    .with_rotation(rot)
                    // Inside-of-cube X flip so face content reads correctly,
                    // matching the demo's INSIDE_PAGE_X_FLIP = -1.0.
                    .with_scale(scale),
                Visibility::Visible,
                RenderLayers::layer(0),
            ));
            face.with_children(|ui| render_page_model(ui, &mut materials, model, active));
        }
    });
}

/// Slowly spin the ring so the active face faces the camera (placeholder feel;
/// the host can drive `ActiveMenuPages::active` to pick the front face).
fn animate_cube_ring<PageId, Action>(
    time: Res<Time>,
    pages: Option<Res<ActiveMenuPages<PageId, Action>>>,
    mut ring: Query<&mut Transform, With<MenuRing>>,
) where
    PageId: PartialEq + Send + Sync + 'static,
    Action: Send + Sync + 'static,
{
    let Ok(mut t) = ring.single_mut() else {
        return;
    };
    let Some(pages) = pages else {
        return;
    };
    let n = pages.pages.len().max(1) as f32;
    // Find the host's active page; rotate the ring so that face turns to the camera.
    let active_idx = pages
        .active
        .as_ref()
        .and_then(|a| pages.pages.iter().position(|p| &p.id == a))
        .unwrap_or(0) as f32;
    let target = Quat::from_rotation_y(-active_idx * std::f32::consts::TAU / n);
    // Smooth snap toward the active face (OoT-style page turn).
    let s = (time.delta_secs() * 8.0).clamp(0.0, 1.0);
    t.rotation = t.rotation.slerp(target, s);
}

/// Ease [`CubeOpenState::amount`] toward `target` and apply the OoT page-fold to
/// every face — ported from the demo's `animate_menu_ring` / `apply_oot_open_fold`.
///
/// The fold is a *local* extra rotation composed with each face's stored base
/// placement, recomputed from scratch each frame so the base transform is never
/// corrupted. It is independent of (and composes cleanly with) the ring rotation
/// in [`animate_cube_ring`], which still drives page navigation.
///
/// This system is intentionally non-generic — it queries the plain [`CubeFace`]
/// marker rather than `AmbitionMenuPage<PageId>`, avoiding a generic-over-`PageId`
/// system (and the need to instantiate it per host type).
fn animate_cube_open(
    time: Res<Time>,
    mut state: ResMut<CubeOpenState>,
    mut faces: Query<(&CubeFace, &mut Transform)>,
) {
    // Demo's ease: `amount += (target - amount) * (1 - exp(-speed*dt))`.
    let step = 1.0 - (-CUBE_OPEN_SPEED * time.delta_secs()).exp();
    state.amount += (state.target - state.amount) * step;
    if (state.amount - state.target).abs() < 0.002 {
        state.amount = state.target;
    }
    let open = smoothstep(state.amount.clamp(0.0, 1.0));
    // Closed (open→0) folds the faces fully away; open (open→1) lays them flat.
    let fold = OOT_PAGE_FOLD_RADIANS * (1.0 - open);
    for (face, mut transform) in &mut faces {
        apply_face_fold(face, fold, &mut transform);
    }
}

/// Generalized port of the demo's `apply_oot_open_fold`: hinge each face about its
/// bottom edge and fold it outward by `fold` radians.
///
/// The demo's n=4 cardinal mapping folds the front/back faces about local X and the
/// side faces about local Z, with alternating signs. We replicate that for any ring
/// size by folding about the face's *own* local X axis (the bottom edge is the
/// hinge), choosing the sign from the index parity so adjacent pages fold to
/// opposite sides — reading like the demo's page fold while staying generic over N.
fn apply_face_fold(face: &CubeFace, fold: f32, transform: &mut Transform) {
    let sign = if face.index % 2 == 0 { 1.0 } else { -1.0 };
    let fold_rotation = Quat::from_rotation_x(sign * fold);
    let rotation = face.base_rotation * fold_rotation;
    // Keep the bottom edge of the page pinned (hinge), exactly like the demo.
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
    model: &MenuPageModel<PageId, Action>,
    active: bool,
) where
    Action: Clone + Send + Sync + 'static,
{
    spawn_panel(
        ui,
        materials,
        MenuRect::new(0.0, 0.0, 100.0, 100.0),
        menu_color(model.background),
        None::<Action>,
        active,
    );
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
            } => spawn_text(ui, materials, *x, *y, *size, text, menu_align(*align), menu_srgba(*color), active),
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
    let depth = panel_depth(rect, action.is_some());
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
    let color = control_color(kind, selected, important);
    let material = materials.add(StandardMaterial {
        base_color: color,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        unlit: true,
        ..default()
    });
    let actionable = action.is_some();
    let mut entity = ui.spawn((
        Name::new("control"),
        UiLayout::window()
            .x(Rl(rect.x))
            .y(Rl(rect.y))
            .width(Rl(rect.w))
            .height(Rh(rect.h))
            .anchor(Anchor::TOP_LEFT)
            .pack(),
        UiDepth::Set(page_depth(DEPTH_ACTION, active)),
        UiMeshPlane3d,
        MeshMaterial3d(material),
        AmbitionMenuControl {
            kind,
            action,
            focus: MenuFocusKey::default(),
        },
        MenuVisualState {
            focused: selected,
            selected,
            disabled: !actionable,
            ..Default::default()
        },
    ));
    if !actionable {
        entity.insert(Pickable::IGNORE);
    }
    entity.with_children(|children| {
        spawn_text(
            children,
            materials,
            50.0,
            44.0,
            if detail.is_some() { 12.0 } else { 14.0 },
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
