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
use bevy::picking::backend::{HitData, PointerHits};
use bevy::picking::pointer::{PointerId, PointerLocation};
use bevy::picking::{Pickable, PickingSystems};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
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
// Edge page-turn buttons get their OWN band, closer than DEPTH_ACTION and away
// from the item-grid action controls, so the flanking L/R buttons never share a
// depth plane with the grid's item planes (which would z-fight / flicker as the
// ring rotates). See `is_edge_button_rect`.
const DEPTH_EDGE_BUTTON: f32 = -0.58;
const DEPTH_EDGE: f32 = -0.68;
const DEPTH_TEXT_TOP: f32 = -0.96;
const DEPTH_SELECTION: f32 = -1.12;
const FONT_FAMILY: &str = "DejaVu Sans";

/// Marks the rotating ring root that holds the cube faces.
#[derive(Component)]
pub struct MenuRing;

/// Non-generic style metadata stashed on each interactive control so a
/// non-generic system ([`sync_control_focus_visuals`]) can recolor the control's
/// material from its [`MenuVisualState`] (focus / selection / hover) without being
/// generic over the host's `Action`. This is what makes keyboard / gamepad focus
/// movement VISIBLE on the cube: the lib otherwise only colours the selected cell
/// once at build time and never re-reads the focus flag.
#[derive(Component, Clone, Copy)]
pub struct CubeControlStyle {
    kind: MenuControlKind,
    important: bool,
    disabled: bool,
}

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
    /// Multiplier applied to [`open_close_speed`] while CLOSING (`target == 0`) so
    /// the cube folds away faster than it opens (OoT subscreen feel; the open keeps
    /// the gentle ease). `1.0` = symmetric. Default `2.0`.
    pub close_speed_scale: f32,
    /// OoT opening SPIN: how many ring page-steps the cube starts rotated toward the
    /// viewer-RIGHT neighbour at the start of an OPEN, spinning around to the active
    /// page as the fold completes (synced to the eased open `amount`). `0.0` disables
    /// the spin (no opening rotation); `1.0` = one page-step. Close never spins.
    /// Default `1.0`.
    pub open_spin_faces: f32,
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
    /// Whether interactive controls are spawned as Bevy-pickable (so `Pointer<*>`
    /// events fire on them). Hosts that drive their own manual world→screen
    /// hit-test (the mock demo) set this `false` to keep controls `Pickable::IGNORE`
    /// and avoid double-handling. The game sets it `true` to use Bevy picking.
    /// Default `true`.
    pub pickable_controls: bool,
}

impl Default for CubeMenuConfig {
    fn default() -> Self {
        Self {
            geometry: MenuCubeGeometry::default(),
            fold_radians: 1.60,
            open_close_speed: 8.0,
            close_speed_scale: 2.0,
            // >1.0 starts the open spin further into the neighbour page so more of
            // the rotation is visible (1.5 = ~135° sweep on a 4-page cube).
            open_spin_faces: 1.5,
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
            pickable_controls: true,
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
        // The cube is framed by a PERSPECTIVE `Camera3d`; bevy_lunex's stock
        // `lunex_2d_picking` backend only raycasts orthographic cameras, so it never
        // generates hits for the cube. When the host wants Bevy picking on the cube
        // controls (`pickable_controls`), install a dedicated 3D picking backend that
        // raycasts the cube camera against the controls' Lunex planes and emits
        // `PointerHits` — that's what makes `Pointer<Over>`/`Pointer<Click>` fire on
        // the cube. Hosts with their own manual hit-test (the demo) leave it off.
        if app
            .world()
            .resource::<CubeMenuConfig>()
            .pickable_controls
        {
            app.add_systems(
                PreUpdate,
                cube_3d_picking.in_set(PickingSystems::Backend),
            );
            // Make ECS-driven focus / hover visible (the host moves focus in ECS
            // without rebuilding the face). The demo drives its own look + rebuilds
            // on nav, so this is gated to the Bevy-picking (game) configuration.
            app.add_systems(Update, sync_control_focus_visuals);
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

/// 3D picking backend for the cube's perspective camera.
///
/// bevy_lunex's stock `lunex_2d_picking` only raycasts ORTHOGRAPHIC cameras, so it
/// never produces hits for the cube (a perspective `Camera3d`). This backend
/// raycasts the [`CubePauseCamera`] against every hoverable Lunex `Dimension`
/// plane (the live controls) and writes `PointerHits` so the picking core can
/// dispatch `Pointer<Over>` / `Pointer<Click>` to the cube controls.
///
/// Only hoverable nodes are considered: controls that opted out of picking
/// (`Pickable::IGNORE` — disabled controls, panels, text, decoration) are skipped,
/// so the ray lands on the actual interactive controls.
fn cube_3d_picking(
    pointers: Query<(&PointerId, &PointerLocation)>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    camera_query: Query<
        (Entity, &Camera, &bevy::camera::RenderTarget, &GlobalTransform),
        With<CubePauseCamera>,
    >,
    nodes: Query<(
        Entity,
        &Dimension,
        &GlobalTransform,
        Option<&Pickable>,
        &ViewVisibility,
    )>,
    mut output: MessageWriter<PointerHits>,
) {
    // The gated cube camera is only active while the menu is open; bail otherwise.
    let Some((cam_entity, camera, render_target, cam_transform)) =
        camera_query.iter().find(|(_, c, _, _)| c.is_active)
    else {
        return;
    };
    let primary_window = primary_window.single().ok();

    // Hoverable Lunex planes only (skip IGNORE: panels / text / disabled controls).
    let candidates: Vec<_> = nodes
        .iter()
        .filter(|(_, _, transform, pickable, vis)| {
            vis.get()
                && !transform.affine().is_nan()
                && pickable.map(|p| p.is_hoverable).unwrap_or(true)
        })
        .map(|(entity, dimension, transform, pickable, _)| (entity, dimension, transform, pickable))
        .collect();

    for (pointer, location) in pointers.iter().filter_map(|(pointer, loc)| {
        loc.location().map(|l| (pointer, l))
    }) {
        // Only handle pointers on this camera's render target.
        let on_target = render_target
            .normalize(primary_window)
            .is_some_and(|t| t == location.target);
        if !on_target {
            continue;
        }

        let viewport_pos = camera
            .logical_viewport_rect()
            .map(|v| v.min)
            .unwrap_or_default();
        let pos_in_viewport = location.position - viewport_pos;
        let Ok(ray) = camera.viewport_to_world(cam_transform, pos_in_viewport) else {
            continue;
        };

        let mut picks: Vec<(Entity, HitData)> = Vec::new();
        for (entity, dimension, node_transform, _pickable) in candidates.iter().copied() {
            // Intersect the cursor ray with the node's local Z=0 plane.
            let world_to_node = node_transform.affine().inverse();
            let ray_origin_node = world_to_node.transform_point3(ray.origin);
            let ray_dir_node = world_to_node.transform_vector3(*ray.direction);
            if ray_dir_node.z.abs() < 1e-6 {
                continue; // parallel to the plane
            }
            let t = -ray_origin_node.z / ray_dir_node.z;
            if t < 0.0 {
                continue; // behind the camera
            }
            let hit_node = ray_origin_node + ray_dir_node * t;
            let rect = Rect::from_center_size(Vec2::ZERO, **dimension);
            if !rect.contains(hit_node.xy()) {
                continue;
            }
            let hit_world = node_transform.transform_point(hit_node.xy().extend(0.0));
            // Depth = distance from the camera along the ray (nearer = smaller).
            let depth = (hit_world - ray.origin).length();
            picks.push((
                entity,
                HitData::new(
                    cam_entity,
                    depth,
                    Some(hit_world),
                    Some(*node_transform.back()),
                ),
            ));
        }
        // Nearest plane first so the picking core's hover/click resolves the
        // front-most control.
        picks.sort_by(|a, b| a.1.depth.total_cmp(&b.1.depth));
        let order = camera.order as f32;
        output.write(PointerHits::new(*pointer, picks, order));
    }
}

/// Recolor each control's material from its live [`MenuVisualState`] so keyboard /
/// gamepad focus and pointer hover are VISIBLE. Without this, the lib only colours
/// the selected cell once at build time, so a host that moves focus purely in ECS
/// (the game) sees no on-screen cursor movement — the "arrow keys do nothing" bug.
///
/// Non-generic (keyed off [`CubeControlStyle`]) so it doesn't need the host's
/// `Action`. Only changed states write a new material handle (cheap, idempotent).
fn sync_control_focus_visuals(
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut controls: Query<
        (&CubeControlStyle, &MenuVisualState, &mut MeshMaterial3d<StandardMaterial>),
        Changed<MenuVisualState>,
    >,
) {
    for (style, vis, mut material) in &mut controls {
        let highlight = vis.focused || vis.selected || vis.hovered;
        let color = if style.disabled {
            disabled_control_color()
        } else {
            control_color(style.kind, highlight, style.important)
        };
        *material = MeshMaterial3d(materials.add(StandardMaterial {
            base_color: opaque_color(color),
            alpha_mode: AlphaMode::Opaque,
            cull_mode: None,
            unlit: true,
            ..default()
        }));
    }
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
    debug!(
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

    // Detect open vs close from the host's target: >0.5 = opening, else closing.
    let opening = state.target > 0.5;

    // Ease the open amount toward the host's target (demo's exp ease). The CLOSE
    // uses a faster rate (`close_speed_scale`×) so the cube folds away snappily
    // without the lingering tail, while the OPEN keeps its gentle ease.
    let rate = if opening {
        config.open_close_speed
    } else {
        config.open_close_speed * config.close_speed_scale
    };
    let open_step = 1.0 - (-rate * time.delta_secs()).exp();
    state.amount += (state.target - state.amount) * open_step;
    if (state.amount - state.target).abs() < 0.002 {
        state.amount = state.target;
    }
    let open = smoothstep(state.amount.clamp(0.0, 1.0));

    // OoT opening SPIN: while opening, start the ring rotated one page-step toward
    // the viewer-RIGHT neighbour and spin around so the active page swings to the
    // front, synced to the eased open `amount` (finishes aligned as the fold-in
    // completes). The ring formula `from_rotation_y(-idx * TAU/n)` brings the
    // viewer-LEFT neighbour (`idx+1`) to front for a positive step; the viewer-RIGHT
    // neighbour is `idx-1`, so the spin offset starts NEGATIVE and eases to 0.
    // (Sign note: if this spins the wrong way, flip the leading `-` below.)
    let spin_offset = if opening {
        -config.open_spin_faces * (1.0 - open)
    } else {
        0.0 // close never spins — it just folds away facing the active page.
    };

    // Snap the ring so the active face turns to the camera (OoT page turn).
    let active_idx = pages
        .active
        .as_ref()
        .and_then(|a| pages.pages.iter().position(|p| &p.id == a))
        .unwrap_or(0) as f32;
    let target =
        Quat::from_rotation_y(-(active_idx + spin_offset) * std::f32::consts::TAU / n);
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
    // Edge page-turn buttons (the narrow flanking L/R controls) live in their own
    // depth band so they don't z-fight with the item-grid action planes (both would
    // otherwise resolve to DEPTH_ACTION and flicker as the ring rotates).
    let control_depth = if action.is_some() && is_edge_button_rect(rect) {
        DEPTH_EDGE_BUTTON
    } else {
        panel_depth(rect, action.is_some())
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
        UiDepth::Set(page_depth(control_depth, active)),
        UiMeshPlane3d,
        MeshMaterial3d(material),
        AmbitionMenuControl {
            kind,
            action,
            focus,
        },
        CubeControlStyle {
            kind,
            important,
            disabled,
        },
        MenuVisualState {
            focused: selected,
            selected,
            disabled,
            ..Default::default()
        },
    ));
    // Disabled controls never participate in picking. Enabled controls are pickable
    // only when the host wants Bevy picking (`pickable_controls`); a host with its
    // own manual hit-test (the demo) keeps them `Pickable::IGNORE`.
    if disabled || !config.pickable_controls {
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

/// True for the narrow, vertically-centred flanking page-turn buttons (the L/R
/// "switch subscreen" controls). Matched by shape (narrow + tall + near a left or
/// right edge) so any host that places edge buttons at the conventional rect gets
/// the dedicated depth band, independent of the host's exact pixel rect.
fn is_edge_button_rect(rect: MenuRect) -> bool {
    let narrow = rect.w <= 12.0;
    let tall = rect.h >= 8.0;
    let near_edge = rect.x <= 10.0 || (rect.x + rect.w) >= 90.0;
    narrow && tall && near_edge
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
