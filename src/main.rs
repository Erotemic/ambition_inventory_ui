use std::f32::consts::{FRAC_PI_2, PI};
use std::sync::Arc;

use bevy::anti_alias::fxaa::Fxaa;
use bevy::core_pipeline::oit::OrderIndependentTransparencySettings;
use bevy::input::mouse::MouseWheel;
use bevy::input::touch::{TouchInput, TouchPhase};
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, SystemCursorIcon};
use bevy_lunex::prelude::*;
use ambition_inventory_ui::{
    MenuColor, MenuControlKind, MenuNode, MenuPageModel, MenuRect, MenuShellEffect,
    MenuShellEffects, MenuShellPhase, MenuTextAlign, TouchActivationPolicy,
};

// OoT builds the pause page background as 3 columns x 5 rows of 80x32
// quads, scales the page by 0.78, and places each page at
// R_PAUSE_DEPTH_OFFSET / 100.0. Horizontally, that makes the page width
// almost exactly 2 * depth, so adjacent page walls meet at a visible cube
// edge instead of floating as separated panels. Keep that relationship here.
const PAGE_RADIUS: f32 = 2.85;
const PAGE_W: f32 = PAGE_RADIUS * 2.0;
const PAGE_H: f32 = PAGE_W * (160.0 / 240.0);
// The viewer is inside the menu volume, but slightly back from the center,
// closer to OoT's eye/opposite-page relationship. This makes the front page
// readable while the side page corners/edges remain barely visible.
const CAMERA_EYE: Vec3 = Vec3::new(0.0, 0.0, -1.35);
const CAMERA_LOOK: Vec3 = Vec3::new(0.0, 0.0, 0.0);
const ROTATE_SPEED: f32 = 5.2;
const OPEN_CLOSE_SPEED: f32 = 8.0;
const MIN_OPEN_SCALE: f32 = 0.64;
// OoT drives pause open/close by animating page pitch from about 1.6 rad
// (source stores 160.0 and divides by 100.0 in draw matrices) down to 0.
const OOT_PAGE_FOLD_RADIANS: f32 = 1.60;
const STATUS_VISIBLE_ROWS: usize = 5;
// The physical cube order stays Gear -> Pack -> Map -> Status, but the
// visible tab strip is reversed to match the inside-cube mental model:
// the right-hand tab corresponds to the right-hand wall being pulled in.
const TAB_PAGES: [Page; 4] = [Page::Status, Page::Map, Page::Pack, Page::Gear];
// The viewer sees the inside/back side of the Lunex page walls.
// Flip each page root in local X so text/layout read normally from inside.
const INSIDE_PAGE_X_FLIP: f32 = -1.0;
const FONT_FAMILY: &str = "DejaVu Sans";

// Lunex computes child plane positions from UiDepth along the page root's
// local facing direction. In the inside-cube view we are looking at the
// inward/back side of each page wall, so positive depths go away from the
// viewer. Use negative depth bands so text/actions sit closer to the camera
// than opaque panels instead of being hidden behind dark rectangles.
const DEPTH_BACKGROUND: f32 = -0.05;
const DEPTH_LARGE_PANEL: f32 = -0.18;
const DEPTH_CARD: f32 = -0.34;
const DEPTH_ACTION: f32 = -0.46;
const DEPTH_TEXT: f32 = -0.70;
const DEPTH_EDGE: f32 = -0.82;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Ambition Inventory UI Prototype - Lunex".to_string(),
                resolution: (1180, 760).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins((UiLunexPlugins, MeshPickingPlugin))
        .insert_resource(ClearColor(Color::srgb(0.018, 0.016, 0.024)))
        .insert_resource(LoadFonts {
            font_directories: vec![
                "assets/fonts".to_string(),
                "/usr/share/fonts".to_string(),
                "/usr/local/share/fonts".to_string(),
            ],
            ..Default::default()
        })
        .insert_resource(InventoryDemo::default())
        .insert_resource(MenuAnimation::default())
        .insert_resource(MenuShell::default())
        .insert_resource(MenuShellEffects::default())
        .add_systems(Startup, setup)
        .add_systems(Update, menu_toggle_input)
        .add_systems(Update, (keyboard_navigation, mouse_navigation, pointer_hit_test, gamepad_navigation))
        .add_systems(Update, (animate_menu_ring, rebuild_lunex_faces))
        .run();
}

#[derive(Resource, Clone, Debug)]
struct InventoryDemo {
    page: Page,
    focus_area: FocusArea,
    selected_slot: usize,
    selected_item: usize,
    selected_action: usize,
    selected_pack: usize,
    selected_map: usize,
    selected_status: usize,
    selected_tab: usize,
    input_hints_enabled: bool,
    compact_layout: bool,
    detail_level: DetailLevel,
    touch_select_then_tap: bool,
    menu_toggle_binding: MenuToggleBinding,
    open_style: OpenCloseStyle,
    status_scroll: usize,
    iron_boots_equipped: bool,
    iron_boots_active: bool,
    status: String,
    revision: u64,
}

impl Default for InventoryDemo {
    fn default() -> Self {
        Self {
            page: Page::Gear,
            focus_area: FocusArea::Slots,
            selected_slot: 1,
            selected_item: 0,
            selected_action: 0,
            selected_pack: 0,
            selected_map: 0,
            selected_status: 0,
            selected_tab: InventoryDemo::tab_index_for_page(Page::Gear),
            input_hints_enabled: true,
            compact_layout: false,
            detail_level: DetailLevel::Normal,
            touch_select_then_tap: true,
            menu_toggle_binding: MenuToggleBinding::EscapeOrStart,
            open_style: OpenCloseStyle::OotPageFold,
            status_scroll: 0,
            iron_boots_equipped: false,
            iron_boots_active: false,
            status: "Select Feet, then equip Iron Boots.".to_string(),
            revision: 0,
        }
    }
}

impl InventoryDemo {
    fn pages() -> [Page; 4] {
        [Page::Gear, Page::Pack, Page::Map, Page::Status]
    }

    fn tab_pages() -> [Page; 4] {
        TAB_PAGES
    }

    fn tab_index_for_page(page: Page) -> usize {
        TAB_PAGES.iter().position(|p| *p == page).unwrap_or(0)
    }

    fn slots(&self) -> [&'static str; 3] {
        ["Weapon", "Feet", "Charm"]
    }

    fn slot_value(&self, idx: usize) -> String {
        match idx {
            0 => "Travel Sword".to_string(),
            1 if self.iron_boots_equipped => {
                if self.iron_boots_active {
                    "Iron Boots  [active]".to_string()
                } else {
                    "Iron Boots".to_string()
                }
            }
            1 => "Empty".to_string(),
            2 => "Empty".to_string(),
            _ => "".to_string(),
        }
    }

    fn items(&self) -> [&'static str; 3] {
        ["Iron Boots", "Feather Boots", "Climbing Spikes"]
    }

    fn actions(&self) -> [&'static str; 3] {
        if self.iron_boots_equipped {
            ["Unequip", "Toggle active", "Inspect"]
        } else {
            ["Equip to Feet", "Assign", "Inspect"]
        }
    }

    fn previous_page(&mut self) {
        let next = Page::from_index(self.page.index() - 1);
        self.goto_page(next);
    }

    fn next_page(&mut self) {
        let next = Page::from_index(self.page.index() + 1);
        self.goto_page(next);
    }

    fn goto_page(&mut self, page: Page) {
        if self.page != page {
            self.page = page;
            self.selected_tab = InventoryDemo::tab_index_for_page(page);
            self.status = format!("{} page selected.", page.label());
            self.bump();
        } else {
            self.selected_tab = InventoryDemo::tab_index_for_page(page);
        }
    }

    fn previous_focus_area(&mut self) {
        self.focus_area = if self.page == Page::Gear {
            self.focus_area.previous()
        } else if self.focus_area == FocusArea::Tabs {
            FocusArea::Items
        } else {
            FocusArea::Tabs
        };
        self.bump();
    }

    fn next_focus_area(&mut self) {
        self.focus_area = if self.page == Page::Gear {
            self.focus_area.next()
        } else if self.focus_area == FocusArea::Tabs {
            FocusArea::Items
        } else {
            FocusArea::Tabs
        };
        self.bump();
    }

    fn selected_tab_page(&self) -> Page {
        TAB_PAGES[self.selected_tab.min(TAB_PAGES.len().saturating_sub(1))]
    }

    fn active_content_area(&self) -> FocusArea {
        match self.page {
            Page::Gear => FocusArea::Slots,
            Page::Pack | Page::Map | Page::Status => FocusArea::Items,
        }
    }

    fn move_focus_vertical(&mut self, delta: i32) {
        if self.focus_area == FocusArea::Tabs {
            if delta > 0 {
                self.focus_area = self.active_content_area();
            }
            self.bump();
            return;
        }

        match self.page {
            Page::Gear => match self.focus_area {
                FocusArea::Tabs => unreachable!(),
                FocusArea::Slots => {
                    if delta < 0 && self.selected_slot == 0 {
                        self.focus_area = FocusArea::Tabs;
                    } else {
                        self.selected_slot = wrap_index(self.selected_slot, self.slots().len(), delta);
                    }
                }
                FocusArea::Items => {
                    if delta < 0 && self.selected_item == 0 {
                        self.focus_area = FocusArea::Tabs;
                    } else {
                        self.selected_item = wrap_index(self.selected_item, self.items().len(), delta);
                    }
                }
                FocusArea::Actions => {
                    if delta < 0 && self.selected_action == 0 {
                        self.focus_area = FocusArea::Tabs;
                    } else {
                        self.selected_action = wrap_index(self.selected_action, self.actions().len(), delta);
                    }
                }
            },
            Page::Pack => {
                if delta < 0 && self.selected_pack < 2 {
                    self.focus_area = FocusArea::Tabs;
                } else {
                    self.focus_area = FocusArea::Items;
                    self.selected_pack = move_pack_index(self.selected_pack, 0, delta);
                }
            }
            Page::Map => {
                if delta < 0 && self.selected_map == 0 {
                    self.focus_area = FocusArea::Tabs;
                } else {
                    self.focus_area = FocusArea::Items;
                    self.selected_map = wrap_index(self.selected_map, map_marker_count(), delta);
                }
            }
            Page::Status => {
                if delta < 0 && self.selected_status == 0 {
                    self.focus_area = FocusArea::Tabs;
                } else {
                    self.focus_area = FocusArea::Items;
                    self.selected_status = clamp_index_delta(self.selected_status, status_row_count(), delta);
                    self.ensure_status_visible();
                }
            }
        }
        self.bump();
    }

    fn move_focus_horizontal(&mut self, delta: i32) {
        if self.focus_area == FocusArea::Tabs {
            self.selected_tab = wrap_index(self.selected_tab, InventoryDemo::tab_pages().len(), delta);
            self.bump();
            return;
        }

        match self.page {
            Page::Gear => {
                match (self.focus_area, delta.signum()) {
                    (FocusArea::Tabs, _) => unreachable!(),
                    (FocusArea::Slots, 1) => {
                        self.focus_area = FocusArea::Items;
                        self.selected_item = self.selected_slot.min(self.items().len().saturating_sub(1));
                    }
                    (FocusArea::Items, 1) => {
                        self.focus_area = FocusArea::Actions;
                        self.selected_action = self.selected_item.min(self.actions().len().saturating_sub(1));
                    }
                    (FocusArea::Items, -1) => {
                        self.focus_area = FocusArea::Slots;
                        self.selected_slot = self.selected_item.min(self.slots().len().saturating_sub(1));
                    }
                    (FocusArea::Actions, -1) => {
                        self.focus_area = FocusArea::Items;
                        self.selected_item = self.selected_action.min(self.items().len().saturating_sub(1));
                    }
                    _ => {}
                }
            }
            Page::Pack => {
                self.focus_area = FocusArea::Items;
                self.selected_pack = move_pack_index(self.selected_pack, delta, 0);
            }
            Page::Map => {
                self.focus_area = FocusArea::Items;
                self.selected_map = wrap_index(self.selected_map, map_marker_count(), delta);
            }
            Page::Status => {
                self.focus_area = FocusArea::Items;
                self.selected_status = clamp_index_delta(self.selected_status, status_row_count(), delta);
                self.ensure_status_visible();
            }
        }
        self.bump();
    }

    fn back(&mut self) {
        self.focus_area = if self.page == Page::Gear {
            match self.focus_area {
                FocusArea::Actions => FocusArea::Items,
                FocusArea::Items => FocusArea::Slots,
                FocusArea::Slots => FocusArea::Tabs,
                FocusArea::Tabs => FocusArea::Tabs,
            }
        } else {
            FocusArea::Tabs
        };
        self.status = "Back.".to_string();
        self.bump();
    }

    fn activate_focused(&mut self) {
        if self.focus_area == FocusArea::Tabs {
            self.goto_page(self.selected_tab_page());
            return;
        }

        match self.page {
            Page::Gear => match self.focus_area {
                FocusArea::Tabs => unreachable!(),
                FocusArea::Slots => {
                    self.focus_area = FocusArea::Items;
                    self.status = format!("Choose gear for {} slot.", self.slots()[self.selected_slot]);
                }
                FocusArea::Items => {
                    self.focus_area = FocusArea::Actions;
                    self.status = format!("{} selected.", self.items()[self.selected_item]);
                }
                FocusArea::Actions => self.activate_action(self.selected_action),
            },
            Page::Pack => {
                self.status = pack_status(self.selected_pack).to_string();
            }
            Page::Map => {
                self.status = map_status(self.selected_map).to_string();
            }
            Page::Status => {
                self.activate_status_row(self.selected_status);
            }
        }
        self.bump();
    }

    fn activate_status_row(&mut self, idx: usize) {
        match idx {
            2 => self.toggle_input_hints(),
            3 => self.toggle_compact_layout(),
            4 => self.cycle_detail_level(),
            5 => self.toggle_touch_mode(),
            6 => self.cycle_menu_toggle_binding(),
            7 => self.cycle_open_style(),
            _ => {
                self.status = status_row_message(idx, self);
            }
        }
    }

    fn toggle_input_hints(&mut self) {
        self.input_hints_enabled = !self.input_hints_enabled;
        self.status = if self.input_hints_enabled {
            "Input hints enabled.".to_string()
        } else {
            "Input hints hidden.".to_string()
        };
        self.bump();
    }

    fn toggle_compact_layout(&mut self) {
        self.compact_layout = !self.compact_layout;
        self.status = if self.compact_layout {
            "Compact layout selected.".to_string()
        } else {
            "Cozy layout selected.".to_string()
        };
        self.bump();
    }

    fn cycle_detail_level(&mut self) {
        self.detail_level = self.detail_level.next();
        self.status = format!("Detail level: {}.", self.detail_level.label());
        self.bump();
    }

    fn toggle_touch_mode(&mut self) {
        self.touch_select_then_tap = !self.touch_select_then_tap;
        self.status = if self.touch_select_then_tap {
            "Touch mode: select first, tap again to activate.".to_string()
        } else {
            "Touch mode: activate immediately.".to_string()
        };
        self.bump();
    }

    fn cycle_menu_toggle_binding(&mut self) {
        self.menu_toggle_binding = self.menu_toggle_binding.next();
        self.status = format!("Menu toggle input: {}.", self.menu_toggle_binding.label());
        self.bump();
    }

    fn cycle_open_style(&mut self) {
        self.open_style = self.open_style.next();
        self.status = format!("Open/close style: {}.", self.open_style.label());
        self.bump();
    }

    fn scroll_status(&mut self, delta: i32) {
        let count = status_row_count();
        let max_start = count.saturating_sub(STATUS_VISIBLE_ROWS);
        self.status_scroll = clamp_index_delta(self.status_scroll, max_start + 1, delta);
        self.selected_status = self.selected_status.clamp(self.status_scroll, (self.status_scroll + STATUS_VISIBLE_ROWS - 1).min(count.saturating_sub(1)));
        self.focus_area = FocusArea::Items;
        self.bump();
    }

    fn ensure_status_visible(&mut self) {
        let count = status_row_count();
        let max_start = count.saturating_sub(STATUS_VISIBLE_ROWS);
        if self.selected_status < self.status_scroll {
            self.status_scroll = self.selected_status;
        } else if self.selected_status >= self.status_scroll + STATUS_VISIBLE_ROWS {
            self.status_scroll = self.selected_status + 1 - STATUS_VISIBLE_ROWS;
        }
        self.status_scroll = self.status_scroll.min(max_start);
    }

    fn activate_action(&mut self, action_idx: usize) {
        match action_idx {
            0 => {
                if self.iron_boots_equipped {
                    self.iron_boots_equipped = false;
                    self.iron_boots_active = false;
                    self.status = "Iron Boots removed from Feet.".to_string();
                } else {
                    self.selected_slot = 1;
                    self.selected_item = 0;
                    self.iron_boots_equipped = true;
                    self.status = "Iron Boots equipped to Feet.".to_string();
                }
            }
            1 => self.try_toggle_iron_boots(),
            _ => {
                self.status = "Iron Boots: heavy footing, current resistance, reduced speed.".to_string();
            }
        }
    }

    fn try_toggle_iron_boots(&mut self) {
        if self.iron_boots_equipped {
            self.iron_boots_active = !self.iron_boots_active;
            self.status = if self.iron_boots_active {
                "Iron Boots active: anchored and heavy.".to_string()
            } else {
                "Iron Boots inactive: normal movement restored.".to_string()
            };
        } else {
            self.status = "Equip Iron Boots before toggling them.".to_string();
        }
        self.bump();
    }

    fn hover(&mut self, action: ClickAction) {
        let before = (self.page, self.focus_area, self.selected_tab, self.selected_slot, self.selected_item, self.selected_action, self.selected_pack, self.selected_map, self.selected_status);
        match action {
            ClickAction::Goto(page) => {
                self.focus_area = FocusArea::Tabs;
                self.selected_tab = InventoryDemo::tab_index_for_page(page);
            }
            ClickAction::FocusArea(area) => {
                self.focus_area = area;
            }
            ClickAction::SelectSlot(idx) => {
                self.page = Page::Gear;
                self.focus_area = FocusArea::Slots;
                self.selected_slot = idx;
            }
            ClickAction::SelectItem(idx) => {
                self.page = Page::Gear;
                self.focus_area = FocusArea::Items;
                self.selected_item = idx;
            }
            ClickAction::Action(idx) => {
                self.page = Page::Gear;
                self.focus_area = FocusArea::Actions;
                self.selected_action = idx;
            }
            ClickAction::PackItem(idx) => {
                self.page = Page::Pack;
                self.selected_pack = idx;
            }
            ClickAction::MapMarker(idx) => {
                self.page = Page::Map;
                self.selected_map = idx;
            }
            ClickAction::StatusRow(idx) => {
                self.page = Page::Status;
                self.focus_area = FocusArea::Items;
                self.selected_status = idx;
            }
            ClickAction::ToggleInputHints
            | ClickAction::ToggleCompactLayout
            | ClickAction::CycleDetailLevel
            | ClickAction::ToggleTouchMode
            | ClickAction::CycleOpenStyle => {
                self.page = Page::Status;
                self.focus_area = FocusArea::Items;
            }
        }
        let after = (self.page, self.focus_area, self.selected_tab, self.selected_slot, self.selected_item, self.selected_action, self.selected_pack, self.selected_map, self.selected_status);
        if before != after {
            self.bump();
        }
    }

    fn click(&mut self, action: ClickAction) {
        match action {
            ClickAction::Goto(page) => {
                self.focus_area = FocusArea::Tabs;
                self.selected_tab = InventoryDemo::tab_index_for_page(page);
                self.goto_page(page);
            }
            ClickAction::FocusArea(area) => {
                self.focus_area = area;
                self.bump();
            }
            ClickAction::SelectSlot(idx) => {
                self.page = Page::Gear;
                self.focus_area = FocusArea::Slots;
                self.selected_slot = idx;
                self.status = format!("{} slot selected.", self.slots()[idx]);
                self.bump();
            }
            ClickAction::SelectItem(idx) => {
                self.page = Page::Gear;
                self.focus_area = FocusArea::Items;
                self.selected_item = idx;
                self.status = format!("{} selected.", self.items()[idx]);
                self.bump();
            }
            ClickAction::Action(idx) => {
                self.page = Page::Gear;
                self.focus_area = FocusArea::Actions;
                self.selected_action = idx;
                self.activate_action(idx);
                self.bump();
            }
            ClickAction::PackItem(idx) => {
                self.page = Page::Pack;
                self.selected_pack = idx;
                self.status = pack_status(idx).to_string();
                self.bump();
            }
            ClickAction::MapMarker(idx) => {
                self.page = Page::Map;
                self.selected_map = idx;
                self.status = map_status(idx).to_string();
                self.bump();
            }
            ClickAction::StatusRow(idx) => {
                self.page = Page::Status;
                self.focus_area = FocusArea::Items;
                self.selected_status = idx;
                self.activate_status_row(idx);
                self.bump();
            }
            ClickAction::ToggleInputHints => self.toggle_input_hints(),
            ClickAction::ToggleCompactLayout => self.toggle_compact_layout(),
            ClickAction::CycleDetailLevel => self.cycle_detail_level(),
            ClickAction::ToggleTouchMode => self.toggle_touch_mode(),
            ClickAction::CycleOpenStyle => self.cycle_open_style(),
        }
    }

    fn bump(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }
}

#[derive(Resource, Clone, Debug)]
struct MenuAnimation {
    current_angle: f32,
    target_angle: f32,
}

impl Default for MenuAnimation {
    fn default() -> Self {
        Self {
            current_angle: 0.0,
            target_angle: 0.0,
        }
    }
}

impl MenuAnimation {
    fn set_page(&mut self, page: Page) {
        self.target_angle = -page.index() as f32 * FRAC_PI_2;
    }
}

/// Reusable menu-shell state for a game-friendly Lunex menu.
///
/// The demo owns the inventory data, but this shell owns lifecycle: opening,
/// open, closing, and closed. Host games should wire their own input into
/// `open`, `close`, or `toggle` rather than hard-coding Escape/Start here.
#[derive(Resource, Clone, Debug)]
struct MenuShell {
    openness: f32,
    target_open: bool,
}

impl Default for MenuShell {
    fn default() -> Self {
        Self {
            openness: 1.0,
            target_open: true,
        }
    }
}

impl MenuShell {
    fn open(&mut self) {
        self.target_open = true;
    }

    fn close(&mut self) {
        self.target_open = false;
    }

    fn toggle(&mut self) {
        self.target_open = !self.target_open;
    }

    fn is_interactive(&self) -> bool {
        self.target_open && self.openness > 0.96
    }

    fn is_visible(&self) -> bool {
        self.target_open || self.openness > 0.02
    }

    fn phase(&self) -> MenuShellPhase {
        match (self.target_open, self.openness) {
            (false, open) if open <= 0.02 => MenuShellPhase::Closed,
            (true, open) if open >= 0.96 => MenuShellPhase::Open,
            (true, _) => MenuShellPhase::Opening,
            (false, _) => MenuShellPhase::Closing,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MenuToggleBinding {
    EscapeOrStart,
    POrStart,
}

impl MenuToggleBinding {
    fn label(self) -> &'static str {
        match self {
            MenuToggleBinding::EscapeOrStart => "Escape / Start",
            MenuToggleBinding::POrStart => "P / Start",
        }
    }

    fn next(self) -> Self {
        match self {
            MenuToggleBinding::EscapeOrStart => MenuToggleBinding::POrStart,
            MenuToggleBinding::POrStart => MenuToggleBinding::EscapeOrStart,
        }
    }

    fn keyboard_pressed(self, keys: &ButtonInput<KeyCode>) -> bool {
        match self {
            MenuToggleBinding::EscapeOrStart => keys.just_pressed(KeyCode::Escape),
            MenuToggleBinding::POrStart => keys.just_pressed(KeyCode::KeyP),
        }
    }

    fn gamepad_pressed(self, gamepad: &Gamepad) -> bool {
        let _ = self;
        gamepad.just_pressed(GamepadButton::Start)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OpenCloseStyle {
    SmoothScale,
    OotPageFold,
}

impl OpenCloseStyle {
    fn label(self) -> &'static str {
        match self {
            OpenCloseStyle::SmoothScale => "Smooth scale",
            OpenCloseStyle::OotPageFold => "OoT page fold",
        }
    }

    fn next(self) -> Self {
        match self {
            OpenCloseStyle::SmoothScale => OpenCloseStyle::OotPageFold,
            OpenCloseStyle::OotPageFold => OpenCloseStyle::SmoothScale,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Page {
    Gear,
    Pack,
    Map,
    Status,
}

impl Page {
    fn index(self) -> i32 {
        match self {
            Page::Gear => 0,
            Page::Pack => 1,
            Page::Map => 2,
            Page::Status => 3,
        }
    }

    fn from_index(idx: i32) -> Self {
        match idx.rem_euclid(4) {
            0 => Page::Gear,
            1 => Page::Pack,
            2 => Page::Map,
            _ => Page::Status,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Page::Gear => "Gear",
            Page::Pack => "Pack",
            Page::Map => "Map",
            Page::Status => "Status",
        }
    }

    fn face_color(self) -> Color {
        match self {
            Page::Gear => Color::srgb(0.070, 0.060, 0.092),
            Page::Pack => Color::srgb(0.060, 0.070, 0.085),
            Page::Map => Color::srgb(0.055, 0.075, 0.066),
            Page::Status => Color::srgb(0.075, 0.060, 0.064),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FocusArea {
    Tabs,
    Slots,
    Items,
    Actions,
}

impl FocusArea {
    fn previous(self) -> Self {
        match self {
            FocusArea::Tabs => FocusArea::Actions,
            FocusArea::Slots => FocusArea::Tabs,
            FocusArea::Items => FocusArea::Slots,
            FocusArea::Actions => FocusArea::Items,
        }
    }

    fn next(self) -> Self {
        match self {
            FocusArea::Tabs => FocusArea::Slots,
            FocusArea::Slots => FocusArea::Items,
            FocusArea::Items => FocusArea::Actions,
            FocusArea::Actions => FocusArea::Tabs,
        }
    }

    fn label(self) -> &'static str {
        match self {
            FocusArea::Tabs => "Menu",
            FocusArea::Slots => "Slots",
            FocusArea::Items => "Items",
            FocusArea::Actions => "Actions",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DetailLevel {
    Minimal,
    Normal,
    Verbose,
}

impl DetailLevel {
    fn label(self) -> &'static str {
        match self {
            DetailLevel::Minimal => "Minimal",
            DetailLevel::Normal => "Normal",
            DetailLevel::Verbose => "Verbose",
        }
    }

    fn next(self) -> Self {
        match self {
            DetailLevel::Minimal => DetailLevel::Normal,
            DetailLevel::Normal => DetailLevel::Verbose,
            DetailLevel::Verbose => DetailLevel::Minimal,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ClickAction {
    Goto(Page),
    FocusArea(FocusArea),
    SelectSlot(usize),
    SelectItem(usize),
    Action(usize),
    PackItem(usize),
    MapMarker(usize),
    StatusRow(usize),
    ToggleInputHints,
    ToggleCompactLayout,
    CycleDetailLevel,
    ToggleTouchMode,
    CycleOpenStyle,
}

#[derive(Component)]
struct MenuRing;

#[derive(Component)]
struct LunexFaceRoot;

#[derive(Component)]
struct PageFace(Page);

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    demo: Res<InventoryDemo>,
) {
    commands.spawn((
        DirectionalLight {
            illuminance: 2800.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(1.5, 3.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Camera3d::default(),
        OrderIndependentTransparencySettings::default(),
        Msaa::Off,
        Fxaa::default(),
        Transform::from_translation(CAMERA_EYE).looking_at(CAMERA_LOOK, Vec3::Y),
    ));

    let ring = commands
        .spawn((
            Name::new("Inside-view Lunex menu room"),
            MenuRing,
            UiRoot3d,
            Transform::default(),
            Visibility::Visible,
        ))
        .id();

    commands.entity(ring).with_children(|ring| {
        spawn_all_faces(ring, &demo, &mut materials);
    });
}

fn rebuild_lunex_faces(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    demo: Res<InventoryDemo>,
    ring_query: Query<Entity, With<MenuRing>>,
    face_query: Query<(Entity, &PageFace), With<LunexFaceRoot>>,
    mut last_revision: Local<Option<u64>>,
    mut last_page: Local<Option<Page>>,
) {
    if *last_revision == Some(demo.revision) {
        return;
    }

    let Ok(ring) = ring_query.single() else {
        return;
    };

    let page_changed = last_page.map(|page| page != demo.page).unwrap_or(false);

    if page_changed {
        // Page changes affect which face is pickable and which tab is active.
        // Rebuild all faces, but keep focus-only changes cheaper below.
        for (entity, _) in &face_query {
            commands.entity(entity).despawn();
        }
        commands.entity(ring).with_children(|ring| {
            spawn_all_faces(ring, &demo, &mut materials);
        });
    } else {
        // Most arrow/D-pad movement only changes highlights on the active face.
        // Rebuilding just that face avoids recreating all four Lunex page trees
        // and cuts the most obvious UI churn without a large architecture rewrite.
        for (entity, face) in &face_query {
            if face.0 == demo.page {
                commands.entity(entity).despawn();
            }
        }
        commands.entity(ring).with_children(|ring| {
            spawn_face(ring, demo.page, &demo, &mut materials);
        });
    }

    *last_revision = Some(demo.revision);
    *last_page = Some(demo.page);
}

fn spawn_all_faces(
    ring: &mut ChildSpawnerCommands,
    demo: &InventoryDemo,
    materials: &mut Assets<StandardMaterial>,
) {
    for page in InventoryDemo::pages() {
        spawn_face(ring, page, demo, materials);
    }
}

fn spawn_face(
    ring: &mut ChildSpawnerCommands,
    page: Page,
    demo: &InventoryDemo,
    materials: &mut Assets<StandardMaterial>,
) {
    let (translation, rotation) = page_face_transform(page);
    ring.spawn((
        Name::new(format!("{} Lunex face", page.label())),
        LunexFaceRoot,
        PageFace(page),
        UiRoot3d,
        UiLayoutRoot::new_3d(),
        Dimension::from((PAGE_W, PAGE_H)),
        Transform::from_translation(translation)
            .with_rotation(rotation)
            .with_scale(Vec3::new(INSIDE_PAGE_X_FLIP, 1.0, 1.0)),
    ))
    .with_children(|ui| {
        let active_face = page == demo.page;
        let model = build_page_model(page, demo, active_face);
        render_page_model(ui, materials, &model);
    });
}

fn page_face_transform(page: Page) -> (Vec3, Quat) {
    // Inside-cube model: page centers sit one radius from the origin and the
    // page width is exactly 2 * radius, so neighboring walls share their
    // vertical edges. The root X flip applied in spawn_face corrects the
    // backface mirror so text reads normally from inside.
    match page {
        Page::Gear => (Vec3::new(0.0, 0.0, PAGE_RADIUS), Quat::IDENTITY),
        Page::Pack => (Vec3::new(PAGE_RADIUS, 0.0, 0.0), Quat::from_rotation_y(FRAC_PI_2)),
        Page::Map => (Vec3::new(0.0, 0.0, -PAGE_RADIUS), Quat::from_rotation_y(PI)),
        Page::Status => (Vec3::new(-PAGE_RADIUS, 0.0, 0.0), Quat::from_rotation_y(-FRAC_PI_2)),
    }
}

fn reset_face_transform(page: Page, transform: &mut Transform) {
    let (translation, rotation) = page_face_transform(page);
    transform.translation = translation;
    transform.rotation = rotation;
    transform.scale = Vec3::new(INSIDE_PAGE_X_FLIP, 1.0, 1.0);
}

fn apply_oot_open_fold(page: Page, fold: f32, transform: &mut Transform) {
    let (base_translation, base_rotation) = page_face_transform(page);
    // OoT's pause pages are not center-pivot cards. During open/close,
    // pagesYOrigin1/R_PAUSE_PAGES_Y_ORIGIN_2 shift the page vertices so each
    // page rotates around its lower edge. Approximate that exactly here by
    // keeping the local bottom-center hinge fixed while applying the same
    // side-specific fold axes/signs from z_kaleido_scope.c:
    //   item/front  : RotateX(-pitch)
    //   quest/back  : RotateX(+pitch)
    //   equip/left  : RotateZ(+pitch)
    //   map/right   : RotateZ(-pitch)
    // With our inside-cube page mapping Gear(+Z), Pack(+X), Map(-Z),
    // Status(-X), that becomes the signs below. The fold is applied before the
    // wall-facing rotation, which prevents side pages from reading as 2D
    // clockwise/counter-clockwise spins.
    let fold_rotation = match page {
        Page::Gear => Quat::from_rotation_x(fold),
        Page::Map => Quat::from_rotation_x(-fold),
        Page::Pack => Quat::from_rotation_z(-fold),
        Page::Status => Quat::from_rotation_z(fold),
    };
    let rotation = fold_rotation * base_rotation;
    let hinge_local = Vec3::new(0.0, -PAGE_H * 0.5, 0.0);
    let hinge_world = base_translation + base_rotation * hinge_local;
    let translation = hinge_world - rotation * hinge_local;

    transform.translation = translation;
    transform.rotation = rotation;
    transform.scale = Vec3::new(INSIDE_PAGE_X_FLIP, 1.0, 1.0);
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

fn render_page_model(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    model: &MenuPageModel<Page, ClickAction>,
) {
    spawn_panel(ui, materials, 0.0, 0.0, 100.0, 100.0, menu_color(model.background), None);
    spawn_cube_edge_frame(ui, materials);

    for node in &model.nodes {
        match node {
            MenuNode::Panel { rect, color, action } => {
                spawn_panel(ui, materials, rect.x, rect.y, rect.w, rect.h, menu_color(*color), *action);
            }
            MenuNode::Text { x, y, size, text, align, color } => {
                spawn_text(ui, materials, *x, *y, *size, text, menu_align(*align), menu_srgba(*color));
            }
            MenuNode::Control { rect, kind, label, detail, selected, important, action } => {
                let color = control_color(*kind, *selected, *important);
                spawn_panel(ui, materials, rect.x, rect.y, rect.w, rect.h, color, *action);
                if let Some(detail) = detail {
                    spawn_text(
                        ui,
                        materials,
                        rect.x + rect.w * 0.5,
                        rect.y + rect.h * 0.30,
                        control_label_size(*kind),
                        label,
                        TextAlign::Center,
                        control_label_color(*kind, *selected, *important),
                    );
                    spawn_text(
                        ui,
                        materials,
                        rect.x + rect.w * 0.5,
                        rect.y + rect.h * 0.68,
                        control_detail_size(*kind),
                        detail,
                        TextAlign::Center,
                        Srgba::rgb_u8(172, 190, 204),
                    );
                } else {
                    spawn_text(
                        ui,
                        materials,
                        rect.x + rect.w * 0.5,
                        rect.y + rect.h * 0.52,
                        control_label_size(*kind),
                        label,
                        TextAlign::Center,
                        control_label_color(*kind, *selected, *important),
                    );
                }
            }
        }
    }
}

fn control_color(kind: MenuControlKind, selected: bool, important: bool) -> Color {
    match kind {
        MenuControlKind::Tab => {
            if selected {
                Color::srgba(0.86, 0.68, 0.30, 0.98)
            } else if important {
                Color::srgba(0.75, 0.57, 0.23, 0.95)
            } else {
                Color::srgba(0.22, 0.20, 0.25, 0.85)
            }
        }
        MenuControlKind::Action => focus_color(selected, important),
        MenuControlKind::OptionToggle | MenuControlKind::OptionChoice => {
            if selected { Color::srgba(0.55, 0.50, 0.68, 0.94) } else { Color::srgba(0.13, 0.10, 0.12, 0.90) }
        }
        MenuControlKind::MapMarker => {
            if selected { Color::srgba(0.82, 0.58, 0.24, 0.96) } else { Color::srgba(0.18, 0.24, 0.18, 0.95) }
        }
        _ => focus_color(selected, important),
    }
}

fn control_label_size(kind: MenuControlKind) -> f32 {
    match kind {
        MenuControlKind::Tab => 2.7,
        MenuControlKind::MapMarker => 2.0,
        MenuControlKind::OptionToggle | MenuControlKind::OptionChoice => 2.35,
        MenuControlKind::Action => 2.8,
        _ => 2.75,
    }
}

fn control_detail_size(kind: MenuControlKind) -> f32 {
    match kind {
        MenuControlKind::OptionToggle | MenuControlKind::OptionChoice => 2.35,
        _ => 2.0,
    }
}

fn control_label_color(kind: MenuControlKind, _selected: bool, important: bool) -> Srgba {
    match kind {
        MenuControlKind::Tab if important => Srgba::rgb_u8(35, 28, 21),
        MenuControlKind::OptionToggle | MenuControlKind::OptionChoice => Srgba::rgb_u8(232, 228, 222),
        _ => Srgba::rgb_u8(238, 229, 202),
    }
}

fn build_page_model(page: Page, demo: &InventoryDemo, active_face: bool) -> MenuPageModel<Page, ClickAction> {
    let mut model = MenuPageModel::new(page, page.label(), mc(page.face_color()));
    model.panel(MenuRect::new(3.0, 4.0, 94.0, 12.0), mc(Color::srgba(0.16, 0.13, 0.20, 0.92)), None);
    model.text(50.0, 9.5, 7.2, page.label(), MenuTextAlign::Center, MenuColor::rgba(238.0 / 255.0, 222.0 / 255.0, 186.0 / 255.0, 1.0));

    add_page_tabs(&mut model, demo, active_face);
    match page {
        Page::Gear => add_gear_nodes(&mut model, demo, active_face),
        Page::Pack => add_pack_nodes(&mut model, demo, active_face),
        Page::Map => add_map_nodes(&mut model, demo, active_face),
        Page::Status => add_status_nodes(&mut model, demo, active_face),
    }

    model.panel(MenuRect::new(5.0, 88.0, 90.0, 7.5), mc(Color::srgba(0.02, 0.018, 0.025, 0.88)), None);
    model.text(50.0, 91.8, 3.4, demo.status.as_str(), MenuTextAlign::Center, MenuColor::rgba(198.0 / 255.0, 206.0 / 255.0, 218.0 / 255.0, 1.0));
    model
}

fn add_page_tabs(model: &mut MenuPageModel<Page, ClickAction>, demo: &InventoryDemo, active_face: bool) {
    for (i, page) in InventoryDemo::tab_pages().iter().enumerate() {
        let active = *page == demo.page;
        let selected = demo.focus_area == FocusArea::Tabs && demo.selected_tab == i;
        model.control(
            MenuRect::new(12.0 + i as f32 * 19.0, 18.0, 16.5, 6.5),
            MenuControlKind::Tab,
            page.label(),
            None,
            selected,
            active,
            active_face.then_some(ClickAction::Goto(*page)),
        );
    }
}

fn add_gear_nodes(model: &mut MenuPageModel<Page, ClickAction>, demo: &InventoryDemo, active_face: bool) {
    model.text(18.0, 31.5, 3.4, "Slots", MenuTextAlign::Center, MenuColor::rgba(235.0 / 255.0, 225.0 / 255.0, 200.0 / 255.0, 1.0));
    model.text(50.0, 31.5, 3.4, "Boots", MenuTextAlign::Center, MenuColor::rgba(235.0 / 255.0, 225.0 / 255.0, 200.0 / 255.0, 1.0));
    model.text(82.0, 31.5, 3.4, "Actions", MenuTextAlign::Center, MenuColor::rgba(235.0 / 255.0, 225.0 / 255.0, 200.0 / 255.0, 1.0));

    for (i, slot) in demo.slots().iter().enumerate() {
        let y = 37.0 + i as f32 * 12.0;
        let selected = demo.focus_area == FocusArea::Slots && demo.selected_slot == i;
        model.control(
            MenuRect::new(7.0, y, 23.0, 9.2),
            MenuControlKind::Slot,
            *slot,
            Some(demo.slot_value(i)),
            selected,
            i == 1,
            active_face.then_some(ClickAction::SelectSlot(i)),
        );
    }

    for (i, item) in demo.items().iter().enumerate() {
        let y = 37.0 + i as f32 * 12.0;
        let detail = match i {
            0 => "Heavy footing / current resist",
            1 => "Light steps / jump control",
            _ => "Wall contact / ledge grip",
        };
        model.control(
            MenuRect::new(36.5, y, 27.0, 9.2),
            MenuControlKind::Item,
            *item,
            Some(detail.to_string()),
            demo.focus_area == FocusArea::Items && demo.selected_item == i,
            i == 0,
            active_face.then_some(ClickAction::SelectItem(i)),
        );
    }

    for (i, action_label) in demo.actions().iter().enumerate() {
        let y = 37.0 + i as f32 * 12.0;
        model.control(
            MenuRect::new(70.5, y, 23.0, 9.2),
            MenuControlKind::Action,
            *action_label,
            None,
            demo.focus_area == FocusArea::Actions && demo.selected_action == i,
            i < 2,
            active_face.then_some(ClickAction::Action(i)),
        );
    }

    let boot_state = if demo.iron_boots_equipped {
        if demo.iron_boots_active { "Feet: Iron Boots are ACTIVE" } else { "Feet: Iron Boots equipped, inactive" }
    } else {
        "Feet: empty; Iron Boots available"
    };
    model.panel(
        MenuRect::new(14.0, 74.0, 72.0, 9.0),
        mc(Color::srgba(0.08, 0.09, 0.12, 0.84)),
        active_face.then_some(ClickAction::FocusArea(FocusArea::Actions)),
    );
    model.text(50.0, 78.6, 3.2, boot_state, MenuTextAlign::Center, MenuColor::rgba(221.0 / 255.0, 230.0 / 255.0, 236.0 / 255.0, 1.0));
}

fn add_pack_nodes(model: &mut MenuPageModel<Page, ClickAction>, demo: &InventoryDemo, active_face: bool) {
    model.text(50.0, 32.7, 3.2, "Pack separates quick consumables, key items, and trade goods.", MenuTextAlign::Center, MenuColor::rgba(224.0 / 255.0, 226.0 / 255.0, 215.0 / 255.0, 1.0));
    for (i, item) in pack_items().iter().enumerate() {
        let x = if i % 2 == 0 { 16.0 } else { 53.5 };
        let y = 41.0 + (i / 2) as f32 * 12.4;
        model.control(
            MenuRect::new(x, y, 31.0, 9.5),
            MenuControlKind::Item,
            item.0,
            Some(item.1.to_string()),
            demo.focus_area == FocusArea::Items && demo.selected_pack == i,
            item.2,
            active_face.then_some(ClickAction::PackItem(i)),
        );
    }
}

fn add_map_nodes(model: &mut MenuPageModel<Page, ClickAction>, demo: &InventoryDemo, active_face: bool) {
    model.text(50.0, 33.5, 3.4, "Map face: markers are controls, not a decorative image.", MenuTextAlign::Center, MenuColor::rgba(224.0 / 255.0, 232.0 / 255.0, 218.0 / 255.0, 1.0));
    model.panel(MenuRect::new(18.0, 41.0, 64.0, 31.0), mc(Color::srgba(0.08, 0.13, 0.105, 0.93)), None);
    for i in 0..5 {
        let y = 46.0 + i as f32 * 5.0;
        model.panel(MenuRect::new(24.0, y, 52.0 - i as f32 * 5.5, 1.2), mc(Color::srgba(0.38, 0.48, 0.38, 0.80)), None);
    }
    for (i, (label, x, y)) in map_markers().iter().enumerate() {
        model.control(
            MenuRect::new(*x, *y, 13.0, 6.0),
            MenuControlKind::MapMarker,
            *label,
            None,
            demo.focus_area == FocusArea::Items && demo.selected_map == i,
            false,
            active_face.then_some(ClickAction::MapMarker(i)),
        );
    }
    model.text(50.0, 78.0, 2.9, "Select markers with arrows/D-pad or pointer.", MenuTextAlign::Center, MenuColor::rgba(185.0 / 255.0, 204.0 / 255.0, 188.0 / 255.0, 1.0));
}

fn add_status_nodes(model: &mut MenuPageModel<Page, ClickAction>, demo: &InventoryDemo, active_face: bool) {
    model.text(50.0, 34.0, 3.4, "Character status / demo settings", MenuTextAlign::Center, MenuColor::rgba(235.0 / 255.0, 224.0 / 255.0, 220.0 / 255.0, 1.0));
    let rows = status_rows(demo);
    let max_start = rows.len().saturating_sub(STATUS_VISIBLE_ROWS);
    let start = demo.status_scroll.min(max_start);
    let end = (start + STATUS_VISIBLE_ROWS).min(rows.len());

    model.panel(MenuRect::new(17.0, 39.0, 66.0, 45.0), mc(Color::srgba(0.065, 0.050, 0.062, 0.94)), None);
    for (visible_idx, i) in (start..end).enumerate() {
        let (k, v, kind) = &rows[i];
        let y = 43.0 + visible_idx as f32 * 8.2;
        model.control(
            MenuRect::new(20.0, y, 58.0, 6.8),
            *kind,
            *k,
            Some(v.clone()),
            demo.focus_area == FocusArea::Items && demo.selected_status == i,
            matches!(kind, MenuControlKind::OptionToggle | MenuControlKind::OptionChoice),
            active_face.then_some(ClickAction::StatusRow(i)),
        );
    }

    if rows.len() > STATUS_VISIBLE_ROWS {
        let track_h = 38.0;
        let thumb_h = (STATUS_VISIBLE_ROWS as f32 / rows.len() as f32 * track_h).max(8.0);
        let max_scroll = rows.len() - STATUS_VISIBLE_ROWS;
        let thumb_y = 42.0 + (demo.status_scroll as f32 / max_scroll as f32) * (track_h - thumb_h);
        model.panel(MenuRect::new(80.3, 42.0, 1.2, track_h), mc(Color::srgba(0.10, 0.09, 0.11, 0.96)), None);
        model.panel(MenuRect::new(80.1, thumb_y, 1.6, thumb_h), mc(Color::srgba(0.70, 0.55, 0.26, 0.98)), None);
        model.text(50.0, 82.4, 2.0, "Scroll pane: wheel, touch, or D-pad follows selection.", MenuTextAlign::Center, MenuColor::rgba(188.0 / 255.0, 190.0 / 255.0, 205.0 / 255.0, 1.0));
    }
}

fn spawn_page_tabs(ui: &mut ChildSpawnerCommands, materials: &mut Assets<StandardMaterial>, demo: &InventoryDemo, active_face: bool) {
    for (i, page) in InventoryDemo::tab_pages().iter().enumerate() {
        let x = 12.0 + i as f32 * 19.0;
        let active = *page == demo.page;
        let selected_tab = demo.focus_area == FocusArea::Tabs && demo.selected_tab == i;
        let color = if selected_tab {
            Color::srgba(0.86, 0.68, 0.30, 0.98)
        } else if active {
            Color::srgba(0.75, 0.57, 0.23, 0.95)
        } else {
            Color::srgba(0.22, 0.20, 0.25, 0.85)
        };
        let action = active_face.then_some(ClickAction::Goto(*page));
        spawn_panel(ui, materials, x, 18.0, 16.5, 6.5, color, action);
        spawn_text(
            ui,
            materials,
            x + 8.25,
            21.3,
            2.7,
            page.label(),
            TextAlign::Center,
            if active { Srgba::rgb_u8(35, 28, 21) } else { Srgba::rgb_u8(214, 207, 190) },
        );
    }
}

fn spawn_gear_page(ui: &mut ChildSpawnerCommands, materials: &mut Assets<StandardMaterial>, demo: &InventoryDemo, active_face: bool) {
    spawn_text(ui, materials, 18.0, 31.5, 3.4, "Slots", TextAlign::Center, Srgba::rgb_u8(235, 225, 200));
    spawn_text(ui, materials, 50.0, 31.5, 3.4, "Boots", TextAlign::Center, Srgba::rgb_u8(235, 225, 200));
    spawn_text(ui, materials, 82.0, 31.5, 3.4, "Actions", TextAlign::Center, Srgba::rgb_u8(235, 225, 200));

    for (i, slot) in demo.slots().iter().enumerate() {
        let y = 37.0 + i as f32 * 12.0;
        let selected = demo.focus_area == FocusArea::Slots && demo.selected_slot == i;
        let color = focus_color(selected, i == 1);
        let action = active_face.then_some(ClickAction::SelectSlot(i));
        spawn_panel(ui, materials, 7.0, y, 23.0, 9.2, color, action);
        spawn_text(ui, materials, 18.5, y + 2.6, 2.7, slot, TextAlign::Center, Srgba::rgb_u8(238, 229, 202));
        let value = demo.slot_value(i);
        spawn_text(ui, materials, 18.5, y + 6.1, 2.1, &value, TextAlign::Center, Srgba::rgb_u8(183, 192, 205));
    }

    for (i, item) in demo.items().iter().enumerate() {
        let y = 37.0 + i as f32 * 12.0;
        let selected = demo.focus_area == FocusArea::Items && demo.selected_item == i;
        let is_iron = i == 0;
        let color = focus_color(selected, is_iron);
        let action = active_face.then_some(ClickAction::SelectItem(i));
        spawn_panel(ui, materials, 36.5, y, 27.0, 9.2, color, action);
        spawn_text(ui, materials, 50.0, y + 2.7, 2.8, item, TextAlign::Center, Srgba::rgb_u8(240, 229, 205));
        let detail = match i {
            0 => "Heavy footing / current resist",
            1 => "Light steps / jump control",
            _ => "Wall contact / ledge grip",
        };
        spawn_text(ui, materials, 50.0, y + 6.0, 2.0, detail, TextAlign::Center, Srgba::rgb_u8(171, 185, 199));
    }

    for (i, action_label) in demo.actions().iter().enumerate() {
        let y = 37.0 + i as f32 * 12.0;
        let selected = demo.focus_area == FocusArea::Actions && demo.selected_action == i;
        let color = focus_color(selected, i < 2);
        let click_action = active_face.then_some(ClickAction::Action(i));
        spawn_panel(ui, materials, 70.5, y, 23.0, 9.2, color, click_action);
        spawn_text(ui, materials, 82.0, y + 4.8, 2.8, action_label, TextAlign::Center, Srgba::rgb_u8(238, 229, 202));
    }

    let boot_state = if demo.iron_boots_equipped {
        if demo.iron_boots_active { "Feet: Iron Boots are ACTIVE" } else { "Feet: Iron Boots equipped, inactive" }
    } else {
        "Feet: empty; Iron Boots available"
    };
    let action = active_face.then_some(ClickAction::FocusArea(FocusArea::Actions));
    spawn_panel(ui, materials, 14.0, 74.0, 72.0, 9.0, Color::srgba(0.08, 0.09, 0.12, 0.84), action);
    spawn_text(ui, materials, 50.0, 78.6, 3.2, boot_state, TextAlign::Center, Srgba::rgb_u8(221, 230, 236));
}

fn spawn_pack_page(ui: &mut ChildSpawnerCommands, materials: &mut Assets<StandardMaterial>, demo: &InventoryDemo, active_face: bool) {
    let items = pack_items();
    spawn_text(ui, materials, 50.0, 34.0, 3.4, "Pack keeps consumables away from gear decisions.", TextAlign::Center, Srgba::rgb_u8(224, 226, 215));
    for (i, (name, detail, _important)) in items.iter().enumerate() {
        let x = if i % 2 == 0 { 17.0 } else { 53.0 };
        let y = 43.0 + (i / 2) as f32 * 14.0;
        let selected = demo.focus_area == FocusArea::Items && demo.selected_pack == i;
        let color = if selected { Color::srgba(0.55, 0.50, 0.68, 0.94) } else { Color::srgba(0.10, 0.13, 0.16, 0.92) };
        let action = active_face.then_some(ClickAction::PackItem(i));
        spawn_panel(ui, materials, x, y, 30.0, 10.0, color, action);
        spawn_text(ui, materials, x + 15.0, y + 3.4, 2.8, name, TextAlign::Center, Srgba::rgb_u8(236, 236, 220));
        spawn_text(ui, materials, x + 15.0, y + 7.0, 2.1, detail, TextAlign::Center, Srgba::rgb_u8(172, 190, 204));
    }
}

fn spawn_map_page(ui: &mut ChildSpawnerCommands, materials: &mut Assets<StandardMaterial>, demo: &InventoryDemo, active_face: bool) {
    spawn_text(ui, materials, 50.0, 33.5, 3.4, "Map face: real panels on the rotating volume.", TextAlign::Center, Srgba::rgb_u8(224, 232, 218));
    spawn_panel(ui, materials, 18.0, 41.0, 64.0, 31.0, Color::srgba(0.08, 0.13, 0.105, 0.93), None);
    for i in 0..5 {
        let y = 46.0 + i as f32 * 5.0;
        spawn_panel(ui, materials, 24.0, y, 52.0 - i as f32 * 5.5, 1.2, Color::srgba(0.38, 0.48, 0.38, 0.80), None);
    }
    let markers = map_markers();
    for (i, (label, x, y)) in markers.iter().enumerate() {
        let selected = demo.focus_area == FocusArea::Items && demo.selected_map == i;
        let color = if selected { Color::srgba(0.82, 0.58, 0.24, 0.96) } else { Color::srgba(0.18, 0.24, 0.18, 0.95) };
        let action = active_face.then_some(ClickAction::MapMarker(i));
        spawn_panel(ui, materials, *x, *y, 13.0, 6.0, color, action);
        spawn_text(ui, materials, *x + 6.5, *y + 3.1, 2.0, label, TextAlign::Center, Srgba::rgb_u8(235, 240, 220));
    }
    spawn_text(ui, materials, 50.0, 78.0, 2.9, "Select markers with arrows/D-pad or pointer.", TextAlign::Center, Srgba::rgb_u8(185, 204, 188));
}

fn spawn_status_page(ui: &mut ChildSpawnerCommands, materials: &mut Assets<StandardMaterial>, demo: &InventoryDemo, active_face: bool) {
    spawn_text(ui, materials, 50.0, 34.0, 3.4, "Character status", TextAlign::Center, Srgba::rgb_u8(235, 224, 220));

    let rows = status_rows(demo);
    let max_start = rows.len().saturating_sub(STATUS_VISIBLE_ROWS);
    let start = demo.status_scroll.min(max_start);
    let end = (start + STATUS_VISIBLE_ROWS).min(rows.len());

    spawn_panel(ui, materials, 17.0, 39.0, 66.0, 45.0, Color::srgba(0.065, 0.050, 0.062, 0.94), None);
    for (visible_idx, i) in (start..end).enumerate() {
        let (k, v, _kind) = &rows[i];
        let y = 43.0 + visible_idx as f32 * 8.2;
        let selected = demo.focus_area == FocusArea::Items && demo.selected_status == i;
        let color = if selected { Color::srgba(0.55, 0.50, 0.68, 0.94) } else { Color::srgba(0.13, 0.10, 0.12, 0.90) };
        let action = active_face.then_some(ClickAction::StatusRow(i));
        spawn_panel(ui, materials, 20.0, y, 58.0, 6.8, color, action);
        spawn_text(ui, materials, 34.0, y + 3.6, 2.35, k, TextAlign::Center, Srgba::rgb_u8(184, 190, 205));
        spawn_text(ui, materials, 62.0, y + 3.6, 2.35, v, TextAlign::Center, Srgba::rgb_u8(240, 226, 218));
    }

    if rows.len() > STATUS_VISIBLE_ROWS {
        let track_h = 38.0;
        let thumb_h = (STATUS_VISIBLE_ROWS as f32 / rows.len() as f32 * track_h).max(8.0);
        let max_scroll = rows.len() - STATUS_VISIBLE_ROWS;
        let thumb_y = 42.0 + (demo.status_scroll as f32 / max_scroll as f32) * (track_h - thumb_h);
        // Scrollbar track and thumb intentionally use separate depth bands; otherwise
        // the two thin overlapping planes can z-fight as the 3D page rotates.
        spawn_panel_at_depth(ui, materials, 80.3, 42.0, 1.2, track_h, Color::srgba(0.10, 0.09, 0.11, 0.96), DEPTH_LARGE_PANEL);
        spawn_panel_at_depth(ui, materials, 80.1, thumb_y, 1.6, thumb_h, Color::srgba(0.70, 0.55, 0.26, 0.98), DEPTH_ACTION);
        spawn_text(ui, materials, 50.0, 82.4, 2.0, "Status is a scroll pane: wheel or D-pad scrolls the selected row.", TextAlign::Center, Srgba::rgb_u8(188, 190, 205));
    }
}

#[derive(Clone, Copy, Debug)]
struct HitRect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

#[derive(Clone, Copy, Debug)]
struct HitTarget {
    rect: HitRect,
    action: ClickAction,
}

fn pack_items() -> [(&'static str, &'static str, bool); 6] {
    [
        ("Healing Tincture", "Consumable x3", true),
        ("Glow Seed", "Cavern light x5", false),
        ("Old Key", "Quest item / locked", true),
        ("Travel Ration", "Stamina snack x8", false),
        ("River Pearl", "Trade good x2", false),
        ("Sketch Map", "Field note", false),
    ]
}

fn pack_status(idx: usize) -> &'static str {
    match idx {
        0 => "Healing Tincture selected: restores health.",
        1 => "Glow Seed selected: marks dark paths.",
        2 => "Old Key selected: quest item, safe from selling.",
        3 => "Travel Ration selected: restores stamina over time.",
        4 => "River Pearl selected: trade good, can be sold safely.",
        _ => "Sketch Map selected: field note linked to the Map page.",
    }
}

fn map_markers() -> [(&'static str, f32, f32); 3] {
    [("Gate", 24.0, 50.0), ("Falls", 47.0, 58.0), ("Forge", 61.0, 46.0)]
}

fn map_marker_count() -> usize {
    map_markers().len()
}

fn map_status(idx: usize) -> &'static str {
    match idx {
        0 => "Gate marker selected: route back to town.",
        1 => "Falls marker selected: strong current suggests heavy footing.",
        _ => "Forge marker selected: upgrade and repair location.",
    }
}

fn status_rows(demo: &InventoryDemo) -> Vec<(&'static str, String, MenuControlKind)> {
    vec![
        ("Mobility", if demo.iron_boots_active { "Anchored" } else { "Normal" }.to_string(), MenuControlKind::Decoration),
        ("Feet slot", if demo.iron_boots_equipped { "Iron Boots" } else { "Empty" }.to_string(), MenuControlKind::Decoration),
        ("[ ] Input hints", checked_label(demo.input_hints_enabled), MenuControlKind::OptionToggle),
        ("Layout density", if demo.compact_layout { "Compact" } else { "Cozy" }.to_string(), MenuControlKind::OptionChoice),
        ("Detail level", demo.detail_level.label().to_string(), MenuControlKind::OptionChoice),
        ("Touch mode", if demo.touch_select_then_tap { "Select + tap" } else { "Instant tap" }.to_string(), MenuControlKind::OptionChoice),
        ("Menu toggle", demo.menu_toggle_binding.label().to_string(), MenuControlKind::OptionChoice),
        ("Open/close", demo.open_style.label().to_string(), MenuControlKind::OptionChoice),
        ("SFX hook", "Queued shell effects".to_string(), MenuControlKind::Decoration),
        ("Music hook", "Host can duck/muffle on Opened".to_string(), MenuControlKind::Decoration),
        ("Page switch", "Q/E, wheel, bumpers".to_string(), MenuControlKind::Decoration),
        ("Component", "Lunex data-driven shell".to_string(), MenuControlKind::Decoration),
    ]
}

fn checked_label(enabled: bool) -> String {
    if enabled { "[x] Enabled" } else { "[ ] Disabled" }.to_string()
}

fn status_row_count() -> usize {
    12
}

fn status_row_message(idx: usize, demo: &InventoryDemo) -> String {
    let rows = status_rows(demo);
    let (label, value, _) = &rows[idx.min(rows.len().saturating_sub(1))];
    format!("{label}: {value}")
}

fn move_pack_index(current: usize, dx: i32, dy: i32) -> usize {
    let len = pack_items().len() as i32;
    let columns = 2;
    let rows = (len + columns - 1) / columns;
    let col = (current as i32 % columns).clamp(0, columns - 1);
    let row = (current as i32 / columns).clamp(0, rows - 1);
    let mut next_col = (col + dx).rem_euclid(columns);
    let mut next_row = (row + dy).rem_euclid(rows);
    let mut next = next_row * columns + next_col;
    if next >= len {
        next_row = 0;
        next_col = next_col.min(columns - 1);
        next = next_row * columns + next_col;
    }
    next as usize
}

fn active_hit_targets(demo: &InventoryDemo) -> Vec<HitTarget> {
    let model = build_page_model(demo.page, demo, true);
    model
        .nodes
        .iter()
        .filter_map(|node| match node {
            MenuNode::Panel { rect, action: Some(action), .. } => Some(HitTarget {
                rect: HitRect { x: rect.x, y: rect.y, w: rect.w, h: rect.h },
                action: *action,
            }),
            MenuNode::Control { rect, action: Some(action), .. } => Some(HitTarget {
                rect: HitRect { x: rect.x, y: rect.y, w: rect.w, h: rect.h },
                action: *action,
            }),
            _ => None,
        })
        .collect()
}

fn rect_corners(rect: HitRect) -> [Vec3; 4] {
    let x0 = rect.x;
    let x1 = rect.x + rect.w;
    let y0 = rect.y;
    let y1 = rect.y + rect.h;
    [
        page_pct_to_local(x0, y0),
        page_pct_to_local(x1, y0),
        page_pct_to_local(x1, y1),
        page_pct_to_local(x0, y1),
    ]
}

fn page_pct_to_local(x: f32, y: f32) -> Vec3 {
    Vec3::new((x / 100.0 - 0.5) * PAGE_W, (0.5 - y / 100.0) * PAGE_H, 0.0)
}

fn hit_test_action(
    cursor: Vec2,
    demo: &InventoryDemo,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    face_transform: &GlobalTransform,
) -> Option<ClickAction> {
    let mut best: Option<(f32, ClickAction)> = None;
    for target in active_hit_targets(demo) {
        let mut min = Vec2::splat(f32::INFINITY);
        let mut max = Vec2::splat(f32::NEG_INFINITY);
        let mut ok = true;
        for local in rect_corners(target.rect) {
            let world = face_transform.transform_point(local);
            let Ok(screen) = camera.world_to_viewport(camera_transform, world) else {
                ok = false;
                break;
            };
            min = min.min(screen);
            max = max.max(screen);
        }
        if !ok {
            continue;
        }
        if cursor.x >= min.x && cursor.x <= max.x && cursor.y >= min.y && cursor.y <= max.y {
            let area = (max.x - min.x).abs() * (max.y - min.y).abs();
            if best.map(|(best_area, _)| area < best_area).unwrap_or(true) {
                best = Some((area, target.action));
            }
        }
    }
    best.map(|(_, action)| action)
}

fn panel_depth(w: f32, h: f32, actionable: bool) -> f32 {
    let area = w * h;
    if area > 8_000.0 {
        DEPTH_BACKGROUND
    } else if actionable {
        DEPTH_ACTION
    } else if area > 900.0 {
        DEPTH_LARGE_PANEL
    } else {
        DEPTH_CARD
    }
}

fn spawn_cube_edge_frame(ui: &mut ChildSpawnerCommands, materials: &mut Assets<StandardMaterial>) {
    // Thin rails make the mathematically touching page edges readable during
    // rotation, closer to the visible folded/page-section borders in OoT.
    let edge = Color::srgba(0.72, 0.55, 0.25, 0.96);
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
    action: Option<ClickAction>,
) {
    let depth = panel_depth(w, h, action.is_some());
    let material = materials.add(StandardMaterial {
        base_color: color,
        // Keep panels in the opaque pass. Semi-transparent overlapping planes on
        // rotating 3D UI are a common source of sorting flicker; this prototype
        // values stable readability over translucency.
        alpha_mode: AlphaMode::Opaque,
        cull_mode: None,
        unlit: true,
        ..default()
    });
    let mut entity = ui.spawn((
        Name::new("Lunex panel"),
        UiLayout::window()
            .x(Rl(x))
            .y(Rl(y))
            .width(Rl(w))
            .height(Rh(h))
            .anchor(Anchor::TOP_LEFT)
            .pack(),
        UiDepth::Set(depth),
        UiMeshPlane3d,
        MeshMaterial3d(material),
    ));
    if let Some(action) = action {
        entity.insert((
            OnHoverSetCursor::new(SystemCursorIcon::Pointer),
            UiHover::new().forward_speed(18.0).backward_speed(10.0),
            UiColor::new(vec![
                (UiBase::id(), color),
                (UiHover::id(), hover_panel_color()),
            ]),
        ));
        entity
            .observe(hover_set::<Pointer<Over>, true>)
            .observe(hover_set::<Pointer<Out>, false>);
    } else {
        entity.insert((UiColor::from(color), Pickable::IGNORE));
    }
}

fn spawn_panel_at_depth(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: Color,
    depth: f32,
) {
    let material = materials.add(StandardMaterial {
        base_color: color,
        alpha_mode: AlphaMode::Opaque,
        cull_mode: None,
        unlit: true,
        ..default()
    });
    ui.spawn((
        Name::new("Lunex depth panel"),
        UiLayout::window()
            .x(Rl(x))
            .y(Rl(y))
            .width(Rl(w))
            .height(Rh(h))
            .anchor(Anchor::TOP_LEFT)
            .pack(),
        UiDepth::Set(depth),
        UiMeshPlane3d,
        MeshMaterial3d(material),
        UiColor::from(color),
        Pickable::IGNORE,
    ));
}

fn hover_panel_color() -> Color {
    Color::srgba(0.86, 0.68, 0.30, 0.98)
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
) {
    let material = materials.add(StandardMaterial {
        base_color_texture: Some(TextAtlas::DEFAULT_IMAGE),
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        unlit: true,
        ..default()
    });
    ui.spawn((
        Name::new("Lunex text"),
        UiLayout::window()
            .x(Rl(x))
            .y(Rl(y))
            .anchor(Anchor::CENTER)
            .pack(),
        UiDepth::Set(DEPTH_TEXT),
        UiTextSize::from(Rh(size)),
        Text3d::new(text),
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

fn focus_color(selected: bool, important: bool) -> Color {
    match (selected, important) {
        (true, true) => Color::srgba(0.82, 0.58, 0.24, 0.96),
        (true, false) => Color::srgba(0.55, 0.50, 0.68, 0.94),
        (false, true) => Color::srgba(0.22, 0.18, 0.13, 0.90),
        (false, false) => Color::srgba(0.12, 0.12, 0.16, 0.88),
    }
}

fn menu_toggle_input(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    demo: Res<InventoryDemo>,
    mut shell: ResMut<MenuShell>,
) {
    let keyboard_toggle = demo.menu_toggle_binding.keyboard_pressed(&keys);
    let gamepad_toggle = gamepads.iter().any(|gamepad| demo.menu_toggle_binding.gamepad_pressed(gamepad));
    if keyboard_toggle || gamepad_toggle {
        shell.toggle();
    }
}

fn keyboard_navigation(keys: Res<ButtonInput<KeyCode>>, shell: Res<MenuShell>, mut demo: ResMut<InventoryDemo>, mut menu: ResMut<MenuAnimation>) {
    if !shell.is_interactive() {
        return;
    }
    let before_page = demo.page;

    // Page/cube navigation lives on explicit page controls.
    if keys.just_pressed(KeyCode::KeyQ) || keys.just_pressed(KeyCode::PageUp) {
        demo.previous_page();
    }
    if keys.just_pressed(KeyCode::KeyE) || keys.just_pressed(KeyCode::PageDown) {
        demo.next_page();
    }

    // Arrow keys / WASD navigate spatially inside the active page.
    if keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::KeyA) {
        demo.move_focus_horizontal(-1);
    }
    if keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::KeyD) {
        demo.move_focus_horizontal(1);
    }
    if keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::KeyW) {
        demo.move_focus_vertical(-1);
    }
    if keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::KeyS) {
        demo.move_focus_vertical(1);
    }

    if keys.just_pressed(KeyCode::Tab) {
        if keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
            demo.previous_focus_area();
        } else {
            demo.next_focus_area();
        }
    }

    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
        demo.activate_focused();
    }
    if keys.just_pressed(KeyCode::Backspace) {
        demo.back();
    }
    if keys.just_pressed(KeyCode::KeyT) {
        demo.try_toggle_iron_boots();
    }
    if demo.page != before_page {
        menu.set_page(demo.page);
    }
}

fn mouse_navigation(
    mut wheel: MessageReader<MouseWheel>,
    buttons: Res<ButtonInput<MouseButton>>,
    shell: Res<MenuShell>,
    mut demo: ResMut<InventoryDemo>,
    mut menu: ResMut<MenuAnimation>,
) {
    if !shell.is_interactive() {
        return;
    }
    let before_page = demo.page;
    for ev in wheel.read() {
        if demo.page == Page::Status {
            if ev.y > 0.0 {
                demo.scroll_status(-1);
            } else if ev.y < 0.0 {
                demo.scroll_status(1);
            }
        } else if ev.y > 0.0 {
            demo.previous_page();
        } else if ev.y < 0.0 {
            demo.next_page();
        }
    }
    // Left-click/tap is handled by the inventory-layer hit test below, not by Lunex observers.
    if buttons.just_pressed(MouseButton::Right) {
        demo.back();
    }
    if demo.page != before_page {
        menu.set_page(demo.page);
    }
}


fn pointer_hit_test(
    windows: Query<&Window, With<PrimaryWindow>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut touches: MessageReader<TouchInput>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    face_query: Query<(&PageFace, &GlobalTransform)>,
    shell: Res<MenuShell>,
    mut demo: ResMut<InventoryDemo>,
    mut menu: ResMut<MenuAnimation>,
    mut last_touch_selection: Local<Option<ClickAction>>,
    mut last_mouse_hover: Local<Option<ClickAction>>,
) {
    if !shell.is_interactive() {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };
    let Some((_, face_transform)) = face_query.iter().find(|(face, _)| face.0 == demo.page) else {
        return;
    };

    let before_page = demo.page;

    // Mouse hover/click is immediate. Hover only mutates focus when the
    // logical target changes, restoring highlight feedback without rebuilding
    // the active Lunex face on every tiny mouse-move event.
    if let Some(pos) = window.cursor_position() {
        let hovered = hit_test_action(pos, &demo, camera, camera_transform, face_transform);
        if hovered != *last_mouse_hover {
            if let Some(action) = hovered {
                demo.hover(action);
            }
            *last_mouse_hover = hovered;
        }
        if buttons.just_pressed(MouseButton::Left) {
            if let Some(action) = hovered {
                demo.click(action);
            }
        }
    }

    for touch in touches.read() {
        if matches!(touch.phase, TouchPhase::Started) {
            if let Some(action) = hit_test_action(touch.position, &demo, camera, camera_transform, face_transform) {
                if demo.touch_select_then_tap {
                    if *last_touch_selection == Some(action) {
                        demo.click(action);
                        *last_touch_selection = None;
                    } else {
                        demo.hover(action);
                        *last_touch_selection = Some(action);
                    }
                } else {
                    demo.click(action);
                    *last_touch_selection = None;
                }
            }
        }
    }

    if demo.page != before_page {
        menu.set_page(demo.page);
    }
}

fn gamepad_navigation(gamepads: Query<&Gamepad>, shell: Res<MenuShell>, mut demo: ResMut<InventoryDemo>, mut menu: ResMut<MenuAnimation>) {
    if !shell.is_interactive() {
        return;
    }
    let before_page = demo.page;
    for gamepad in &gamepads {
        // Page/cube navigation is on shoulder/trigger-style controls.
        // In the inside-cube view the room turns opposite the desired page
        // direction: RB pulls the next right-hand wall into view, LB pulls the
        // left-hand wall into view.
        if gamepad.just_pressed(GamepadButton::LeftTrigger) {
            demo.next_page();
        }
        if gamepad.just_pressed(GamepadButton::RightTrigger) {
            demo.previous_page();
        }

        // D-pad navigates spatially inside the active page. Shoulder buttons
        // still own page/cube rotation, so the D-pad is safe for menu focus.
        if gamepad.just_pressed(GamepadButton::DPadLeft) {
            demo.move_focus_horizontal(-1);
        }
        if gamepad.just_pressed(GamepadButton::DPadRight) {
            demo.move_focus_horizontal(1);
        }
        if gamepad.just_pressed(GamepadButton::DPadUp) {
            demo.move_focus_vertical(-1);
        }
        if gamepad.just_pressed(GamepadButton::DPadDown) {
            demo.move_focus_vertical(1);
        }

        // Face buttons perform actions. North also cycles focus groups for pads
        // that prefer face-button tabbing over D-pad horizontal navigation.
        if gamepad.just_pressed(GamepadButton::North) {
            demo.next_focus_area();
        }
        if gamepad.just_pressed(GamepadButton::South) {
            demo.activate_focused();
        }
        if gamepad.just_pressed(GamepadButton::East) {
            demo.back();
        }
        if gamepad.just_pressed(GamepadButton::West) {
            demo.try_toggle_iron_boots();
        }
    }
    if demo.page != before_page {
        menu.set_page(demo.page);
    }
}

fn animate_menu_ring(
    time: Res<Time>,
    demo: Res<InventoryDemo>,
    mut menu: ResMut<MenuAnimation>,
    mut shell: ResMut<MenuShell>,
    mut effects: ResMut<MenuShellEffects>,
    mut last_phase: Local<Option<MenuShellPhase>>,
    mut ring_query: Query<(&mut Transform, &mut Visibility), (With<MenuRing>, Without<LunexFaceRoot>)>,
    mut face_query: Query<(&PageFace, &mut Transform), (With<LunexFaceRoot>, Without<MenuRing>)>,
) {
    let Ok((mut transform, mut visibility)) = ring_query.single_mut() else {
        return;
    };

    let phase_before = shell.phase();

    let delta = shortest_angle_delta(menu.current_angle, menu.target_angle);
    let rotate_step = 1.0 - (-ROTATE_SPEED * time.delta_secs()).exp();
    menu.current_angle += delta * rotate_step;

    if delta.abs() < 0.001 {
        menu.current_angle = menu.target_angle;
    }

    let target = if shell.target_open { 1.0 } else { 0.0 };
    let open_step = 1.0 - (-OPEN_CLOSE_SPEED * time.delta_secs()).exp();
    shell.openness += (target - shell.openness) * open_step;
    if (shell.openness - target).abs() < 0.002 {
        shell.openness = target;
    }

    *visibility = if shell.is_visible() { Visibility::Visible } else { Visibility::Hidden };

    let phase_after = shell.phase();
    if *last_phase != Some(phase_after) {
        let effect = match phase_after {
            MenuShellPhase::Opening => MenuShellEffect::Opening,
            MenuShellPhase::Open => MenuShellEffect::Opened,
            MenuShellPhase::Closing => MenuShellEffect::Closing,
            MenuShellPhase::Closed => MenuShellEffect::Closed,
        };
        effects.push(effect);
        *last_phase = Some(phase_after);
    } else if phase_before != phase_after {
        // Kept for clarity if Bevy scheduling initializes the Local before the
        // first observed phase.
        *last_phase = Some(phase_after);
    }

    let open = smoothstep(shell.openness.clamp(0.0, 1.0));
    transform.rotation = Quat::from_rotation_y(menu.current_angle);

    match demo.open_style {
        OpenCloseStyle::SmoothScale => {
            let scale = MIN_OPEN_SCALE + (1.0 - MIN_OPEN_SCALE) * open;
            transform.scale = Vec3::splat(scale);
            transform.translation = Vec3::new(0.0, -0.05 * (1.0 - open), -0.42 * (1.0 - open));
            for (face, mut face_transform) in &mut face_query {
                reset_face_transform(face.0, &mut face_transform);
            }
        }
        OpenCloseStyle::OotPageFold => {
            // OoT's pause menu opens by driving a shared page pitch from 160.0
            // toward 0.0. In the draw matrices the source divides by 100, so
            // this is roughly 1.6 radians of fold. Front/back pages rotate on
            // their local X axis; side pages rotate on local Z. We keep the
            // camera inside the room and make the page walls build/fall away.
            transform.scale = Vec3::ONE;
            transform.translation = Vec3::new(0.0, -0.10 * (1.0 - open), 0.0);
            let fold = OOT_PAGE_FOLD_RADIANS * (1.0 - open);
            for (face, mut face_transform) in &mut face_query {
                apply_oot_open_fold(face.0, fold, &mut face_transform);
            }
        }
    }
}

fn wrap_index(current: usize, len: usize, delta: i32) -> usize {
    if len == 0 {
        return 0;
    }
    (current as i32 + delta).rem_euclid(len as i32) as usize
}

fn clamp_index_delta(current: usize, len: usize, delta: i32) -> usize {
    if len == 0 {
        return 0;
    }
    (current as i32 + delta).clamp(0, len as i32 - 1) as usize
}

fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn shortest_angle_delta(current: f32, target: f32) -> f32 {
    let two_pi = PI * 2.0;
    (target - current + PI).rem_euclid(two_pi) - PI
}
