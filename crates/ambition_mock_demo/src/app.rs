use std::collections::VecDeque;
use std::sync::Arc;

use bevy::asset::AssetPlugin;
use bevy::camera::{visibility::RenderLayers, ClearColorConfig};
use bevy::input::mouse::MouseWheel;
use bevy::input::touch::{TouchInput, TouchPhase};
use bevy::prelude::*;
use bevy::window::{PresentMode, PrimaryWindow};
use bevy::winit::WinitSettings;
use bevy_lunex::prelude::*;

use ambition_inventory_ui::cube::{
    CubeFace, CubeMenuConfig, CubeMenuPlugin, CubeOpenState, CubePauseCamera,
};
use ambition_inventory_ui::{
    ActiveMenuPages, AmbitionMenuControl, AmbitionMenuPage, InventoryItemNode, InventorySlotId,
    ItemsOnlyPageSpec, MenuColor, MenuControlKind, MenuCubeGeometry, MenuFocusKey, MenuNode,
    MenuOpenCloseStyle, MenuPageModel, MenuRect, MenuShellEffect, MenuShellEffects,
    MenuShellPhase, MenuTextAlign, MenuVisualState,
};

// Inside-the-cube shell geometry, shared with the lib via `CubeMenuConfig`.
const PAGE_RADIUS: f32 = 2.85;
const PAGE_W: f32 = PAGE_RADIUS * 2.0;
const PAGE_H: f32 = PAGE_W * (160.0 / 240.0);
const CAMERA_EYE: Vec3 = Vec3::new(0.0, 0.0, -2.20);
const CAMERA_LOOK: Vec3 = Vec3::new(0.0, 0.0, 0.0);
const INSIDE_PAGE_X_FLIP: f32 = -1.0;
const OOT_PAGE_FOLD_RADIANS: f32 = 1.60;
// HUD-overlay depth bands (app-only; the cube's own bands live in the lib).
const DEPTH_HUD_PANEL: f32 = -1.35;
const DEPTH_HUD_TEXT: f32 = -1.70;
const FONT_FAMILY: &str = "DejaVu Sans";
const FPS_WINDOW_SAMPLES: usize = 120;
const FPS_OVERLAY_UPDATE_SECS: f32 = 0.25;
const HUD_Z_OFFSET_TOWARD_CAMERA: f32 = 0.08;
const HUD_SCREEN_X_FLIP: f32 = -1.0;
const HUD_RENDER_LAYER: usize = 1;
const ITEM_GRID_COLS: usize = 6;
const ITEM_GRID_ROWS: usize = 4;
const ITEM_COUNT: usize = ITEM_GRID_COLS * ITEM_GRID_ROWS;
const DETAIL_WRAP_COLS: usize = 18;
const STATUS_WRAP_COLS: usize = 56;
const DETAIL_VISIBLE_LINES: usize = 5;

pub(crate) fn run() {
    if std::env::args().skip(1).any(|arg| arg == "--smoke" || arg == "smoke") {
        run_smoke();
        return;
    }

    App::new()
        .add_plugins(DefaultPlugins
            .set(AssetPlugin {
                file_path: "../../assets".to_string(),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Ambition Inventory UI - Ambition Mock Kaleidoscope".to_string(),
                    resolution: (1180, 760).into(),
                    present_mode: PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            }))
        .insert_resource(WinitSettings::continuous())
        .insert_resource(ClearColor(Color::srgb(0.008, 0.009, 0.020)))
        .insert_resource(LoadFonts {
            font_directories: vec![
                "assets/fonts".to_string(),
                "/usr/share/fonts".to_string(),
                "/usr/local/share/fonts".to_string(),
            ],
            ..Default::default()
        })
        // Standalone-demo cube config: the demo owns the window, so its cube
        // camera clears (dark room), starts active, and the ring starts visible.
        // The game keeps the lib defaults (overlay: no clear / gated off).
        .insert_resource(CubeMenuConfig {
            geometry: MenuCubeGeometry::oot_like(PAGE_RADIUS),
            fold_radians: OOT_PAGE_FOLD_RADIANS,
            open_close_speed: 8.0,
            // OoT feel: faster close + an opening spin (the demo is the reference).
            close_speed_scale: 2.0,
            open_spin_faces: 1.5,
            page_rotate_speed: 5.2,
            open_close_style: MenuOpenCloseStyle::OotPageFold,
            inside_x_flip: INSIDE_PAGE_X_FLIP,
            min_open_scale: 0.64,
            draw_edge_frame: true,
            draw_selection_corners: true,
            // The demo keeps its own RICHER interactive L/R edge buttons in its page
            // model (neighbor-page labels + click/keyboard page-turn via
            // `add_edge_buttons`), so it opts OUT of the lib's decorative nav arrows
            // to avoid double-drawing. The game uses the lib arrows (default `true`).
            draw_nav_arrows: false,
            camera_order: 0,
            camera_clears: true,
            camera_starts_active: true,
            ring_starts_visible: true,
            // The demo drives its OWN manual world→screen hit-test (`pointer_hit_test`),
            // so it opts OUT of Bevy-pickable controls to avoid double-handling. The
            // game sets this `true` (the default) to use Bevy picking.
            pickable_controls: false,
        })
        .insert_resource(MockDemo::default())
        .insert_resource(MenuShell::default_open())
        // The demo's HUD overlay (`rebuild_hud_overlay`) reads this; the refactor
        // dropped the insert, which panicked at runtime (resource does not exist).
        .insert_resource(MenuShellEffects::default())
        .insert_resource(FpsWindow::default())
        .insert_resource(ActiveMenuPages::<MockPage, MockAction>::default())
        // The ONE canonical cube renderer, consumed identically to the game.
        .add_plugins(CubeMenuPlugin::<MockPage, MockAction>::default())
        .add_systems(Startup, setup_app_shell)
        .add_systems(Update, publish_mock_page_models)
        .add_systems(Update, menu_toggle_input)
        .add_systems(Update, (keyboard_navigation, mouse_navigation, pointer_hit_test))
        .add_systems(Update, (drive_cube_open, rebuild_hud_overlay, update_fps_debug_overlay, sync_dummy_unpaused_overlay).chain())
        .run();
}

include!("app/state.rs");
include!("app/data.rs");
include!("app/models.rs");
include!("app/render.rs");
include!("app/systems.rs");
include!("app/input.rs");
