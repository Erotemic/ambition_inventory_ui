use std::collections::VecDeque;
use std::f32::consts::{FRAC_PI_2, PI};
use std::sync::Arc;

use bevy::anti_alias::fxaa::Fxaa;
use bevy::asset::AssetPlugin;
use bevy::core_pipeline::oit::OrderIndependentTransparencySettings;
use bevy::input::gamepad::GamepadAxis;
use bevy::input::mouse::MouseWheel;
use bevy::input::touch::{TouchInput, TouchPhase};
use bevy::prelude::*;
use bevy::camera::{ClearColorConfig, visibility::RenderLayers};
use bevy::window::{PresentMode, PrimaryWindow, SystemCursorIcon};
use bevy::winit::WinitSettings;
use bevy_lunex::prelude::*;

use ambition_inventory_ui::{
    AmbitionMenuControl, AmbitionMenuPage, AmbitionMenuRoot, MenuColor, MenuControlKind, MenuFocusKey, MenuNode,
    MenuOpenCloseStyle, MenuPageModel, MenuRect, MenuShellConfig, MenuShellEffect, MenuShellEffects, MenuShellPhase,
    MenuTextAlign, MenuVisualState,
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
const MIN_OPEN_SCALE: f32 = 0.64;
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

// HUD rectangles are authored in final visual page coordinates: x grows left-to-right
// and y grows top-to-bottom on the visible pause face. Earlier patches tried to
// compensate for the inside-face transform by hand and accidentally mirrored the
// action buttons to the left side of the screen. Keep all OoT/source-inspired HUD
// points funneled through these constants/helpers instead of repeating ad-hoc
// inversions in each call site.
const C_BUTTON_SIZE: f32 = 7.8;
const C_LEFT_RECT: MenuRect = MenuRect { x: 76.5, y: 8.0, w: C_BUTTON_SIZE, h: C_BUTTON_SIZE };
const C_DOWN_RECT: MenuRect = MenuRect { x: 84.8, y: 16.2, w: C_BUTTON_SIZE, h: C_BUTTON_SIZE };
const C_RIGHT_RECT: MenuRect = MenuRect { x: 93.0, y: 8.0, w: C_BUTTON_SIZE, h: C_BUTTON_SIZE };
const B_BUTTON_RECT: MenuRect = MenuRect { x: 59.0, y: 9.5, w: 8.2, h: 8.2 };
const A_BUTTON_RECT: MenuRect = MenuRect { x: 68.5, y: 8.7, w: 9.2, h: 9.2 };
const START_BUTTON_RECT: MenuRect = MenuRect { x: 45.8, y: 6.4, w: 8.5, h: 5.8 };
const HUD_Z_OFFSET_TOWARD_CAMERA: f32 = 0.08;
const HUD_SCREEN_X_FLIP: f32 = -1.0;
const HUD_RENDER_LAYER: usize = 1;

// OoT draws the pause pages through POLY_OPA_DISP, while the life/magic HUD
// is drawn later through OVERLAY_DISP (see z_kaleido_scope*.c and
// z_parameter.c::Magic_DrawMeter). Mirror that separation here with a dedicated
// HUD camera/render layer so cube faces can never depth-clip the HUD.


fn main() {
    App::new()
        .add_plugins(DefaultPlugins
            .set(AssetPlugin {
                // Bevy resolves asset paths relative to this demo crate by default
                // when running `cargo run -p oot_pause_demo`. Keep the canonical
                // generated icons at the workspace root and point the crate there.
                file_path: "../../assets".to_string(),
                ..default()
            })
            .set(WindowPlugin {
            primary_window: Some(Window {
                title: "Ambition Inventory UI - OoT Functional Pause Demo".to_string(),
                resolution: (1180, 760).into(),
                // Do not emulate OoT's low presentation cadence. Keep animations
                // time-based, but let the demo present as fast as the host can render.
                present_mode: PresentMode::AutoNoVsync,
                ..default()
            }),
            ..default()
        }))
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
        .insert_resource(MenuShellEffects::default())
        .insert_resource(FpsWindow::default())
        .insert_resource(GamepadCStickState::default())
        .insert_resource(GamepadNavStickState::default())
        .insert_resource(MenuShellConfig {
            open_close_style: MenuOpenCloseStyle::OotPageFold,
            page_rotate_speed: 5.2,
            open_close_speed: 8.0,
            ..Default::default()
        })
        .add_systems(Startup, setup)
        .add_systems(Update, menu_toggle_input)
        .add_systems(Update, (keyboard_navigation, mouse_navigation, pointer_hit_test, gamepad_navigation))
        .add_systems(Update, (animate_equip_and_save, animate_menu_ring, rebuild_lunex_faces, tag_hud_render_layers, update_fps_debug_overlay).chain())
        .run();
}

#[derive(Resource, Clone, Debug)]
struct OotDemo {
    page: OotPage,
    selected: OotAction,
    equipped_sword: usize,
    equipped_shield: usize,
    equipped_tunic: usize,
    equipped_boots: usize,
    c_left: usize,
    c_down: usize,
    c_right: usize,
    save_prompt_open: bool,
    save_flip: f32,
    save_flip_target: f32,
    equip_anim: Option<EquipAnim>,
    status: String,
    revision: u64,
}

impl Default for OotDemo {
    fn default() -> Self {
        Self {
            page: OotPage::Items,
            selected: OotAction::Item(0),
            equipped_sword: 1,
            equipped_shield: 1,
            equipped_tunic: 0,
            equipped_boots: 0,
            c_left: 9,
            c_down: 7,
            c_right: 3,
            save_prompt_open: false,
            save_flip: 0.0,
            save_flip_target: 0.0,
            equip_anim: None,
            status: "Complete inventory demo. Pick an item, assign it to C, or press B to save.".to_string(),
            revision: 0,
        }
    }
}

impl OotDemo {
    fn bump(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }

    fn save_modal_active(&self) -> bool {
        self.save_flip > 0.001 || self.save_flip_target > 0.001 || self.save_prompt_open
    }

    fn save_prompt_face_visible(&self) -> bool {
        self.save_flip >= 0.5 || (self.save_prompt_open && self.save_flip_target >= 1.0)
    }

    fn choose_save_yes(&mut self) {
        if self.selected != OotAction::SaveYes {
            self.selected = OotAction::SaveYes;
            self.status = "Save: YES".to_string();
            self.bump();
        }
    }

    fn choose_save_no(&mut self) {
        if self.selected != OotAction::SaveNo {
            self.selected = OotAction::SaveNo;
            self.status = "Save: NO".to_string();
            self.bump();
        }
    }

    fn pages() -> [OotPage; 4] {
        [OotPage::Items, OotPage::Map, OotPage::Quest, OotPage::Equipment]
    }

    fn default_action_for_page(page: OotPage) -> OotAction {
        match page {
            OotPage::Items => OotAction::Item(0),
            OotPage::Equipment => OotAction::EquipChoice { slot: 0, choice: 1 },
            OotPage::Map => OotAction::MapMarker(0),
            OotPage::Quest => OotAction::QuestIcon(0),
        }
    }

    fn goto_page(&mut self, page: OotPage) {
        if self.page != page {
            self.page = page;
            self.selected = Self::default_action_for_page(page);
            self.status = format!("{} page", page.label());
            self.bump();
        }
    }

    fn page_on_viewer_left(page: OotPage) -> OotPage {
        // Observed inside-the-cube convention: the page physically on the left
        // is the next index in the source page ring.
        OotPage::from_index(page.index() + 1)
    }

    fn page_on_viewer_right(page: OotPage) -> OotPage {
        // Observed inside-the-cube convention: the page physically on the right
        // is the previous index in the source page ring.
        OotPage::from_index(page.index() - 1)
    }

    fn turn_page(&mut self, direction: PageTurn) {
        let target = match direction {
            PageTurn::ViewerLeft => Self::page_on_viewer_left(self.page),
            PageTurn::ViewerRight => Self::page_on_viewer_right(self.page),
        };
        self.goto_page(target);
    }

    fn next_page(&mut self) {
        self.turn_page(PageTurn::ViewerRight);
    }

    fn previous_page(&mut self) {
        self.turn_page(PageTurn::ViewerLeft);
    }

    fn hover(&mut self, action: OotAction) {
        if self.selected != action {
            self.selected = action;
            self.status = action.describe_hover(self);
            self.bump();
        }
    }

    fn click(&mut self, action: OotAction) {
        let previous_selected = self.selected;
        self.selected = action;
        match action {
            // OoT-style edge prompts: left/right are physical directions from the player's view.
            OotAction::EdgeLeft => self.turn_page(PageTurn::ViewerLeft),
            OotAction::EdgeRight => self.turn_page(PageTurn::ViewerRight),
            OotAction::Item(idx) => {
                let item = oot_items()[idx];
                self.status = format!("{} selected. Press Z/X/C to assign.", item.name);
                self.bump();
            }
            OotAction::AssignC(button) => {
                if let OotAction::Item(idx) = previous_selected {
                    self.start_c_button_equip(idx, button);
                } else {
                    self.status = "Select an item first, then assign it to a C-button.".to_string();
                    self.bump();
                }
            }
            OotAction::Save => {
                self.toggle_save_prompt();
            }
            OotAction::SaveYes => {
                self.status = "Game saved. Return to play or continue editing C-button assignments.".to_string();
                self.toggle_save_prompt();
            }
            OotAction::SaveNo => {
                self.status = "Save cancelled.".to_string();
                self.toggle_save_prompt();
            }
            OotAction::EquipChoice { slot, choice } => {
                match slot {
                    0 => self.equipped_sword = choice,
                    1 => self.equipped_shield = choice,
                    2 => self.equipped_tunic = choice,
                    _ => self.equipped_boots = choice,
                }
                let option = equip_slots()[slot].choices[choice];
                self.status = format!("Equipped {}.", option.name);
                self.bump();
            }
            OotAction::MapMarker(idx) => {
                let marker = map_markers()[idx];
                self.status = format!("Map marker: {}.", marker.name);
                self.bump();
            }
            OotAction::QuestIcon(idx) => {
                let q = all_quest_icons()[idx];
                self.status = format!("{} achieved.", q.name);
                self.bump();
            }
            OotAction::Song(idx) => {
                let song = songs()[idx];
                self.status = format!("{} reminder: {}", song.name, song.pattern);
                self.bump();
            }
        }
    }


    fn toggle_save_prompt(&mut self) {
        self.save_prompt_open = !self.save_prompt_open;
        self.save_flip_target = if self.save_prompt_open { 1.0 } else { 0.0 };
        self.status = if self.save_prompt_open {
            self.selected = OotAction::SaveYes;
            "Save? Choose Yes or No. The active page flips around its horizontal center line.".to_string()
        } else {
            if matches!(self.selected, OotAction::SaveYes | OotAction::SaveNo | OotAction::Save) {
                self.selected = Self::default_action_for_page(self.page);
            }
            "Returned to item selection.".to_string()
        };
        self.bump();
    }

    fn start_c_button_equip(&mut self, item_idx: usize, button: CButton) {
        let item = oot_items()[item_idx];
        let button_idx = button.index();
        let start = item_grid_center(item_idx);
        let bow_idx = bow_item_index();
        let is_arrow = arrow_kind(item_idx).is_some();
        self.equip_anim = Some(EquipAnim {
            item_idx,
            target_button: button,
            phase: if is_arrow { EquipAnimPhase::ArrowGlowToBow } else { EquipAnimPhase::ItemToButton },
            progress: 0.0,
            from: start,
            via: item_grid_center(bow_idx),
            to: c_button_center(button),
        });
        self.status = if let Some(kind) = arrow_kind(item_idx) {
            format!("{} magic is modifying the Fairy Bow for C-{}.", kind.label(), button.label())
        } else {
            format!("Equipping {} to C-{}.", item.name, button.label())
        };
        // Functional OoT behavior happens at animation completion, but keep the
        // target unique immediately so the button preview never duplicates slots.
        self.preview_unique_c_button(item_idx, button_idx);
        self.bump();
    }

    fn preview_unique_c_button(&mut self, item_idx: usize, button_idx: usize) {
        let mut values = [self.c_left, self.c_down, self.c_right];
        let target_family = c_slot_family(item_idx);
        for i in 0..values.len() {
            if i != button_idx && c_slot_family(values[i]) == target_family {
                values.swap(i, button_idx);
                break;
            }
        }
        values[button_idx] = item_idx;
        self.c_left = values[0];
        self.c_down = values[1];
        self.c_right = values[2];
    }

    fn finish_c_button_equip(&mut self, item_idx: usize, button: CButton) {
        self.preview_unique_c_button(item_idx, button.index());
        self.status = format!("Assigned {} to C-{}.", oot_items()[item_idx].name, button.label());
        self.equip_anim = None;
        self.bump();
    }

    fn assign_selected_item_to_c_button(&mut self, button: CButton) {
        if let OotAction::Item(idx) = self.selected {
            // C-buttons are status indicators in the pause HUD, not focusable
            // controls. Keep the cursor on the inventory item while the equip
            // animation runs toward the requested C slot.
            self.start_c_button_equip(idx, button);
        } else {
            self.status = "Move the cursor to an inventory item before assigning it to a C-button.".to_string();
            self.bump();
        }
    }

    fn press_b_button(&mut self) {
        // The visual B button is also an indicator. Keyboard/gamepad B opens
        // the save prompt without moving focus to the B button.
        self.toggle_save_prompt();
    }

    fn activate_selected(&mut self) {
        self.click(self.selected);
    }

    fn move_spatial(&mut self, dx: i32, dy: i32) {
        let targets = active_page_hit_targets(self);
        let current = self.selected;
        let Some(current_target) = targets.iter().find(|t| t.action == current) else {
            if let Some(first) = targets.first() {
                self.hover(first.action);
            }
            return;
        };
        let current_center = current_target.rect.center();
        let mut best: Option<(f32, OotAction)> = None;
        for target in targets {
            if target.action == current {
                continue;
            }
            let center = target.rect.center();
            let delta = center - current_center;
            let forward = if dx < 0 {
                -delta.x
            } else if dx > 0 {
                delta.x
            } else if dy < 0 {
                -delta.y
            } else {
                delta.y
            };
            if forward <= 0.25 {
                continue;
            }
            let perp = if dx != 0 { delta.y.abs() } else { delta.x.abs() };
            let score = forward + perp * 0.42;
            if best.map(|(best_score, _)| score < best_score).unwrap_or(true) {
                best = Some((score, target.action));
            }
        }
        if let Some((_, action)) = best {
            self.hover(action);
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
enum OotPage {
    Items,
    Map,
    Quest,
    Equipment,
}

impl OotPage {
    fn index(self) -> i32 {
        match self {
            OotPage::Items => 0,
            OotPage::Map => 1,
            OotPage::Quest => 2,
            OotPage::Equipment => 3,
        }
    }

    fn from_index(idx: i32) -> Self {
        match idx.rem_euclid(4) {
            0 => OotPage::Items,
            1 => OotPage::Map,
            2 => OotPage::Quest,
            _ => OotPage::Equipment,
        }
    }

    fn label(self) -> &'static str {
        match self {
            OotPage::Items => "Select Item",
            OotPage::Equipment => "Equipment",
            OotPage::Map => "Map",
            OotPage::Quest => "Quest Status",
        }
    }

    fn face_color(self) -> Color {
        match self {
            OotPage::Items => Color::srgb(0.040, 0.105, 0.155),
            OotPage::Equipment => Color::srgb(0.095, 0.075, 0.035),
            OotPage::Map => Color::srgb(0.040, 0.090, 0.060),
            OotPage::Quest => Color::srgb(0.090, 0.070, 0.100),
        }
    }

}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
enum CButton {
    Left,
    Down,
    Right,
}

impl CButton {
    fn label(self) -> &'static str {
        match self {
            CButton::Left => "Left",
            CButton::Down => "Down",
            CButton::Right => "Right",
        }
    }

    fn index(self) -> usize {
        match self {
            CButton::Left => 0,
            CButton::Down => 1,
            CButton::Right => 2,
        }
    }
}

/// Physical page-turn direction from the player's viewpoint inside the cube.
///
/// Do not replace these calls with raw `index() +/- 1` elsewhere. The page ring
/// is stored in OoT source order, while the inside-facing Lunex room is mirrored
/// relative to screen-space page motion. Keeping the convention here prevents
/// LB/RB, edge buttons, keyboard, and mouse wheel from drifting out of sync.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
enum PageTurn {
    ViewerLeft,
    ViewerRight,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
enum OotAction {
    EdgeLeft,
    EdgeRight,
    AssignC(CButton),
    Save,
    SaveYes,
    SaveNo,
    Item(usize),
    EquipChoice { slot: usize, choice: usize },
    MapMarker(usize),
    QuestIcon(usize),
    Song(usize),
}

impl OotAction {
    fn describe_hover(self, demo: &OotDemo) -> String {
        match self {
            OotAction::EdgeLeft => format!("Rotate left to {}.", OotDemo::page_on_viewer_left(demo.page).label()),
            OotAction::EdgeRight => format!("Rotate right to {}.", OotDemo::page_on_viewer_right(demo.page).label()),
            OotAction::AssignC(button) => format!("Assign selected item to C-{}.", button.label()),
            OotAction::Save => "Open the save confirmation.".to_string(),
            OotAction::SaveYes => "Save and close the confirmation.".to_string(),
            OotAction::SaveNo => "Cancel saving.".to_string(),
            OotAction::Item(idx) => oot_items()[idx].name.to_string(),
            OotAction::EquipChoice { slot, choice } => format!("{}: {}", equip_slots()[slot].name, equip_slots()[slot].choices[choice].name),
            OotAction::MapMarker(idx) => map_markers()[idx].name.to_string(),
            OotAction::QuestIcon(idx) => all_quest_icons()[idx].name.to_string(),
            OotAction::Song(idx) => songs()[idx].name.to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct EquipAnim {
    item_idx: usize,
    target_button: CButton,
    phase: EquipAnimPhase,
    progress: f32,
    from: Vec2,
    via: Vec2,
    to: Vec2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EquipAnimPhase {
    ItemToButton,
    ArrowGlowToBow,
    ArrowBowHold,
    BowToButton,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ArrowKind { Fire, Ice, Light }

impl ArrowKind {
    fn label(self) -> &'static str {
        match self {
            ArrowKind::Fire => "Fire Arrow",
            ArrowKind::Ice => "Ice Arrow",
            ArrowKind::Light => "Light Arrow",
        }
    }

    fn glow_icon(self) -> &'static str {
        match self {
            ArrowKind::Fire => "icons/oot/fire_arrow.png",
            ArrowKind::Ice => "icons/oot/ice_arrow.png",
            ArrowKind::Light => "icons/oot/light_arrow.png",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CSlotFamily { Bow, Item(usize) }


#[derive(Resource, Clone, Debug)]
struct MenuAnimation {
    current_angle: f32,
    target_angle: f32,
}

impl Default for MenuAnimation {
    fn default() -> Self {
        Self { current_angle: 0.0, target_angle: 0.0 }
    }
}

impl MenuAnimation {
    fn set_page(&mut self, page: OotPage) {
        self.target_angle = -page.index() as f32 * FRAC_PI_2;
    }
}

#[derive(Resource, Clone, Debug)]
struct MenuShell {
    openness: f32,
    target_open: bool,
}

impl MenuShell {
    fn default_open() -> Self {
        Self { openness: 1.0, target_open: true }
    }

    fn toggle(&mut self) {
        self.target_open = !self.target_open;
    }

    fn is_visible(&self) -> bool {
        self.target_open || self.openness > 0.01
    }

    fn is_interactive(&self) -> bool {
        self.target_open && self.openness > 0.985
    }

    fn phase(&self) -> MenuShellPhase {
        if self.target_open {
            if self.openness >= 0.985 { MenuShellPhase::Open } else { MenuShellPhase::Opening }
        } else if self.openness <= 0.015 {
            MenuShellPhase::Closed
        } else {
            MenuShellPhase::Closing
        }
    }
}

#[derive(Component)]
struct MenuRing;
#[derive(Component)]
struct LunexFaceRoot;
#[derive(Component)]
struct PageFace(OotPage);
#[derive(Component)]
struct FpsDebugText;
#[derive(Component)]
struct HudOverlayRoot;
#[derive(Component)]
struct MainPauseCamera;

#[derive(Resource, Debug)]
struct FpsWindow {
    samples: VecDeque<f32>,
}

impl Default for FpsWindow {
    fn default() -> Self { Self { samples: VecDeque::with_capacity(FPS_WINDOW_SAMPLES) } }
}

#[derive(Resource, Default, Debug)]
struct GamepadCStickState {
    active: Option<CButton>,
}

#[derive(Resource, Default, Debug)]
struct GamepadNavStickState {
    active: Option<(i32, i32)>,
}


fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    demo: Res<OotDemo>,
) {
    commands.spawn((
        DirectionalLight { illuminance: 2800.0, shadows_enabled: false, ..default() },
        Transform::from_xyz(1.5, 3.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Name::new("OoT pause cube camera"),
        MainPauseCamera,
        Camera3d::default(),
        Camera { order: 0, ..default() },
        RenderLayers::layer(0),
        OrderIndependentTransparencySettings::default(),
        Msaa::Off,
        Fxaa::default(),
        Transform::from_translation(CAMERA_EYE).looking_at(CAMERA_LOOK, Vec3::Y),
    ));
    commands.spawn((
        Name::new("OoT pause HUD overlay camera"),
        Camera3d::default(),
        Camera {
            order: 10,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        Msaa::Off,
        Fxaa::default(),
        RenderLayers::layer(HUD_RENDER_LAYER),
        Transform::from_translation(CAMERA_EYE).looking_at(CAMERA_LOOK, Vec3::Y),
    ));
    commands.spawn((
        FpsDebugText,
        Text::new("fps: collecting..."),
        TextFont { font_size: 14.0, ..default() },
        TextColor(Color::srgba(0.86, 0.95, 0.88, 0.92)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            top: Val::Px(8.0),
            ..default()
        },
    ));
    let ring = commands
        .spawn((
            Name::new("OoT-style Lunex pause room"),
            AmbitionMenuRoot,
            MenuRing,
            UiRoot3d,
            Transform::default(),
            Visibility::Visible,
            RenderLayers::layer(0),
        ))
        .id();
    commands.entity(ring).with_children(|ring| {
        spawn_all_faces(ring, &demo, &mut materials, &asset_server);
    });
    spawn_hud_overlay(&mut commands, &demo, &mut materials, &asset_server);
}

fn update_fps_debug_overlay(
    time: Res<Time>,
    mut fps: ResMut<FpsWindow>,
    mut text_query: Query<&mut Text, With<FpsDebugText>>,
) {
    let delta = time.delta_secs();
    if delta <= 0.0 {
        return;
    }
    if fps.samples.len() == FPS_WINDOW_SAMPLES {
        fps.samples.pop_front();
    }
    fps.samples.push_back(1.0 / delta);

    let mut min = f32::INFINITY;
    let mut max = 0.0_f32;
    let mut sum = 0.0_f32;
    for sample in fps.samples.iter().copied() {
        min = min.min(sample);
        max = max.max(sample);
        sum += sample;
    }
    let mean = sum / fps.samples.len().max(1) as f32;

    for mut text in &mut text_query {
        *text = Text::new(format!("FPS {mean:5.1}  min {min:5.1}  max {max:5.1}"));
    }
}

fn rebuild_lunex_faces(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    demo: Res<OotDemo>,
    ring_query: Query<Entity, With<MenuRing>>,
    face_query: Query<(Entity, &PageFace), With<LunexFaceRoot>>,
    hud_query: Query<Entity, With<HudOverlayRoot>>,
    mut last_revision: Local<Option<u64>>,
    mut last_page: Local<Option<OotPage>>,
) {
    if *last_revision == Some(demo.revision) {
        return;
    }
    let Ok(ring) = ring_query.single() else { return; };
    let page_changed = last_page.map(|p| p != demo.page).unwrap_or(true);
    if page_changed {
        for (entity, _) in &face_query {
            commands.entity(entity).despawn();
        }
        commands.entity(ring).with_children(|ring| spawn_all_faces(ring, &demo, &mut materials, &asset_server));
    } else {
        for (entity, face) in &face_query {
            if face.0 == demo.page {
                commands.entity(entity).despawn();
            }
        }
        commands.entity(ring).with_children(|ring| spawn_face(ring, demo.page, &demo, &mut materials, &asset_server));
    }
    for entity in &hud_query {
        commands.entity(entity).despawn();
    }
    spawn_hud_overlay(&mut commands, &demo, &mut materials, &asset_server);
    *last_revision = Some(demo.revision);
    *last_page = Some(demo.page);
}

fn spawn_hud_overlay(
    commands: &mut Commands,
    demo: &OotDemo,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
) {
    let model = build_pause_hud_model(demo);
    commands.spawn((
        Name::new("OoT pause HUD overlay"),
        HudOverlayRoot,
        UiRoot3d,
        UiLayoutRoot::new_3d(),
        Dimension::from((PAGE_W, PAGE_H)),
        // The HUD is not a child of MenuRing, so it does not rotate with the
        // cube or with the save-prompt flip. It sits just in front of the active
        // face. Because the pause camera is viewing the inside/back side of the
        // page plane, raw local +X projects as visual-left; keep HUD models
        // authored in normal screen coordinates and flip the overlay root once.
        Transform::from_translation(Vec3::new(0.0, 0.0, PAGE_RADIUS - HUD_Z_OFFSET_TOWARD_CAMERA))
            .with_scale(Vec3::new(HUD_SCREEN_X_FLIP, 1.0, 1.0)),
        Visibility::Visible,
        RenderLayers::layer(HUD_RENDER_LAYER),
    )).with_children(|ui| render_overlay_model(ui, materials, asset_server, &model));
}


fn tag_hud_render_layers(
    mut commands: Commands,
    hud_roots: Query<Entity, With<HudOverlayRoot>>,
    children_query: Query<&Children>,
    unlayered: Query<Entity, Without<RenderLayers>>,
) {
    for root in &hud_roots {
        tag_hud_entity_recursive(root, &mut commands, &children_query, &unlayered);
    }
}

fn tag_hud_entity_recursive(
    entity: Entity,
    commands: &mut Commands,
    children_query: &Query<&Children>,
    unlayered: &Query<Entity, Without<RenderLayers>>,
) {
    if unlayered.get(entity).is_ok() {
        commands.entity(entity).insert(RenderLayers::layer(HUD_RENDER_LAYER));
    }
    if let Ok(children) = children_query.get(entity) {
        for child in children.iter() {
            tag_hud_entity_recursive(child, commands, children_query, unlayered);
        }
    }
}

fn spawn_all_faces(
    ring: &mut ChildSpawnerCommands,
    demo: &OotDemo,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
) {
    for page in OotDemo::pages() {
        spawn_face(ring, page, demo, materials, asset_server);
    }
}

fn spawn_face(
    ring: &mut ChildSpawnerCommands,
    page: OotPage,
    demo: &OotDemo,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
) {
    let (translation, rotation) = page_face_transform(page);
    let mut face = ring.spawn((
        Name::new(format!("{} OoT face", page.label())),
        LunexFaceRoot,
        PageFace(page),
        AmbitionMenuPage { id: page, active: page == demo.page },
        UiRoot3d,
        UiLayoutRoot::new_3d(),
        Dimension::from((PAGE_W, PAGE_H)),
        Transform::from_translation(translation)
            .with_rotation(rotation)
            .with_scale(Vec3::new(INSIDE_PAGE_X_FLIP, 1.0, 1.0)),
    ));
    face.with_children(|ui| {
        let active_face = page == demo.page;
        let model = build_page_model(page, demo, active_face);
        render_page_model(ui, materials, asset_server, &model);
    });
}

fn page_face_transform(page: OotPage) -> (Vec3, Quat) {
    match page {
        OotPage::Items => (Vec3::new(0.0, 0.0, PAGE_RADIUS), Quat::IDENTITY),
        OotPage::Map => (Vec3::new(PAGE_RADIUS, 0.0, 0.0), Quat::from_rotation_y(FRAC_PI_2)),
        OotPage::Quest => (Vec3::new(0.0, 0.0, -PAGE_RADIUS), Quat::from_rotation_y(PI)),
        OotPage::Equipment => (Vec3::new(-PAGE_RADIUS, 0.0, 0.0), Quat::from_rotation_y(-FRAC_PI_2)),
    }
}

fn reset_face_transform(page: OotPage, transform: &mut Transform) {
    let (translation, rotation) = page_face_transform(page);
    transform.translation = translation;
    transform.rotation = rotation;
    transform.scale = Vec3::new(INSIDE_PAGE_X_FLIP, 1.0, 1.0);
}

fn apply_oot_open_fold(page: OotPage, fold: f32, transform: &mut Transform) {
    let (base_translation, base_rotation) = page_face_transform(page);
    // Matches the source transform idea: pages are fixed around the origin,
    // fold around their lower edge, and side pages use Z-pitch before their Y-facing rotation.
    let fold_rotation = match page {
        OotPage::Items => Quat::from_rotation_x(fold),
        OotPage::Quest => Quat::from_rotation_x(-fold),
        OotPage::Map => Quat::from_rotation_z(-fold),
        OotPage::Equipment => Quat::from_rotation_z(fold),
    };
    let rotation = fold_rotation * base_rotation;
    let hinge_local = Vec3::new(0.0, -PAGE_H * 0.5, 0.0);
    let hinge_world = base_translation + base_rotation * hinge_local;
    let translation = hinge_world - rotation * hinge_local;
    transform.translation = translation;
    transform.rotation = rotation;
    transform.scale = Vec3::new(INSIDE_PAGE_X_FLIP, 1.0, 1.0);
}

fn build_page_model(page: OotPage, demo: &OotDemo, active_face: bool) -> MenuPageModel<OotPage, OotAction> {
    let prompt_face = active_face && demo.save_prompt_face_visible();
    let background = if prompt_face { Color::srgba(0.010, 0.011, 0.026, 1.0) } else { page.face_color() };
    let mut model = MenuPageModel::new(page, page.label(), mc(background));

    // OoT does not draw the normal pause pane underneath the save page: the
    // active page is pitched away, then the prompt page is drawn with the same
    // transform. Keep that same single-surface invariant here. Rendering both
    // was the cause of the visible normal menu plus flickering Yes/No options.
    if prompt_face {
        add_save_prompt_panel(&mut model, demo);
        return model;
    }

    let page_actions_enabled = active_face && !demo.save_modal_active();
    add_edge_buttons(&mut model, page, page_actions_enabled);
    match page {
        OotPage::Items => add_items_page(&mut model, demo, page_actions_enabled),
        OotPage::Equipment => add_equipment_page(&mut model, demo, page_actions_enabled),
        OotPage::Map => add_map_page(&mut model, demo, page_actions_enabled),
        OotPage::Quest => add_quest_page(&mut model, demo, page_actions_enabled),
    }
    if !demo.save_modal_active() {
        add_status_band(&mut model, demo);
    }
    model
}

fn build_pause_hud_model(demo: &OotDemo) -> MenuPageModel<OotPage, OotAction> {
    let mut model = MenuPageModel::new(demo.page, "Pause HUD", mc(Color::NONE));
    add_pause_hud_overlay(&mut model, demo, true);
    model
}

fn add_edge_buttons(model: &mut MenuPageModel<OotPage, OotAction>, _page: OotPage, active_face: bool) {
    model.control_with_icon(
        MenuRect::new(1.2, 38.0, 10.0, 24.0),
        MenuControlKind::Tab,
        "",
        Some("L".to_string()),
        Some("icons/oot/edge_left.png"),
        false,
        true,
        active_face.then_some(OotAction::EdgeLeft),
    );
    model.control_with_icon(
        MenuRect::new(88.8, 38.0, 10.0, 24.0),
        MenuControlKind::Tab,
        "",
        Some("R".to_string()),
        Some("icons/oot/edge_right.png"),
        false,
        true,
        active_face.then_some(OotAction::EdgeRight),
    );
}

fn add_items_page(model: &mut MenuPageModel<OotPage, OotAction>, demo: &OotDemo, active_face: bool) {
    model.panel(MenuRect::new(14.0, 20.0, 72.0, 54.0), mc(Color::srgba(0.02, 0.03, 0.055, 0.94)), None);
    let cols = 6;
    let cell_w = 10.0;
    let cell_h = 11.5;
    let gap_x = 1.4;
    let gap_y = 1.5;
    let x0 = 17.0;
    let y0 = 24.0;
    for (i, item) in oot_items().iter().enumerate() {
        let col = i % cols;
        let row = i / cols;
        let x = x0 + col as f32 * (cell_w + gap_x);
        let y = y0 + row as f32 * (cell_h + gap_y);
        model.control_with_icon(
            MenuRect::new(x, y, cell_w, cell_h),
            MenuControlKind::Item,
            "",
            item.detail.map(|s| s.to_string()),
            Some(item.icon),
            demo.selected == OotAction::Item(i),
            item.important,
            active_face.then_some(OotAction::Item(i)),
        );
    }
}

fn add_save_prompt_panel(model: &mut MenuPageModel<OotPage, OotAction>, demo: &OotDemo) {
    // Prompt contents are the only contents on the active face after the flip
    // midpoint. Keep this opaque and sparse to avoid z-fighting with the normal
    // inventory/equipment/map/quest controls.
    model.panel(MenuRect::new(18.0, 24.0, 64.0, 46.0), mc(Color::srgba(0.006, 0.008, 0.025, 1.0)), None);
    model.panel(MenuRect::new(24.0, 31.0, 52.0, 29.0), mc(Color::srgba(0.022, 0.026, 0.060, 1.0)), None);
    model.text(50.0, 38.5, 3.2, "Would you like to save?", MenuTextAlign::Center, mc(Color::srgb(0.94, 0.86, 0.55)));
    model.control_with_icon(MenuRect::new(34.0, 47.0, 13.5, 7.8), MenuControlKind::Action, "YES", None, None::<String>, demo.selected == OotAction::SaveYes, true, Some(OotAction::SaveYes));
    model.control_with_icon(MenuRect::new(52.5, 47.0, 13.5, 7.8), MenuControlKind::Action, "NO", None, None::<String>, demo.selected == OotAction::SaveNo, true, Some(OotAction::SaveNo));
}

fn add_pause_hud_overlay(model: &mut MenuPageModel<OotPage, OotAction>, demo: &OotDemo, _active_face: bool) {
    // HUD elements are indicators layered over every pause face. They are not
    // focusable menu cells, and the explicit project instruction says the C/A/B
    // area must not become keyboard/gamepad cursor targets.
    add_health_and_magic(model);
    add_start_button_indicator(model);
    add_action_button_indicators(model, demo);
    add_c_button_assignments(model, demo);

    if let Some(anim) = demo.equip_anim {
        add_equip_anim_visual(model, anim);
    }
    model.text(50.0, 75.0, 2.2, "Select an item, then press Z / X / C to assign. Press B for save.", MenuTextAlign::Center, mc(Color::srgb(0.75, 0.83, 0.90)));
}

fn add_health_and_magic(model: &mut MenuPageModel<OotPage, OotAction>) {
    for i in 0..10 {
        let x = 6.0 + (i % 10) as f32 * 3.2;
        model.control_with_icon(
            MenuRect::new(x, 6.0, 2.8, 2.8),
            MenuControlKind::Decoration,
            "",
            None,
            Some("icons/oot/heart_piece.png"),
            false,
            false,
            None,
        );
    }
    // Keep the magic meter in the HUD overlay, not on a rotating pane. The fill
    // is rendered with explicit HUD depths below so it cannot z-fight with the
    // backing or be clipped by cube pages while the pause shell spins.
    model.panel(MenuRect::new(6.0, 11.0, 27.0, 2.8), mc(Color::srgb(0.018, 0.045, 0.020)), None);
    model.panel(MenuRect::new(6.7, 11.72, 20.9, 1.35), mc(Color::srgb(0.08, 0.72, 0.24)), None);
}

fn add_start_button_indicator(model: &mut MenuPageModel<OotPage, OotAction>) {
    model.control_with_icon(
        START_BUTTON_RECT,
        MenuControlKind::Decoration,
        "",
        Some("START".to_string()),
        Some("icons/oot/hud_start.png"),
        false,
        true,
        None,
    );
}

fn add_action_button_indicators(model: &mut MenuPageModel<OotPage, OotAction>, demo: &OotDemo) {
    let in_prompt = demo.save_modal_active();
    model.control_with_icon(
        B_BUTTON_RECT,
        MenuControlKind::Action,
        "",
        Some(if in_prompt { "Back".to_string() } else { "Save".to_string() }),
        Some("icons/oot/hud_button_b.png"),
        false,
        true,
        Some(if in_prompt { OotAction::SaveNo } else { OotAction::Save }),
    );
    model.control_with_icon(
        A_BUTTON_RECT,
        MenuControlKind::Action,
        "",
        Some(if in_prompt { "Decide".to_string() } else { "Decide".to_string() }),
        Some("icons/oot/hud_button_a.png"),
        false,
        true,
        if in_prompt { Some(demo.selected) } else { None },
    );
}

fn add_c_button_assignments(model: &mut MenuPageModel<OotPage, OotAction>, demo: &OotDemo) {
    // C-up is intentionally omitted: it is not an assignable inventory target in
    // this demo. Keep only the three yellow C targets, anchored in screen/HUD
    // space rather than baked into any rotating page face.
    let assignments = [
        ("", demo.c_left, C_LEFT_RECT, CButton::Left),
        ("", demo.c_down, C_DOWN_RECT, CButton::Down),
        ("", demo.c_right, C_RIGHT_RECT, CButton::Right),
    ];
    for (_label, idx, rect, button) in assignments {
        let item = oot_items()[idx];
        model.control_with_icon(
            rect,
            MenuControlKind::Action,
            "",
            Some(item.name.to_string()),
            Some("icons/oot/hud_button_c.png"),
            false,
            true,
            (!demo.save_modal_active()).then_some(OotAction::AssignC(button)),
        );
        let inset = rect.w * 0.24;
        model.control_with_icon(
            MenuRect::new(rect.x + inset, rect.y + inset, rect.w - inset * 2.0, rect.h - inset * 2.0),
            MenuControlKind::Decoration,
            "",
            None,
            Some(item.icon),
            false,
            false,
            None,
        );
    }
}

fn add_equipment_page(model: &mut MenuPageModel<OotPage, OotAction>, demo: &OotDemo, active_face: bool) {
    model.panel(MenuRect::new(14.0, 20.0, 72.0, 58.0), mc(Color::srgba(0.055, 0.042, 0.025, 1.0)), None);

    // Closer to OoT's equipment page: an upgrades column at far left, a player preview
    // in the left-center, and the 3-choice equipment grid on the right.
    model.panel(MenuRect::new(29.0, 25.0, 16.0, 43.0), mc(Color::srgba(0.045, 0.100, 0.065, 1.0)), None);
    model.control_with_icon(
        MenuRect::new(30.7, 27.0, 12.6, 37.5),
        MenuControlKind::Decoration,
        "LINK",
        Some("preview".to_string()),
        Some("icons/oot/player.png"),
        false,
        false,
        None,
    );

    let upgrade_icons = [
        ("Quiver", "icons/oot/bow.png"),
        ("Bomb", "icons/oot/bomb.png"),
        ("Power", "icons/oot/stone_ruby.png"),
        ("Scale", "icons/oot/stone_sapphire.png"),
    ];
    for (row, (label, icon)) in upgrade_icons.iter().enumerate() {
        let y = 26.0 + row as f32 * 12.0;
        model.control_with_icon(
            MenuRect::new(17.5, y, 8.4, 8.4),
            MenuControlKind::Decoration,
            *label,
            None,
            Some(*icon),
            false,
            false,
            None,
        );
    }

    let row_y = [26.0, 38.0, 50.0, 62.0];
    let col_x = [50.0, 62.0, 74.0];
    for (slot_idx, slot) in equip_slots().iter().enumerate() {
        model.text(47.0, row_y[slot_idx] + 4.3, 2.15, slot.name, MenuTextAlign::Right, mc(Color::srgb(0.92, 0.80, 0.50)));
        for (choice_idx, choice) in slot.choices.iter().enumerate() {
            let equipped = match slot_idx {
                0 => demo.equipped_sword == choice_idx,
                1 => demo.equipped_shield == choice_idx,
                2 => demo.equipped_tunic == choice_idx,
                _ => demo.equipped_boots == choice_idx,
            };
            let action = OotAction::EquipChoice { slot: slot_idx, choice: choice_idx };
            model.control_with_icon(
                MenuRect::new(col_x[choice_idx], row_y[slot_idx], 9.5, 9.5),
                MenuControlKind::Item,
                "",
                equipped.then(|| "E".to_string()),
                Some(choice.icon),
                demo.selected == action || equipped,
                equipped,
                active_face.then_some(action),
            );
        }
    }
    model.text(50.0, 78.7, 2.5, "Equipment grid: upgrades / player preview / 3 choices per slot", MenuTextAlign::Center, mc(Color::srgb(0.82, 0.72, 0.48)));
}

fn add_map_page(model: &mut MenuPageModel<OotPage, OotAction>, demo: &OotDemo, active_face: bool) {
    // Keep the earlier relative marker placement, but use one opaque map plate plus
    // non-overlapping decorative cells to avoid depth shimmer on the angled face.
    model.panel(MenuRect::new(18.0, 19.0, 64.0, 60.0), mc(Color::srgba(0.022, 0.070, 0.048, 1.0)), None);
    model.panel(MenuRect::new(23.0, 24.0, 54.0, 43.0), mc(Color::srgba(0.070, 0.125, 0.075, 1.0)), None);
    model.text(50.0, 30.0, 3.0, "HYRULE FIELD", MenuTextAlign::Center, mc(Color::srgb(0.85, 0.88, 0.64)));
    model.text(39.5, 63.5, 2.0, "LAKE", MenuTextAlign::Center, mc(Color::srgb(0.50, 0.68, 0.85)));
    model.text(60.5, 28.5, 2.0, "MTN", MenuTextAlign::Center, mc(Color::srgb(0.83, 0.64, 0.50)));
    model.text(30.0, 48.0, 2.0, "VALLEY", MenuTextAlign::Center, mc(Color::srgb(0.82, 0.69, 0.48)));
    for (idx, marker) in map_markers().iter().enumerate() {
        let action = OotAction::MapMarker(idx);
        model.control_with_icon(
            MenuRect::new(marker.x, marker.y, 8.8, 8.8),
            MenuControlKind::MapMarker,
            marker.short,
            Some(marker.name.to_string()),
            Some("icons/oot/map_marker.png"),
            demo.selected == action,
            idx == 0,
            active_face.then_some(action),
        );
    }
    model.text(50.0, 73.0, 2.55, "Map placeholder: relative locations preserved; simplified layers prevent flicker", MenuTextAlign::Center, mc(Color::srgb(0.74, 0.90, 0.74)));
}

fn add_quest_page(model: &mut MenuPageModel<OotPage, OotAction>, demo: &OotDemo, active_face: bool) {
    model.panel(MenuRect::new(13.5, 18.5, 73.0, 61.0), mc(Color::srgba(0.055, 0.035, 0.070, 1.0)), None);
    model.text(26.0, 23.5, 2.5, "Songs", MenuTextAlign::Center, mc(Color::srgb(0.91, 0.83, 0.55)));
    model.text(69.0, 21.5, 2.5, "Quest Status", MenuTextAlign::Center, mc(Color::srgb(0.91, 0.83, 0.55)));

    // Left-side quest indicators similar to the OoT status page.
    model.control_with_icon(
        MenuRect::new(16.0, 29.0, 7.2, 7.2),
        MenuControlKind::Decoration,
        "Skull",
        Some("100".to_string()),
        Some("icons/oot/skull_token.png"),
        false,
        false,
        None,
    );
    model.text(25.0, 34.0, 2.45, "100", MenuTextAlign::Left, mc(Color::srgb(0.92, 0.88, 0.74)));
    model.control_with_icon(
        MenuRect::new(16.0, 40.5, 7.2, 7.2),
        MenuControlKind::Decoration,
        "Agony",
        None,
        Some("icons/oot/stone_agony.png"),
        false,
        false,
        None,
    );
    model.control_with_icon(
        MenuRect::new(26.0, 40.5, 7.2, 7.2),
        MenuControlKind::Decoration,
        "Card",
        None,
        Some("icons/oot/gerudo_card.png"),
        false,
        false,
        None,
    );

    // Song reminder icons are deliberately smaller than medallions, matching the reference's dense rows.
    for (idx, song) in songs().iter().enumerate() {
        let row = idx / 6;
        let col = idx % 6;
        let x = 18.0 + col as f32 * 5.7;
        let y = 52.0 + row as f32 * 8.0;
        let action = OotAction::Song(idx);
        model.control_with_icon(
            MenuRect::new(x, y, 5.4, 5.4),
            MenuControlKind::Item,
            "",
            None,
            Some(song.icon),
            demo.selected == action,
            true,
            active_face.then_some(action),
        );
    }
    for i in 0..8 {
        let icon = if i % 3 == 0 { "icons/oot/song_button_a.png" } else { "icons/oot/song_button_c.png" };
        model.control_with_icon(
            MenuRect::new(18.0 + i as f32 * 4.2, 68.0, 3.8, 3.8),
            MenuControlKind::Decoration,
            "",
            None,
            Some(icon),
            false,
            false,
            None,
        );
    }

    // Compact medallion hex cluster and stones on the right side.
    let med_pos = [
        (73.0, 34.5), // Forest
        (69.5, 25.0), // Fire
        (60.5, 25.0), // Water
        (56.5, 34.5), // Spirit
        (61.0, 44.0), // Shadow
        (70.0, 44.0), // Light
    ];
    for (idx, q) in quest_icons().iter().enumerate() {
        let action = OotAction::QuestIcon(idx);
        model.control_with_icon(
            MenuRect::new(med_pos[idx].0, med_pos[idx].1, 8.0, 8.0),
            MenuControlKind::Item,
            "",
            None,
            Some(q.icon),
            demo.selected == action,
            true,
            active_face.then_some(action),
        );
    }
    let stone_pos = [(57.0, 57.0), (66.0, 57.0), (75.0, 57.0)];
    let quest_offset = quest_icons().len();
    for (local_idx, q) in stones().iter().enumerate() {
        let idx = quest_offset + local_idx;
        let action = OotAction::QuestIcon(idx);
        model.control_with_icon(
            MenuRect::new(stone_pos[local_idx].0, stone_pos[local_idx].1, 7.5, 7.5),
            MenuControlKind::Item,
            "",
            None,
            Some(q.icon),
            demo.selected == action,
            true,
            active_face.then_some(action),
        );
    }

    // Heart-piece reminder. Four small hearts read better than one huge 48px source quad here.
    for i in 0..4 {
        model.control_with_icon(
            MenuRect::new(60.5 + i as f32 * 5.1, 66.0, 4.8, 4.8),
            MenuControlKind::Decoration,
            "",
            None,
            Some("icons/oot/heart_piece.png"),
            false,
            false,
            None,
        );
    }
    model.text(50.0, 78.7, 2.35, "Quest icons, songs, skulltulas, stones, and heart reminders", MenuTextAlign::Center, mc(Color::srgb(0.82, 0.72, 0.88)));
}

fn add_status_band(model: &mut MenuPageModel<OotPage, OotAction>, demo: &OotDemo) {
    model.panel(
        MenuRect::new(15.0, 86.0, 70.0, 8.0),
        mc(Color::srgba(0.02, 0.02, 0.03, 0.98)),
        None,
    );
    model.text(
        50.0,
        90.0,
        2.8,
        &demo.status,
        MenuTextAlign::Center,
        mc(Color::srgb(0.90, 0.84, 0.64)),
    );
}

fn render_overlay_model(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
    model: &MenuPageModel<OotPage, OotAction>,
) {
    for node in &model.nodes {
        match node {
            MenuNode::Panel { rect, color, action } => spawn_hud_panel(ui, materials, rect.x, rect.y, rect.w, rect.h, menu_color(*color), *action),
            MenuNode::Text { x, y, size, text, align, color } => spawn_hud_text(ui, materials, *x, *y, *size, text, menu_align(*align), menu_srgba(*color)),
            MenuNode::Control { rect, kind, label, detail, icon, selected, important, action } => {
                spawn_hud_control(ui, materials, asset_server, *rect, *kind, label, detail.as_deref(), icon.as_deref(), *selected, *important, *action);
            }
        }
    }
}


fn spawn_hud_control(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
    rect: MenuRect,
    kind: MenuControlKind,
    label: &str,
    detail: Option<&str>,
    icon: Option<&str>,
    selected: bool,
    important: bool,
    action: Option<OotAction>,
) {
    let color = if icon.is_some() { Color::srgba(1.0, 1.0, 1.0, 0.02) } else { control_color(kind, selected, important) };
    let material = materials.add(StandardMaterial {
        base_color: color,
        alpha_mode: AlphaMode::Blend,
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
        Name::new(format!("HUD {:?} control", kind)),
        UiLayout::window()
            .x(Rl(rect.x))
            .y(Rl(rect.y))
            .width(Rl(rect.w))
            .height(Rh(rect.h))
            .anchor(Anchor::TOP_LEFT)
            .pack(),
        UiDepth::Set(DEPTH_HUD_PANEL),
        UiMeshPlane3d,
        MeshMaterial3d(material),
        AmbitionMenuControl { kind, action, focus },
        MenuVisualState { focused: selected, selected, disabled: action.is_none(), ..Default::default() },
        RenderLayers::layer(HUD_RENDER_LAYER),
    ));
    if action.is_some() {
        entity.insert((
            OnHoverSetCursor::new(SystemCursorIcon::Pointer),
            UiHover::new().forward_speed(18.0).backward_speed(10.0),
            UiColor::new(vec![(UiBase::id(), color), (UiHover::id(), hover_panel_color())]),
        ));
    } else {
        entity.insert((UiColor::from(color), Pickable::IGNORE));
    }
    entity.with_children(|children| {
        if let Some(icon_path) = icon {
            spawn_hud_icon(children, materials, asset_server, icon_path);
        }
        if !label.is_empty() {
            spawn_hud_control_text(children, materials, if icon.is_some() { 62.0 } else { 50.0 }, 45.0, 22.0, label, TextAlign::Center, Srgba::rgb_u8(240, 232, 198));
        }
        if let Some(detail) = detail {
            let y = if label.is_empty() { 82.0 } else { 76.0 };
            spawn_hud_control_text(children, materials, if icon.is_some() { 62.0 } else { 50.0 }, y, 10.5, detail, TextAlign::Center, Srgba::rgb_u8(185, 196, 210));
        }
    });
}

fn spawn_hud_icon(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
    icon: &str,
) {
    let texture = asset_server.load(icon.to_string());
    let material = materials.add(StandardMaterial {
        base_color_texture: Some(texture),
        base_color: Color::WHITE,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        unlit: true,
        ..default()
    });
    ui.spawn((
        Name::new(format!("HUD icon {icon}")),
        UiLayout::window().x(Rl(50.0)).y(Rl(50.0)).width(Rl(92.0)).height(Rh(92.0)).anchor(Anchor::CENTER).pack(),
        UiDepth::Set(DEPTH_HUD_ICON),
        UiMeshPlane3d,
        MeshMaterial3d(material),
        Pickable::IGNORE,
        RenderLayers::layer(HUD_RENDER_LAYER),
    ));
}

fn spawn_hud_control_text(ui: &mut ChildSpawnerCommands, materials: &mut Assets<StandardMaterial>, x: f32, y: f32, size: f32, text: &str, align: TextAlign, color: Srgba) {
    let material = materials.add(StandardMaterial { base_color_texture: Some(TextAtlas::DEFAULT_IMAGE), alpha_mode: AlphaMode::Blend, cull_mode: None, unlit: true, ..default() });
    ui.spawn((
        Name::new("HUD control text"),
        UiLayout::window().x(Rl(x)).y(Rl(y)).anchor(Anchor::CENTER).pack(),
        UiDepth::Set(DEPTH_HUD_TEXT),
        UiTextSize::from(Rh(size)),
        Text3d::new(text),
        Text3dStyling { size: 64.0, color, align, font: Arc::from(FONT_FAMILY), weight: Weight::BOLD, ..Default::default() },
        MeshMaterial3d(material),
        Mesh3d::default(),
        Pickable::IGNORE,
        RenderLayers::layer(HUD_RENDER_LAYER),
    ));
}

fn spawn_hud_panel(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: Color,
    action: Option<OotAction>,
) {
    let material = materials.add(StandardMaterial { base_color: color, alpha_mode: AlphaMode::Opaque, cull_mode: None, unlit: true, ..default() });
    let depth = if w < 25.0 && h < 2.0 { DEPTH_HUD_ICON } else { DEPTH_HUD_PANEL };
    let mut entity = ui.spawn((
        Name::new("HUD panel"),
        UiLayout::window().x(Rl(x)).y(Rl(y)).width(Rl(w)).height(Rh(h)).anchor(Anchor::TOP_LEFT).pack(),
        UiDepth::Set(depth),
        UiMeshPlane3d,
        MeshMaterial3d(material),
        RenderLayers::layer(HUD_RENDER_LAYER),
    ));
    if action.is_some() {
        entity.insert((OnHoverSetCursor::new(SystemCursorIcon::Pointer), UiHover::new().forward_speed(18.0).backward_speed(10.0), UiColor::new(vec![(UiBase::id(), color), (UiHover::id(), hover_panel_color())])));
    } else {
        entity.insert((UiColor::from(color), Pickable::IGNORE));
    }
}

fn spawn_hud_text(ui: &mut ChildSpawnerCommands, materials: &mut Assets<StandardMaterial>, x: f32, y: f32, size: f32, text: &str, align: TextAlign, color: Srgba) {
    let material = materials.add(StandardMaterial { base_color_texture: Some(TextAtlas::DEFAULT_IMAGE), alpha_mode: AlphaMode::Blend, cull_mode: None, unlit: true, ..default() });
    ui.spawn((
        Name::new("HUD text"),
        UiLayout::window().x(Rl(x)).y(Rl(y)).anchor(Anchor::CENTER).pack(),
        UiDepth::Set(DEPTH_HUD_TEXT),
        UiTextSize::from(Rh(size)),
        Text3d::new(text),
        Text3dStyling { size: 64.0, color, align, font: Arc::from(FONT_FAMILY), weight: Weight::BOLD, ..Default::default() },
        MeshMaterial3d(material),
        Mesh3d::default(),
        Pickable::IGNORE,
        RenderLayers::layer(HUD_RENDER_LAYER),
    ));
}

fn render_page_model(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
    model: &MenuPageModel<OotPage, OotAction>,
) {
    spawn_panel(ui, materials, 0.0, 0.0, 100.0, 100.0, menu_color(model.background), None);
    spawn_cube_edge_frame(ui, materials);
    for node in &model.nodes {
        match node {
            MenuNode::Panel { rect, color, action } => spawn_panel(ui, materials, rect.x, rect.y, rect.w, rect.h, menu_color(*color), *action),
            MenuNode::Text { x, y, size, text, align, color } => spawn_text(ui, materials, *x, *y, *size, text, menu_align(*align), menu_srgba(*color)),
            MenuNode::Control { rect, kind, label, detail, icon, selected, important, action } => {
                spawn_control(ui, materials, asset_server, *rect, *kind, label, detail.as_deref(), icon.as_deref(), *selected, *important, *action);
            }
        }
    }
}

fn spawn_control(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
    rect: MenuRect,
    kind: MenuControlKind,
    label: &str,
    detail: Option<&str>,
    icon: Option<&str>,
    selected: bool,
    important: bool,
    action: Option<OotAction>,
) {
    let color = control_color(kind, selected, important);
    let material = materials.add(StandardMaterial {
        base_color: color,
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
        Name::new(format!("{:?} control", kind)),
        UiLayout::window()
            .x(Rl(rect.x))
            .y(Rl(rect.y))
            .width(Rl(rect.w))
            .height(Rh(rect.h))
            .anchor(Anchor::TOP_LEFT)
            .pack(),
        UiDepth::Set(panel_depth(rect.w, rect.h, action.is_some())),
        UiMeshPlane3d,
        MeshMaterial3d(material),
        AmbitionMenuControl { kind, action, focus },
        MenuVisualState { focused: selected, selected, disabled: action.is_none(), ..Default::default() },
    ));
    if action.is_some() {
        entity.insert((
            OnHoverSetCursor::new(SystemCursorIcon::Pointer),
            UiHover::new().forward_speed(18.0).backward_speed(10.0),
            UiColor::new(vec![(UiBase::id(), color), (UiHover::id(), hover_panel_color())]),
        ));
    } else {
        entity.insert((UiColor::from(color), Pickable::IGNORE));
    }
    entity.with_children(|children| {
        let icon_is_primary = matches!(kind, MenuControlKind::Item | MenuControlKind::MapMarker | MenuControlKind::Decoration);
        if let Some(icon_path) = icon {
            spawn_icon(children, materials, asset_server, icon_path, icon_is_primary);
        }
        if icon_is_primary {
            if !label.is_empty() {
                spawn_control_text(children, materials, 50.0, 86.0, 14.0, label, TextAlign::Center, Srgba::rgb_u8(240, 232, 198));
            }
            if let Some(detail) = detail {
                spawn_control_text(children, materials, 50.0, 108.0, 10.5, detail, TextAlign::Center, Srgba::rgb_u8(185, 196, 210));
            }
        } else {
            let text_x = if icon.is_some() { 62.0 } else { 50.0 };
            let size = if rect.h < 8.5 { 20.0 } else { 22.0 };
            spawn_control_text(children, materials, text_x, 45.0, size, label, TextAlign::Center, Srgba::rgb_u8(240, 232, 198));
            if let Some(detail) = detail {
                spawn_control_text(children, materials, text_x, 76.0, size * 0.72, detail, TextAlign::Center, Srgba::rgb_u8(185, 196, 210));
            }
        }
    });
}

fn spawn_icon(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
    icon: &str,
    primary: bool,
) {
    // This function is called as a child of a control. Its layout is therefore in
    // control-local percentages, not page percentages. The earlier demo used page
    // coordinates here, which made icons tiny and off-center inside the buttons.
    let icon_size = if primary { 86.0 } else { 58.0 };
    let x = if primary { 50.0 } else { 23.0 };
    let y = if primary { 47.0 } else { 50.0 };
    let texture = asset_server.load(icon.to_string());
    let material = materials.add(StandardMaterial {
        base_color_texture: Some(texture),
        base_color: Color::WHITE,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        unlit: true,
        ..default()
    });
    ui.spawn((
        Name::new(format!("Icon {icon}")),
        UiLayout::window()
            .x(Rl(x))
            .y(Rl(y))
            .width(Rl(icon_size))
            .height(Rh(icon_size))
            .anchor(Anchor::CENTER)
            .pack(),
        UiDepth::Set(DEPTH_ICON),
        UiMeshPlane3d,
        MeshMaterial3d(material),
        Pickable::IGNORE,
    ));
}

fn spawn_control_text(ui: &mut ChildSpawnerCommands, materials: &mut Assets<StandardMaterial>, x: f32, y: f32, size: f32, text: &str, align: TextAlign, color: Srgba) {
    let material = materials.add(StandardMaterial { base_color_texture: Some(TextAtlas::DEFAULT_IMAGE), alpha_mode: AlphaMode::Blend, cull_mode: None, unlit: true, ..default() });
    ui.spawn((
        Name::new("OoT control text"),
        UiLayout::window().x(Rl(x)).y(Rl(y)).anchor(Anchor::CENTER).pack(),
        UiDepth::Set(DEPTH_TEXT_TOP),
        UiTextSize::from(Rh(size)),
        Text3d::new(text),
        Text3dStyling { size: 64.0, color, align, font: Arc::from(FONT_FAMILY), weight: Weight::BOLD, ..Default::default() },
        MeshMaterial3d(material),
        Mesh3d::default(),
        Pickable::IGNORE,
    ));
}

fn spawn_cube_edge_frame(ui: &mut ChildSpawnerCommands, materials: &mut Assets<StandardMaterial>) {
    let edge = Color::srgba(0.76, 0.58, 0.24, 0.98);
    spawn_panel_at_depth(ui, materials, 0.0, 0.0, 1.0, 100.0, edge, DEPTH_EDGE);
    spawn_panel_at_depth(ui, materials, 99.0, 0.0, 1.0, 100.0, edge, DEPTH_EDGE);
    spawn_panel_at_depth(ui, materials, 0.0, 0.0, 100.0, 0.8, edge, DEPTH_EDGE);
    spawn_panel_at_depth(ui, materials, 0.0, 99.2, 100.0, 0.8, edge, DEPTH_EDGE);
}

fn spawn_panel(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: Color,
    action: Option<OotAction>,
) {
    let material = materials.add(StandardMaterial { base_color: color, alpha_mode: AlphaMode::Opaque, cull_mode: None, unlit: true, ..default() });
    let mut entity = ui.spawn((
        Name::new("OoT panel"),
        UiLayout::window().x(Rl(x)).y(Rl(y)).width(Rl(w)).height(Rh(h)).anchor(Anchor::TOP_LEFT).pack(),
        UiDepth::Set(panel_depth_at(x, y, w, h, action.is_some())),
        UiMeshPlane3d,
        MeshMaterial3d(material),
    ));
    if action.is_some() {
        entity.insert((OnHoverSetCursor::new(SystemCursorIcon::Pointer), UiHover::new().forward_speed(18.0).backward_speed(10.0), UiColor::new(vec![(UiBase::id(), color), (UiHover::id(), hover_panel_color())])));
    } else {
        entity.insert((UiColor::from(color), Pickable::IGNORE));
    }
}

fn spawn_panel_at_depth(ui: &mut ChildSpawnerCommands, materials: &mut Assets<StandardMaterial>, x: f32, y: f32, w: f32, h: f32, color: Color, depth: f32) {
    let material = materials.add(StandardMaterial { base_color: color, alpha_mode: AlphaMode::Opaque, cull_mode: None, unlit: true, ..default() });
    ui.spawn((
        Name::new("OoT depth panel"),
        UiLayout::window().x(Rl(x)).y(Rl(y)).width(Rl(w)).height(Rh(h)).anchor(Anchor::TOP_LEFT).pack(),
        UiDepth::Set(depth),
        UiMeshPlane3d,
        MeshMaterial3d(material),
        UiColor::from(color),
        Pickable::IGNORE,
    ));
}

fn spawn_text(ui: &mut ChildSpawnerCommands, materials: &mut Assets<StandardMaterial>, x: f32, y: f32, size: f32, text: &str, align: TextAlign, color: Srgba) {
    let material = materials.add(StandardMaterial { base_color_texture: Some(TextAtlas::DEFAULT_IMAGE), alpha_mode: AlphaMode::Blend, cull_mode: None, unlit: true, ..default() });
    ui.spawn((
        Name::new("OoT text"),
        UiLayout::window().x(Rl(x)).y(Rl(y)).anchor(Anchor::CENTER).pack(),
        UiDepth::Set(DEPTH_TEXT_TOP),
        UiTextSize::from(Rh(size)),
        Text3d::new(text),
        Text3dStyling { size: 64.0, color, align, font: Arc::from(FONT_FAMILY), weight: Weight::BOLD, ..Default::default() },
        MeshMaterial3d(material),
        Mesh3d::default(),
        Pickable::IGNORE,
    ));
}

fn control_color(kind: MenuControlKind, selected: bool, important: bool) -> Color {
    match kind {
        MenuControlKind::Tab if selected => Color::srgba(0.78, 0.55, 0.20, 0.98),
        MenuControlKind::Tab => Color::srgba(0.10, 0.08, 0.12, 0.95),
        _ => focus_color(selected, important),
    }
}

fn focus_color(selected: bool, important: bool) -> Color {
    match (selected, important) {
        (true, true) => Color::srgba(0.82, 0.58, 0.20, 0.98),
        (true, false) => Color::srgba(0.45, 0.48, 0.68, 0.96),
        (false, true) => Color::srgba(0.18, 0.15, 0.09, 0.94),
        (false, false) => Color::srgba(0.08, 0.08, 0.12, 0.92),
    }
}

fn hover_panel_color() -> Color {
    Color::srgba(0.88, 0.70, 0.28, 0.99)
}

fn panel_depth(w: f32, h: f32, actionable: bool) -> f32 {
    panel_depth_at(0.0, 0.0, w, h, actionable)
}

fn panel_depth_at(x: f32, y: f32, w: f32, h: f32, actionable: bool) -> f32 {
    if actionable {
        return DEPTH_ACTION;
    }
    let area = w * h;
    // Avoid z-fighting between nested non-action panels. Small HUD bars are
    // intentionally biased by position/size so the magic fill and backing never
    // occupy the exact same plane.
    let base = if area > 8_000.0 {
        DEPTH_BACKGROUND
    } else if area > 3_000.0 {
        DEPTH_LARGE_PANEL
    } else if area > 1_200.0 {
        DEPTH_LARGE_PANEL - 0.08
    } else if area > 500.0 {
        DEPTH_CARD
    } else {
        DEPTH_CARD - 0.05
    };
    let stable_bias = ((x * 13.0 + y * 17.0 + w * 19.0 + h * 23.0).round() % 97.0) * 0.00005;
    base - stable_bias
}

fn mc(color: Color) -> MenuColor {
    let srgba = color.to_srgba();
    MenuColor::rgba(srgba.red, srgba.green, srgba.blue, srgba.alpha)
}

fn menu_color(color: MenuColor) -> Color {
    Color::srgba(color.r, color.g, color.b, color.a)
}

fn menu_srgba(color: MenuColor) -> Srgba {
    let r = (color.r.clamp(0.0, 1.0) * 255.0).round() as u8;
    let g = (color.g.clamp(0.0, 1.0) * 255.0).round() as u8;
    let b = (color.b.clamp(0.0, 1.0) * 255.0).round() as u8;
    Srgba::rgb_u8(r, g, b)
}

fn menu_align(align: MenuTextAlign) -> TextAlign {
    match align {
        MenuTextAlign::Left => TextAlign::Left,
        MenuTextAlign::Center => TextAlign::Center,
        MenuTextAlign::Right => TextAlign::Right,
    }
}

fn menu_toggle_input(keys: Res<ButtonInput<KeyCode>>, gamepads: Query<&Gamepad>, mut shell: ResMut<MenuShell>, mut demo: ResMut<OotDemo>) {
    let keyboard_pause = keys.just_pressed(KeyCode::KeyP);
    let keyboard_cancel = keys.just_pressed(KeyCode::Escape);
    let gamepad_start = gamepads.iter().any(|g| g.just_pressed(GamepadButton::Start));
    if (keyboard_cancel || gamepad_start) && demo.save_modal_active() {
        if demo.save_prompt_open {
            demo.toggle_save_prompt();
        }
        return;
    }
    if keyboard_pause || keyboard_cancel || gamepad_start {
        shell.toggle();
    }
}

fn keyboard_navigation(keys: Res<ButtonInput<KeyCode>>, shell: Res<MenuShell>, mut demo: ResMut<OotDemo>, mut menu: ResMut<MenuAnimation>) {
    if !shell.is_interactive() {
        return;
    }
    if demo.save_modal_active() {
        if keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::KeyA) {
            demo.choose_save_yes();
        }
        if keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::KeyD) {
            demo.choose_save_no();
        }
        if keys.just_pressed(KeyCode::KeyB) || keys.just_pressed(KeyCode::Escape) {
            if demo.save_prompt_open {
                demo.toggle_save_prompt();
            }
        }
        if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
            match demo.selected {
                OotAction::SaveYes => demo.click(OotAction::SaveYes),
                OotAction::SaveNo => demo.click(OotAction::SaveNo),
                _ => demo.choose_save_yes(),
            }
        }
        return;
    }
    let before_page = demo.page;
    if keys.just_pressed(KeyCode::KeyQ) || keys.just_pressed(KeyCode::PageUp) {
        demo.turn_page(PageTurn::ViewerLeft);
    }
    if keys.just_pressed(KeyCode::KeyE) || keys.just_pressed(KeyCode::PageDown) {
        demo.turn_page(PageTurn::ViewerRight);
    }
    if keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::KeyA) {
        demo.move_spatial(-1, 0);
    }
    if keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::KeyD) {
        demo.move_spatial(1, 0);
    }
    if keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::KeyW) {
        demo.move_spatial(0, -1);
    }
    if keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::KeyS) {
        demo.move_spatial(0, 1);
    }
    if keys.just_pressed(KeyCode::KeyZ) {
        demo.assign_selected_item_to_c_button(CButton::Left);
    }
    if keys.just_pressed(KeyCode::KeyX) {
        demo.assign_selected_item_to_c_button(CButton::Down);
    }
    if keys.just_pressed(KeyCode::KeyC) {
        demo.assign_selected_item_to_c_button(CButton::Right);
    }
    if keys.just_pressed(KeyCode::KeyB) {
        demo.press_b_button();
    }
    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
        demo.activate_selected();
    }
    if demo.page != before_page {
        menu.set_page(demo.page);
    }
}

fn gamepad_navigation(
    gamepads: Query<&Gamepad>,
    shell: Res<MenuShell>,
    mut demo: ResMut<OotDemo>,
    mut menu: ResMut<MenuAnimation>,
    mut c_stick: ResMut<GamepadCStickState>,
    mut nav_stick: ResMut<GamepadNavStickState>,
) {
    if !shell.is_interactive() {
        c_stick.active = None;
        nav_stick.active = None;
        return;
    }
    if demo.save_modal_active() {
        let mut any_nav_stick_direction = None;
        for gamepad in &gamepads {
            if gamepad.just_pressed(GamepadButton::DPadLeft) {
                demo.choose_save_yes();
            }
            if gamepad.just_pressed(GamepadButton::DPadRight) {
                demo.choose_save_no();
            }
            if gamepad.just_pressed(GamepadButton::South) {
                match demo.selected {
                    OotAction::SaveYes => demo.click(OotAction::SaveYes),
                    OotAction::SaveNo => demo.click(OotAction::SaveNo),
                    _ => demo.choose_save_yes(),
                }
            }
            if gamepad.just_pressed(GamepadButton::East) || gamepad.just_pressed(GamepadButton::Start) {
                if demo.save_prompt_open {
                    demo.toggle_save_prompt();
                }
            }
            let nav_x = gamepad.get(GamepadAxis::LeftStickX).unwrap_or(0.0);
            let nav_y = gamepad.get(GamepadAxis::LeftStickY).unwrap_or(0.0);
            any_nav_stick_direction = any_nav_stick_direction.or_else(|| nav_direction_from_left_stick(nav_x, nav_y));
        }
        if any_nav_stick_direction != nav_stick.active {
            if let Some((dx, _dy)) = any_nav_stick_direction {
                if dx < 0 { demo.choose_save_yes(); }
                if dx > 0 { demo.choose_save_no(); }
            }
            nav_stick.active = any_nav_stick_direction;
        }
        c_stick.active = None;
        return;
    }
    let before_page = demo.page;
    let mut any_c_stick_direction = None;
    let mut any_nav_stick_direction = None;
    for gamepad in &gamepads {
        if gamepad.just_pressed(GamepadButton::LeftTrigger) || gamepad.just_pressed(GamepadButton::LeftTrigger2) {
            demo.turn_page(PageTurn::ViewerLeft);
        }
        if gamepad.just_pressed(GamepadButton::RightTrigger) || gamepad.just_pressed(GamepadButton::RightTrigger2) {
            demo.turn_page(PageTurn::ViewerRight);
        }
        if gamepad.just_pressed(GamepadButton::DPadLeft) {
            demo.move_spatial(-1, 0);
        }
        if gamepad.just_pressed(GamepadButton::DPadRight) {
            demo.move_spatial(1, 0);
        }
        if gamepad.just_pressed(GamepadButton::DPadUp) {
            demo.move_spatial(0, -1);
        }
        if gamepad.just_pressed(GamepadButton::DPadDown) {
            demo.move_spatial(0, 1);
        }
        if gamepad.just_pressed(GamepadButton::South) {
            demo.activate_selected();
        }
        // Left stick is regular menu navigation. Trigger once when crossing the
        // dead zone so holding the stick does not race across the grid.
        let nav_x = gamepad.get(GamepadAxis::LeftStickX).unwrap_or(0.0);
        let nav_y = gamepad.get(GamepadAxis::LeftStickY).unwrap_or(0.0);
        any_nav_stick_direction = any_nav_stick_direction.or_else(|| nav_direction_from_left_stick(nav_x, nav_y));

        // In the N64 layout these are C-left/C-down/C-right, not focusable
        // menu controls. On modern pads, use the right stick as the C-button
        // cluster: push left/down/right to assign the highlighted inventory item.
        let c_x = gamepad.get(GamepadAxis::RightStickX).unwrap_or(0.0);
        let c_y = gamepad.get(GamepadAxis::RightStickY).unwrap_or(0.0);
        any_c_stick_direction = any_c_stick_direction.or_else(|| c_button_from_right_stick(c_x, c_y));

        // Keep the face-button fallback for controllers or keyboards that do not
        // expose reliable analog stick events, but do not move the cursor.
        if gamepad.just_pressed(GamepadButton::West) {
            demo.assign_selected_item_to_c_button(CButton::Left);
        }
        if gamepad.just_pressed(GamepadButton::North) {
            demo.assign_selected_item_to_c_button(CButton::Down);
        }
        if gamepad.just_pressed(GamepadButton::East) {
            demo.press_b_button();
        }
    }
    if any_nav_stick_direction != nav_stick.active {
        if let Some((dx, dy)) = any_nav_stick_direction {
            demo.move_spatial(dx, dy);
        }
        nav_stick.active = any_nav_stick_direction;
    }
    if any_c_stick_direction != c_stick.active {
        if let Some(button) = any_c_stick_direction {
            demo.assign_selected_item_to_c_button(button);
        }
        c_stick.active = any_c_stick_direction;
    }
    if demo.page != before_page {
        menu.set_page(demo.page);
    }
}

fn c_button_from_right_stick(x: f32, y: f32) -> Option<CButton> {
    const DEAD_ZONE: f32 = 0.62;
    let ax = x.abs();
    let ay = y.abs();
    if ax < DEAD_ZONE && ay < DEAD_ZONE {
        return None;
    }
    if ax >= ay {
        if x < 0.0 { Some(CButton::Left) } else { Some(CButton::Right) }
    } else if y < 0.0 {
        Some(CButton::Down)
    } else {
        // C-up is not an inventory assignment slot in this demo.
        None
    }
}

fn nav_direction_from_left_stick(x: f32, y: f32) -> Option<(i32, i32)> {
    const DEAD_ZONE: f32 = 0.62;
    let ax = x.abs();
    let ay = y.abs();
    if ax < DEAD_ZONE && ay < DEAD_ZONE {
        return None;
    }
    if ax >= ay {
        if x < 0.0 { Some((-1, 0)) } else { Some((1, 0)) }
    } else if y < 0.0 {
        Some((0, 1))
    } else {
        Some((0, -1))
    }
}


fn animate_equip_and_save(time: Res<Time>, shell: Res<MenuShell>, mut demo: ResMut<OotDemo>) {
    if !shell.is_visible() {
        return;
    }
    let save_step = 1.0 - (-10.0 * time.delta_secs()).exp();
    let next_save = demo.save_flip + (demo.save_flip_target - demo.save_flip) * save_step;
    if (next_save - demo.save_flip).abs() > 0.001 {
        demo.save_flip = next_save;
        if (demo.save_flip - demo.save_flip_target).abs() < 0.004 {
            demo.save_flip = demo.save_flip_target;
        }
        demo.bump();
    }
    if let Some(mut anim) = demo.equip_anim {
        let speed = match anim.phase {
            EquipAnimPhase::ItemToButton => 4.5,
            EquipAnimPhase::ArrowGlowToBow => 5.8,
            EquipAnimPhase::ArrowBowHold => 3.5,
            EquipAnimPhase::BowToButton => 4.5,
        };
        anim.progress += time.delta_secs() * speed;
        if anim.progress >= 1.0 {
            match anim.phase {
                EquipAnimPhase::ItemToButton => {
                    demo.finish_c_button_equip(anim.item_idx, anim.target_button);
                    return;
                }
                EquipAnimPhase::ArrowGlowToBow => {
                    anim.phase = EquipAnimPhase::ArrowBowHold;
                    anim.progress = 0.0;
                }
                EquipAnimPhase::ArrowBowHold => {
                    anim.phase = EquipAnimPhase::BowToButton;
                    anim.progress = 0.0;
                }
                EquipAnimPhase::BowToButton => {
                    demo.finish_c_button_equip(anim.item_idx, anim.target_button);
                    return;
                }
            }
        }
        demo.equip_anim = Some(anim);
        demo.bump();
    }
}

fn apply_save_flip(face: OotPage, active: OotPage, amount: f32, transform: &mut Transform) {
    if face != active || amount <= 0.001 {
        return;
    }
    // Two-phase prompt flip: rotate the active face to edge-on, swap contents in
    // add_save_prompt_panel at the midpoint, then rotate back to a front-facing
    // prompt. A full 180 degree rotation leaves the text upside down/back-facing
    // in this inside-cube setup, which was the source of the broken B-to-save
    // display.
    let t = amount.clamp(0.0, 1.0);
    let half = if t < 0.5 { t * 2.0 } else { (1.0 - t) * 2.0 };
    let eased = smoothstep(half);
    let a = FRAC_PI_2 * eased;
    transform.rotation = transform.rotation * Quat::from_rotation_x(a);
    transform.scale.y *= a.cos().abs().max(0.08);
}

fn mouse_navigation(mut wheel: MessageReader<MouseWheel>, shell: Res<MenuShell>, mut demo: ResMut<OotDemo>, mut menu: ResMut<MenuAnimation>) {
    if !shell.is_interactive() {
        return;
    }
    if demo.save_modal_active() {
        for _ in wheel.read() {}
        return;
    }
    let before_page = demo.page;
    for ev in wheel.read() {
        if ev.y > 0.0 {
            demo.turn_page(PageTurn::ViewerRight);
        } else if ev.y < 0.0 {
            demo.turn_page(PageTurn::ViewerLeft);
        }
    }
    if demo.page != before_page {
        menu.set_page(demo.page);
    }
}

fn animate_menu_ring(
    time: Res<Time>,
    config: Res<MenuShellConfig>,
    mut menu: ResMut<MenuAnimation>,
    mut shell: ResMut<MenuShell>,
    mut effects: ResMut<MenuShellEffects>,
    demo: Res<OotDemo>,
    mut last_phase: Local<Option<MenuShellPhase>>,
    mut ring_query: Query<(&mut Transform, &mut Visibility), (With<MenuRing>, Without<LunexFaceRoot>)>,
    mut face_query: Query<(&PageFace, &mut Transform), (With<LunexFaceRoot>, Without<MenuRing>)>,
    mut hud_query: Query<(&mut Transform, &mut Visibility), (With<HudOverlayRoot>, Without<MenuRing>, Without<LunexFaceRoot>)>,
) {
    let Ok((mut transform, mut visibility)) = ring_query.single_mut() else { return; };
    let delta = shortest_angle_delta(menu.current_angle, menu.target_angle);
    let rotate_step = 1.0 - (-config.page_rotate_speed * time.delta_secs()).exp();
    menu.current_angle += delta * rotate_step;
    if delta.abs() < 0.001 {
        menu.current_angle = menu.target_angle;
    }
    let target = if shell.target_open { 1.0 } else { 0.0 };
    let open_step = 1.0 - (-config.open_close_speed * time.delta_secs()).exp();
    shell.openness += (target - shell.openness) * open_step;
    if (shell.openness - target).abs() < 0.002 {
        shell.openness = target;
    }
    *visibility = if shell.is_visible() { Visibility::Visible } else { Visibility::Hidden };
    for (mut hud_transform, mut hud_visibility) in &mut hud_query {
        *hud_visibility = if shell.is_visible() { Visibility::Visible } else { Visibility::Hidden };
        let open = smoothstep(shell.openness.clamp(0.0, 1.0));
        hud_transform.translation = Vec3::new(0.0, -0.10 * (1.0 - open), PAGE_RADIUS - HUD_Z_OFFSET_TOWARD_CAMERA);
        let hud_scale = MIN_OPEN_SCALE + (1.0 - MIN_OPEN_SCALE) * open;
        hud_transform.scale = Vec3::new(HUD_SCREEN_X_FLIP * hud_scale, hud_scale, hud_scale);
        hud_transform.rotation = Quat::IDENTITY;
    }
    let phase = shell.phase();
    if *last_phase != Some(phase) {
        effects.push(match phase {
            MenuShellPhase::Opening => MenuShellEffect::Opening,
            MenuShellPhase::Open => MenuShellEffect::Opened,
            MenuShellPhase::Closing => MenuShellEffect::Closing,
            MenuShellPhase::Closed => MenuShellEffect::Closed,
        });
        *last_phase = Some(phase);
    }
    let open = smoothstep(shell.openness.clamp(0.0, 1.0));
    transform.rotation = Quat::from_rotation_y(menu.current_angle);
    match config.open_close_style {
        MenuOpenCloseStyle::SmoothScale => {
            let scale = MIN_OPEN_SCALE + (1.0 - MIN_OPEN_SCALE) * open;
            transform.scale = Vec3::splat(scale);
            transform.translation = Vec3::new(0.0, -0.05 * (1.0 - open), -0.42 * (1.0 - open));
            for (face, mut t) in &mut face_query {
                reset_face_transform(face.0, &mut t);
                apply_save_flip(face.0, demo.page, demo.save_flip, &mut t);
            }
        }
        MenuOpenCloseStyle::OotPageFold => {
            transform.scale = Vec3::ONE;
            transform.translation = Vec3::new(0.0, -0.10 * (1.0 - open), 0.0);
            let fold = OOT_PAGE_FOLD_RADIANS * (1.0 - open);
            for (face, mut t) in &mut face_query {
                apply_oot_open_fold(face.0, fold, &mut t);
                apply_save_flip(face.0, demo.page, demo.save_flip, &mut t);
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct HitRect { x: f32, y: f32, w: f32, h: f32 }
impl HitRect {
    fn center(self) -> Vec2 { Vec2::new(self.x + self.w * 0.5, self.y + self.h * 0.5) }
}
#[derive(Clone, Copy, Debug)]
struct HitTarget { rect: HitRect, action: OotAction }

fn model_hit_targets(model: &MenuPageModel<OotPage, OotAction>) -> Vec<HitTarget> {
    model.nodes.iter().filter_map(|node| match node {
        MenuNode::Panel { rect, action: Some(action), .. } => Some(HitTarget { rect: HitRect { x: rect.x, y: rect.y, w: rect.w, h: rect.h }, action: *action }),
        MenuNode::Control { rect, action: Some(action), .. } => Some(HitTarget { rect: HitRect { x: rect.x, y: rect.y, w: rect.w, h: rect.h }, action: *action }),
        _ => None,
    }).collect()
}

fn active_page_hit_targets(demo: &OotDemo) -> Vec<HitTarget> {
    let model = build_page_model(demo.page, demo, true);
    model_hit_targets(&model)
}

fn active_hud_hit_targets(demo: &OotDemo) -> Vec<HitTarget> {
    let model = build_pause_hud_model(demo);
    model_hit_targets(&model)
}

fn pointer_hit_test(
    windows: Query<&Window, With<PrimaryWindow>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut touches: MessageReader<TouchInput>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainPauseCamera>>,
    face_query: Query<(&PageFace, &GlobalTransform)>,
    hud_query: Query<&GlobalTransform, With<HudOverlayRoot>>,
    shell: Res<MenuShell>,
    mut demo: ResMut<OotDemo>,
    mut menu: ResMut<MenuAnimation>,
    mut last_mouse_hover: Local<Option<OotAction>>,
) {
    if !shell.is_interactive() { return; }
    let Ok(window) = windows.single() else { return; };
    let Ok((camera, camera_transform)) = camera_query.single() else { return; };
    let Some((_, face_transform)) = face_query.iter().find(|(face, _)| face.0 == demo.page) else { return; };
    let hud_transform = hud_query.single().ok();
    let before_page = demo.page;

    if let Some(pos) = window.cursor_position() {
        let hovered = hud_transform
            .and_then(|hud| hit_test_targets(pos, &active_hud_hit_targets(&demo), camera, camera_transform, hud))
            .or_else(|| hit_test_targets(pos, &active_page_hit_targets(&demo), camera, camera_transform, face_transform));
        if hovered != *last_mouse_hover {
            if let Some(action) = hovered { demo.hover(action); }
            *last_mouse_hover = hovered;
        }
        if buttons.just_released(MouseButton::Left) {
            if let Some(action) = hovered { demo.click(action); }
        }
        if buttons.just_released(MouseButton::Right) {
            demo.status = "Cancel/back.".to_string();
            demo.bump();
        }
    }
    for touch in touches.read() {
        if touch.phase == TouchPhase::Ended {
            let action = hud_transform
                .and_then(|hud| hit_test_targets(touch.position, &active_hud_hit_targets(&demo), camera, camera_transform, hud))
                .or_else(|| hit_test_targets(touch.position, &active_page_hit_targets(&demo), camera, camera_transform, face_transform));
            if let Some(action) = action { demo.click(action); }
        }
    }
    if demo.page != before_page {
        menu.set_page(demo.page);
    }
}

fn hit_test_targets(cursor: Vec2, targets: &[HitTarget], camera: &Camera, camera_transform: &GlobalTransform, face_transform: &GlobalTransform) -> Option<OotAction> {
    let mut best: Option<(f32, OotAction)> = None;
    for target in targets {
        let mut min = Vec2::splat(f32::INFINITY);
        let mut max = Vec2::splat(f32::NEG_INFINITY);
        let mut ok = true;
        for local in rect_corners(target.rect) {
            let world = face_transform.transform_point(local);
            let Ok(screen) = camera.world_to_viewport(camera_transform, world) else { ok = false; break; };
            min = min.min(screen);
            max = max.max(screen);
        }
        if !ok { continue; }
        if cursor.x >= min.x && cursor.x <= max.x && cursor.y >= min.y && cursor.y <= max.y {
            let area = (max.x - min.x).abs() * (max.y - min.y).abs();
            if best.map(|(best_area, _)| area < best_area).unwrap_or(true) {
                best = Some((area, target.action));
            }
        }
    }
    best.map(|(_, action)| action)
}

fn rect_corners(rect: HitRect) -> [Vec3; 4] {
    let x0 = rect.x;
    let x1 = rect.x + rect.w;
    let y0 = rect.y;
    let y1 = rect.y + rect.h;
    [page_pct_to_local(x0, y0), page_pct_to_local(x1, y0), page_pct_to_local(x1, y1), page_pct_to_local(x0, y1)]
}

fn page_pct_to_local(x: f32, y: f32) -> Vec3 {
    Vec3::new((x / 100.0 - 0.5) * PAGE_W, (0.5 - y / 100.0) * PAGE_H, 0.0)
}

fn smoothstep(t: f32) -> f32 { t * t * (3.0 - 2.0 * t) }
fn shortest_angle_delta(current: f32, target: f32) -> f32 {
    let two_pi = PI * 2.0;
    (target - current + PI).rem_euclid(two_pi) - PI
}

fn item_grid_center(idx: usize) -> Vec2 {
    let cols = 6;
    let cell_w = 10.0;
    let cell_h = 11.5;
    let gap_x = 1.4;
    let gap_y = 1.5;
    let x0 = 17.0;
    let y0 = 24.0;
    let col = idx % cols;
    let row = idx / cols;
    Vec2::new(x0 + col as f32 * (cell_w + gap_x) + cell_w * 0.5, y0 + row as f32 * (cell_h + gap_y) + cell_h * 0.5)
}

fn c_button_center(button: CButton) -> Vec2 {
    let rect = match button {
        CButton::Left => C_LEFT_RECT,
        CButton::Down => C_DOWN_RECT,
        CButton::Right => C_RIGHT_RECT,
    };
    Vec2::new(rect.x + rect.w * 0.5, rect.y + rect.h * 0.5)
}

fn bow_item_index() -> usize { 3 }

fn arrow_kind(item_idx: usize) -> Option<ArrowKind> {
    match item_idx {
        4 => Some(ArrowKind::Fire),
        10 => Some(ArrowKind::Ice),
        16 => Some(ArrowKind::Light),
        _ => None,
    }
}

fn c_slot_family(item_idx: usize) -> CSlotFamily {
    if item_idx == bow_item_index() || arrow_kind(item_idx).is_some() {
        CSlotFamily::Bow
    } else {
        CSlotFamily::Item(item_idx)
    }
}

fn lerp_vec2(a: Vec2, b: Vec2, t: f32) -> Vec2 {
    a + (b - a) * smoothstep(t.clamp(0.0, 1.0))
}

fn add_equip_anim_visual(model: &mut MenuPageModel<OotPage, OotAction>, anim: EquipAnim) {
    let t = anim.progress.clamp(0.0, 1.0);
    let (pos, icon, label, size) = match anim.phase {
        EquipAnimPhase::ItemToButton => (lerp_vec2(anim.from, anim.to, t), oot_items()[anim.item_idx].icon, "", 7.0),
        EquipAnimPhase::ArrowGlowToBow => {
            let kind = arrow_kind(anim.item_idx).unwrap_or(ArrowKind::Fire);
            (lerp_vec2(anim.from, anim.via, t), kind.glow_icon(), "glow", 7.2 + 1.6 * (1.0 - t))
        }
        EquipAnimPhase::ArrowBowHold => {
            let kind = arrow_kind(anim.item_idx).unwrap_or(ArrowKind::Fire);
            (anim.via, kind.glow_icon(), "glow", 8.2 + (t * PI * 4.0).sin().abs())
        }
        EquipAnimPhase::BowToButton => (lerp_vec2(anim.via, anim.to, t), oot_items()[anim.item_idx].icon, "", 7.0),
    };
    model.control_with_icon(
        MenuRect::new(pos.x - size * 0.5, pos.y - size * 0.5, size, size),
        MenuControlKind::Decoration,
        label,
        None,
        Some(icon),
        true,
        true,
        None,
    );
}

#[derive(Clone, Copy)]
struct OotItem { name: &'static str, _short: &'static str, icon: &'static str, detail: Option<&'static str>, important: bool }
fn oot_items() -> [OotItem; 24] {
    // Source-like inventory slot order from OoT's InventorySlot enum:
    // row 1: sticks/nuts/bombs/bow/fire/din
    // row 2: slingshot/ocarina/bombchu/hookshot/ice/farore
    // row 3: boomerang/lens/beans/hammer/light/nayru
    // row 4: bottle1..4/adult trade/child trade
    [
        OotItem { name: "Deku Stick", _short: "Stick", icon: "icons/oot/deku_stick.png", detail: Some("x99"), important: false },
        OotItem { name: "Deku Nut", _short: "Nut", icon: "icons/oot/deku_nut.png", detail: Some("x99"), important: false },
        OotItem { name: "Bomb", _short: "Bomb", icon: "icons/oot/bomb.png", detail: Some("x99"), important: false },
        OotItem { name: "Fairy Bow", _short: "Bow", icon: "icons/oot/bow.png", detail: Some("x50"), important: true },
        OotItem { name: "Fire Arrow", _short: "Fire", icon: "icons/oot/fire_arrow.png", detail: None, important: true },
        OotItem { name: "Din's Fire", _short: "Din", icon: "icons/oot/dins_fire.png", detail: None, important: true },
        OotItem { name: "Fairy Slingshot", _short: "Shot", icon: "icons/oot/slingshot.png", detail: Some("x50"), important: true },
        OotItem { name: "Ocarina of Time", _short: "Ocarina", icon: "icons/oot/ocarina.png", detail: None, important: true },
        OotItem { name: "Bombchu", _short: "Bombchu", icon: "icons/oot/bombchu.png", detail: Some("x50"), important: false },
        OotItem { name: "Longshot", _short: "Long", icon: "icons/oot/longshot.png", detail: None, important: true },
        OotItem { name: "Ice Arrow", _short: "Ice", icon: "icons/oot/ice_arrow.png", detail: None, important: true },
        OotItem { name: "Farore's Wind", _short: "Farore", icon: "icons/oot/farores_wind.png", detail: None, important: true },
        OotItem { name: "Boomerang", _short: "Boom", icon: "icons/oot/boomerang.png", detail: None, important: true },
        OotItem { name: "Lens of Truth", _short: "Lens", icon: "icons/oot/lens.png", detail: None, important: true },
        OotItem { name: "Magic Bean", _short: "Bean", icon: "icons/oot/beans.png", detail: Some("x10"), important: false },
        OotItem { name: "Megaton Hammer", _short: "Hammer", icon: "icons/oot/hammer.png", detail: None, important: true },
        OotItem { name: "Light Arrow", _short: "Light", icon: "icons/oot/light_arrow.png", detail: None, important: true },
        OotItem { name: "Nayru's Love", _short: "Nayru", icon: "icons/oot/nayrus_love.png", detail: None, important: true },
        OotItem { name: "Bottle", _short: "Fairy", icon: "icons/oot/bottle.png", detail: Some("Fairy"), important: true },
        OotItem { name: "Bottle", _short: "Milk", icon: "icons/oot/milk.png", detail: Some("Milk"), important: true },
        OotItem { name: "Bottle", _short: "Fire", icon: "icons/oot/bottle.png", detail: Some("Fire"), important: true },
        OotItem { name: "Bottle", _short: "Poe", icon: "icons/oot/poe.png", detail: Some("Poe"), important: true },
        OotItem { name: "Claim Check", _short: "Check", icon: "icons/oot/claim_check.png", detail: None, important: false },
        OotItem { name: "Mask", _short: "Mask", icon: "icons/oot/mask.png", detail: None, important: false },
    ]
}
#[derive(Clone, Copy)]
struct EquipChoice { name: &'static str, _short: &'static str, icon: &'static str }
#[derive(Clone, Copy)]
struct EquipSlot { name: &'static str, choices: [EquipChoice; 3] }
fn equip_slots() -> [EquipSlot; 4] {
    [
        EquipSlot { name: "Sword", choices: [
            EquipChoice { name: "Kokiri Sword", _short: "Kok", icon: "icons/oot/kokiri_sword.png" },
            EquipChoice { name: "Master Sword", _short: "Mas", icon: "icons/oot/master_sword.png" },
            EquipChoice { name: "Biggoron Sword", _short: "Big", icon: "icons/oot/biggoron_sword.png" },
        ]},
        EquipSlot { name: "Shield", choices: [
            EquipChoice { name: "Deku Shield", _short: "Deku", icon: "icons/oot/deku_shield.png" },
            EquipChoice { name: "Hylian Shield", _short: "Hyl", icon: "icons/oot/hylian_shield.png" },
            EquipChoice { name: "Mirror Shield", _short: "Mir", icon: "icons/oot/mirror_shield.png" },
        ]},
        EquipSlot { name: "Tunic", choices: [
            EquipChoice { name: "Kokiri Tunic", _short: "Kok", icon: "icons/oot/kokiri_tunic.png" },
            EquipChoice { name: "Goron Tunic", _short: "Gor", icon: "icons/oot/goron_tunic.png" },
            EquipChoice { name: "Zora Tunic", _short: "Zora", icon: "icons/oot/zora_tunic.png" },
        ]},
        EquipSlot { name: "Boots", choices: [
            EquipChoice { name: "Kokiri Boots", _short: "Kok", icon: "icons/oot/kokiri_boots.png" },
            EquipChoice { name: "Iron Boots", _short: "Iron", icon: "icons/oot/iron_boots.png" },
            EquipChoice { name: "Hover Boots", _short: "Hover", icon: "icons/oot/hover_boots.png" },
        ]},
    ]
}

#[derive(Clone, Copy)]
struct MapMarker { name: &'static str, short: &'static str, x: f32, y: f32 }
fn map_markers() -> [MapMarker; 8] {
    [
        MapMarker { name: "Kokiri Forest", short: "K", x: 63.0, y: 55.0 },
        MapMarker { name: "Lost Woods", short: "W", x: 57.0, y: 46.0 },
        MapMarker { name: "Market", short: "M", x: 50.0, y: 35.0 },
        MapMarker { name: "Death Mountain", short: "D", x: 59.0, y: 28.0 },
        MapMarker { name: "Zora Domain", short: "Z", x: 67.0, y: 42.0 },
        MapMarker { name: "Lake Hylia", short: "L", x: 40.0, y: 61.0 },
        MapMarker { name: "Gerudo Valley", short: "G", x: 28.0, y: 48.0 },
        MapMarker { name: "Lon Lon Ranch", short: "R", x: 47.0, y: 50.0 },
    ]
}

#[derive(Clone, Copy)]
struct QuestIcon { name: &'static str, _short: &'static str, icon: &'static str }
fn quest_icons() -> [QuestIcon; 6] {
    [
        QuestIcon { name: "Forest Medallion", _short: "Fo", icon: "icons/oot/med_forest.png" },
        QuestIcon { name: "Fire Medallion", _short: "Fi", icon: "icons/oot/med_fire.png" },
        QuestIcon { name: "Water Medallion", _short: "Wa", icon: "icons/oot/med_water.png" },
        QuestIcon { name: "Spirit Medallion", _short: "Sp", icon: "icons/oot/med_spirit.png" },
        QuestIcon { name: "Shadow Medallion", _short: "Sh", icon: "icons/oot/med_shadow.png" },
        QuestIcon { name: "Light Medallion", _short: "Li", icon: "icons/oot/med_light.png" },
    ]
}
fn stones() -> [QuestIcon; 3] {
    [
        QuestIcon { name: "Kokiri Emerald", _short: "Em", icon: "icons/oot/stone_emerald.png" },
        QuestIcon { name: "Goron Ruby", _short: "Ru", icon: "icons/oot/stone_ruby.png" },
        QuestIcon { name: "Zora Sapphire", _short: "Sa", icon: "icons/oot/stone_sapphire.png" },
    ]
}


fn all_quest_icons() -> Vec<QuestIcon> {
    let mut out = Vec::new();
    out.extend_from_slice(&quest_icons());
    out.extend_from_slice(&stones());
    out
}

#[derive(Clone, Copy)]
struct Song { name: &'static str, _short: &'static str, icon: &'static str, pattern: &'static str }
fn songs() -> [Song; 12] {
    [
        Song { name: "Minuet of Forest", _short: "Min", icon: "icons/oot/song_minuet.png", pattern: "A ↑ ← → ← →" },
        Song { name: "Bolero of Fire", _short: "Bol", icon: "icons/oot/song_bolero.png", pattern: "↓ A ↓ A → ↓ → ↓" },
        Song { name: "Serenade of Water", _short: "Ser", icon: "icons/oot/song_serenade.png", pattern: "A ↓ → → ←" },
        Song { name: "Requiem of Spirit", _short: "Req", icon: "icons/oot/song_requiem.png", pattern: "A ↓ A → ↓ A" },
        Song { name: "Nocturne of Shadow", _short: "Noc", icon: "icons/oot/song_nocturne.png", pattern: "← → → A ← → ↓" },
        Song { name: "Prelude of Light", _short: "Pre", icon: "icons/oot/song_prelude.png", pattern: "↑ → ↑ → ← ↑" },
        Song { name: "Zelda's Lullaby", _short: "Zel", icon: "icons/oot/song_lullaby.png", pattern: "← ↑ → ← ↑ →" },
        Song { name: "Epona's Song", _short: "Epo", icon: "icons/oot/song_epona.png", pattern: "↑ ← → ↑ ← →" },
        Song { name: "Saria's Song", _short: "Sar", icon: "icons/oot/song_saria.png", pattern: "↓ → ← ↓ → ←" },
        Song { name: "Sun's Song", _short: "Sun", icon: "icons/oot/song_sun.png", pattern: "→ ↓ ↑ → ↓ ↑" },
        Song { name: "Song of Time", _short: "Tim", icon: "icons/oot/song_time.png", pattern: "→ A ↓ → A ↓" },
        Song { name: "Song of Storms", _short: "Sto", icon: "icons/oot/song_storms.png", pattern: "A ↓ ↑ A ↓ ↑" },
    ]
}
