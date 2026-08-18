use std::collections::VecDeque;
use std::f32::consts::{FRAC_PI_2, PI};
use std::sync::Arc;

use bevy::anti_alias::fxaa::Fxaa;
use bevy::asset::AssetPlugin;
use bevy::camera::{visibility::RenderLayers, ClearColorConfig};
use bevy::core_pipeline::oit::OrderIndependentTransparencySettings;
use bevy::input::gamepad::GamepadAxis;
use bevy::input::mouse::MouseWheel;
use bevy::input::touch::{TouchInput, TouchPhase};
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Capturing, Screenshot};
use bevy::window::{PresentMode, PrimaryWindow, SystemCursorIcon};
use bevy::winit::WinitSettings;
use bevy_lunex::prelude::*;

use crate::menu::{
    MenuColor, MenuControlKind, MenuNode, MenuPageModel, MenuRect, MenuShellConfig, MenuTextAlign,
};

// Source-derived pause geometry notes:
// - z_kaleido_scope builds pause pages from a 3 * 80 by 5 * 32 page, scaled by 0.78.
// - R_PAUSE_DEPTH_OFFSET / 100.0 is 93.55, and 2 * 93.55 ~= 240 * 0.78.
// - That means adjacent pages meet at cube edges instead of floating apart.
const PAGE_RADIUS: f32 = 2.85;
const PAGE_W: f32 = PAGE_RADIUS * 2.0;
const PAGE_H: f32 = PAGE_W * (160.0 / 240.0);
const CAMERA_EYE: Vec3 = Vec3::new(0.0, 0.0, -2.20);
const CAMERA_LOOK: Vec3 = Vec3::new(0.0, 0.0, 0.0);
const INSIDE_PAGE_X_FLIP: f32 = -1.0;
const OOT_PAGE_FOLD_RADIANS: f32 = 1.60;
const DEPTH_BACKGROUND: f32 = -0.05;
const DEPTH_LARGE_PANEL: f32 = -0.18;
const DEPTH_CARD: f32 = -0.34;
const DEPTH_ACTION: f32 = -0.46;
const DEPTH_EDGE: f32 = -0.82;
const DEPTH_ICON: f32 = -0.74;
const DEPTH_TEXT_TOP: f32 = -0.90;
const DEPTH_HUD_PANEL: f32 = -1.35;
const DEPTH_HUD_ICON: f32 = -1.55;
const DEPTH_HUD_TEXT: f32 = -1.70;
const FONT_FAMILY: &str = "DejaVu Sans";
const FPS_WINDOW_SAMPLES: usize = 120;
const FPS_OVERLAY_UPDATE_SECS: f32 = 0.25;
const SAVE_FLIP_SPEED: f32 = 2.8;
const LINK_IS_ADULT: bool = true;

// HUD rectangles are authored in final visual page coordinates: x grows left-to-right
// and y grows top-to-bottom on the visible pause face. Earlier patches tried to
// compensate for the inside-face transform by hand and accidentally mirrored the
// action buttons to the left side of the screen. Keep all OoT/source-inspired HUD
// points funneled through these constants/helpers instead of repeating ad-hoc
// inversions in each call site.
const C_BUTTON_SIZE: f32 = 7.8;
const C_LEFT_RECT: MenuRect = MenuRect {
    x: 76.5,
    y: 8.0,
    w: C_BUTTON_SIZE,
    h: C_BUTTON_SIZE,
};
const C_DOWN_RECT: MenuRect = MenuRect {
    x: 84.8,
    y: 16.2,
    w: C_BUTTON_SIZE,
    h: C_BUTTON_SIZE,
};
const C_RIGHT_RECT: MenuRect = MenuRect {
    x: 93.0,
    y: 8.0,
    w: C_BUTTON_SIZE,
    h: C_BUTTON_SIZE,
};
const B_BUTTON_RECT: MenuRect = MenuRect {
    x: 59.0,
    y: 9.5,
    w: 8.2,
    h: 8.2,
};
const A_BUTTON_RECT: MenuRect = MenuRect {
    x: 68.5,
    y: 8.7,
    w: 9.2,
    h: 9.2,
};
const START_BUTTON_RECT: MenuRect = MenuRect {
    x: 45.8,
    y: 6.4,
    w: 8.5,
    h: 5.8,
};
const HUD_Z_OFFSET_TOWARD_CAMERA: f32 = 0.08;
const HUD_SCREEN_X_FLIP: f32 = -1.0;
const HUD_RENDER_LAYER: usize = 1;

// OoT draws the pause pages through POLY_OPA_DISP, while the life/magic HUD
// is drawn later through OVERLAY_DISP (see z_kaleido_scope*.c and
// z_parameter.c::Magic_DrawMeter). Mirror that separation here with a dedicated
// HUD camera/render layer so cube faces can never depth-clip the HUD.

pub(crate) fn run() {
    let readme_capture = ReadmeCapture::from_env();
    let window_resolution = readme_capture
        .as_ref()
        .map(ReadmeCapture::window_resolution)
        .unwrap_or((1180, 760));
    let present_mode = if readme_capture.is_some() {
        PresentMode::AutoVsync
    } else {
        PresentMode::AutoNoVsync
    };

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                // Bevy resolves asset paths relative to this demo crate by default
                // when running the demo. Load the demo assets directly from the repository root.
                file_path: "assets".to_string(),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Bevy Lunex OoT Kaleidoscope Menu Demo".to_string(),
                    resolution: window_resolution.into(),
                    // Interactive mode presents as fast as the host can render. Capture
                    // mode uses VSync so generated assets have deterministic warmup frames.
                    present_mode,
                    ..default()
                }),
                ..default()
            }),
    )
    .insert_resource(WinitSettings::continuous())
    .add_plugins((UiLunexPlugins, MeshPickingPlugin))
        .insert_resource(ClearColor(Color::srgb(0.012, 0.011, 0.018)))
        .insert_resource(LoadFonts {
            font_directories: vec![
                "assets/fonts".to_string(),
                "/usr/share/fonts".to_string(),
                "/usr/local/share/fonts".to_string(),
            ],
            ..Default::default()
        })
        .insert_resource(OotDemo::default())
        .insert_resource(MenuAnimation::default())
        .insert_resource(MenuShell::default_open())
        .insert_resource(FpsWindow::default())
        .insert_resource(GamepadCStickState::default())
        .insert_resource(GamepadNavStickState::default())
        .insert_resource(MenuShellConfig::default());

    if let Some(capture) = readme_capture {
        app.insert_resource(capture);
    }

    app.add_systems(Startup, setup).add_systems(
        Update,
        (
            drive_readme_demo_input,
            menu_toggle_input,
            keyboard_navigation,
            mouse_navigation,
            pointer_hit_test,
            gamepad_navigation,
            animate_equip_and_save,
            rebuild_lunex_faces,
            animate_menu_ring,
            request_readme_capture_frame,
            advance_readme_capture_frame,
            update_fps_debug_overlay,
        )
            .chain(),
    );

    app.run();
}

// Split out from the original single-file prototype. These files are
// included into this private `app` module so the first refactor is
// structural and behavior-preserving: no visibility churn or public API
// expansion is required just to make the code navigable.
include!("app/state.rs");
include!("app/capture.rs");
include!("app/data.rs");
include!("app/models.rs");
include!("app/render.rs");
include!("app/systems.rs");
include!("app/input.rs");
