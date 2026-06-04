use std::collections::VecDeque;
use std::f32::consts::{FRAC_PI_2, PI};
use std::sync::Arc;

use bevy::asset::AssetPlugin;
use bevy::camera::{visibility::RenderLayers, ClearColorConfig};
use bevy::input::mouse::MouseWheel;
use bevy::input::touch::{TouchInput, TouchPhase};
use bevy::prelude::*;
use bevy::window::{PresentMode, PrimaryWindow};
use bevy::winit::WinitSettings;
use bevy_lunex::prelude::*;

use ambition_inventory_ui::{
    ActiveMenuPages, AmbitionMenuControl, AmbitionMenuPage, AmbitionMenuRoot,
    InventoryItemNode, InventorySlotId, ItemsOnlyPageSpec, MenuColor, MenuControlKind,
    MenuFocusKey, MenuNode, MenuOpenCloseStyle, MenuPageModel, MenuRect, MenuShellConfig,
    MenuShellEffect, MenuShellEffects, MenuShellPhase, MenuTextAlign, MenuVisualState,
};

// These are intentionally copied from crates/oot_pause_demo/src/app.rs. The mock
// demo is supposed to exercise the exact same inside-the-cube shell geometry,
// page ring, and pause/unpause fold before Ambition's real inventory data is
// connected.
const PAGE_RADIUS: f32 = 2.85;
const PAGE_W: f32 = PAGE_RADIUS * 2.0;
const PAGE_H: f32 = PAGE_W * (160.0 / 240.0);
const CAMERA_EYE: Vec3 = Vec3::new(0.0, 0.0, -2.20);
const CAMERA_LOOK: Vec3 = Vec3::new(0.0, 0.0, 0.0);
const INSIDE_PAGE_X_FLIP: f32 = -1.0;
const OOT_PAGE_FOLD_RADIANS: f32 = 1.60;
const MIN_OPEN_SCALE: f32 = 0.64;
const DEPTH_BACKGROUND: f32 = -0.05;
const DEPTH_LARGE_PANEL: f32 = -0.18;
const DEPTH_CARD: f32 = -0.34;
const DEPTH_ACTION: f32 = -0.46;
const DEPTH_EDGE: f32 = -0.82;
const DEPTH_ICON: f32 = -0.74;
const DEPTH_TEXT_TOP: f32 = -0.90;
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
const DETAIL_WRAP_COLS: usize = 64;
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
        .add_plugins(UiLunexPlugins)
        .insert_resource(ClearColor(Color::srgb(0.008, 0.009, 0.020)))
        .insert_resource(LoadFonts {
            font_directories: vec![
                "assets/fonts".to_string(),
                "/usr/share/fonts".to_string(),
                "/usr/local/share/fonts".to_string(),
            ],
            ..Default::default()
        })
        .insert_resource(MockDemo::default())
        .insert_resource(MenuAnimation::default())
        .insert_resource(MenuShell::default_open())
        .insert_resource(MenuShellEffects::default())
        .insert_resource(FpsWindow::default())
        .insert_resource(ActiveMenuPages::<MockPage, MockAction>::default())
        .insert_resource(MenuShellConfig {
            open_close_style: MenuOpenCloseStyle::OotPageFold,
            page_rotate_speed: 5.2,
            open_close_speed: 8.0,
            ..Default::default()
        })
        .add_systems(Startup, setup)
        .add_systems(Update, publish_mock_page_models)
        .add_systems(Update, menu_toggle_input)
        .add_systems(Update, (keyboard_navigation, mouse_navigation, pointer_hit_test))
        .add_systems(Update, (rebuild_lunex_faces, animate_menu_ring, update_fps_debug_overlay, sync_dummy_unpaused_overlay).chain())
        .run();
}

include!("app/state.rs");
include!("app/data.rs");
include!("app/models.rs");
include!("app/render.rs");
include!("app/systems.rs");
include!("app/input.rs");
