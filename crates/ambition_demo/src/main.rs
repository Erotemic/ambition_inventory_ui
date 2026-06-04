use std::f32::consts::{FRAC_PI_2, PI};
use std::sync::Arc;

use ambition_inventory_ui::{
    AmbitionMenuControl, AmbitionMenuPage, AmbitionMenuRoot, MenuColor, MenuControlKind,
    MenuFocusKey, MenuNode, MenuOpenCloseStyle, MenuPageModel, MenuRect, MenuScrollPane,
    MenuShellConfig, MenuShellEffect, MenuShellEffects, MenuShellPhase, MenuTextAlign,
    MenuVisualState, TouchActivationPolicy,
};
use bevy::anti_alias::fxaa::Fxaa;
use bevy::asset::AssetPlugin;
use bevy::core_pipeline::oit::OrderIndependentTransparencySettings;
use bevy::input::mouse::MouseWheel;
use bevy::input::touch::{TouchInput, TouchPhase};
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, SystemCursorIcon};
use bevy_lunex::prelude::*;

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
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    // This crate lives under crates/ambition_demo, while shared demo
                    // assets remain at the workspace root.
                    file_path: "../../assets".to_string(),
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Ambition Inventory UI Prototype - Lunex".to_string(),
                        resolution: (1180, 760).into(),
                        ..default()
                    }),
                    ..default()
                }),
        )
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
        .insert_resource(MenuShellConfig {
            // The library default is intentionally SmoothScale; this demo opts
            // into the nostalgic OoT fold so the example remains expressive.
            open_close_style: MenuOpenCloseStyle::OotPageFold,
            ..Default::default()
        })
        .add_systems(Startup, setup)
        .add_systems(Update, menu_toggle_input)
        .add_systems(
            Update,
            (
                keyboard_navigation,
                mouse_navigation,
                pointer_hit_test,
                gamepad_navigation,
            ),
        )
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
    equipped_weapon: usize,
    equipped_feet: Option<usize>,
    equipped_charm: Option<usize>,
    pack_counts: [u8; 6],
    iron_boots_active: bool,
    gear_action_popup_open: bool,
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
            open_style: OpenCloseStyle::from(MenuOpenCloseStyle::OotPageFold),
            status_scroll: 0,
            equipped_weapon: 0,
            equipped_feet: None,
            equipped_charm: None,
            pack_counts: [3, 5, 1, 8, 2, 1],
            iron_boots_active: false,
            gear_action_popup_open: false,
            status: "Select a gear item to open its action menu.".to_string(),
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
            0 => gear_items_for_slot(0)[self.equipped_weapon].0.to_string(),
            1 => match self.equipped_feet {
                Some(i) if i == 0 && self.iron_boots_active => "Iron Boots  [active]".to_string(),
                Some(i) => gear_items_for_slot(1)[i].0.to_string(),
                None => "Empty".to_string(),
            },
            2 => match self.equipped_charm {
                Some(i) => gear_items_for_slot(2)[i].0.to_string(),
                None => "Empty".to_string(),
            },
            _ => "".to_string(),
        }
    }

    fn items(&self) -> [&'static str; 3] {
        let items = gear_items_for_slot(self.selected_slot);
        [items[0].0, items[1].0, items[2].0]
    }

    fn item_detail(&self, idx: usize) -> &'static str {
        gear_items_for_slot(self.selected_slot)[idx].1
    }

    fn actions(&self) -> [&'static str; 3] {
        match self.selected_slot {
            1 if self.equipped_feet == Some(0) && self.selected_item == 0 => {
                ["Unequip", "Toggle active", "Inspect"]
            }
            1 if self.equipped_feet == Some(self.selected_item) => {
                ["Unequip", "Compare", "Inspect"]
            }
            2 if self.equipped_charm == Some(self.selected_item) => {
                ["Unequip", "Compare", "Inspect"]
            }
            0 if self.equipped_weapon == self.selected_item => ["Equipped", "Compare", "Inspect"],
            _ => ["Equip", "Compare", "Inspect"],
        }
    }

    fn is_selected_item_equipped(&self) -> bool {
        match self.selected_slot {
            0 => self.equipped_weapon == self.selected_item,
            1 => self.equipped_feet == Some(self.selected_item),
            2 => self.equipped_charm == Some(self.selected_item),
            _ => false,
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
            self.gear_action_popup_open = false;
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
                        self.selected_slot =
                            wrap_index(self.selected_slot, self.slots().len(), delta);
                    }
                }
                FocusArea::Items => {
                    if delta < 0 && self.selected_item == 0 {
                        self.focus_area = FocusArea::Tabs;
                    } else {
                        self.selected_item =
                            wrap_index(self.selected_item, self.items().len(), delta);
                    }
                }
                FocusArea::Actions => {
                    if delta < 0 && self.selected_action == 0 {
                        self.focus_area = FocusArea::Tabs;
                    } else {
                        self.selected_action =
                            wrap_index(self.selected_action, self.actions().len(), delta);
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
                    self.selected_status =
                        clamp_index_delta(self.selected_status, status_row_count(), delta);
                    self.ensure_status_visible();
                }
            }
        }
        self.bump();
    }

    fn move_focus_horizontal(&mut self, delta: i32) {
        if self.focus_area == FocusArea::Tabs {
            self.selected_tab =
                wrap_index(self.selected_tab, InventoryDemo::tab_pages().len(), delta);
            self.bump();
            return;
        }

        match self.page {
            Page::Gear => match (self.focus_area, delta.signum()) {
                (FocusArea::Tabs, _) => unreachable!(),
                (FocusArea::Slots, 1) => {
                    self.focus_area = FocusArea::Items;
                    self.selected_item =
                        self.selected_slot.min(self.items().len().saturating_sub(1));
                }
                (FocusArea::Items, 1) => {
                    self.focus_area = FocusArea::Actions;
                    self.selected_action = 0;
                    self.gear_action_popup_open = true;
                }
                (FocusArea::Items, -1) => {
                    self.focus_area = FocusArea::Slots;
                    self.selected_slot =
                        self.selected_item.min(self.slots().len().saturating_sub(1));
                }
                (FocusArea::Actions, -1) => {
                    self.focus_area = FocusArea::Items;
                    self.gear_action_popup_open = false;
                }
                _ => {}
            },
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
                self.selected_status =
                    clamp_index_delta(self.selected_status, status_row_count(), delta);
                self.ensure_status_visible();
            }
        }
        self.bump();
    }

    fn back(&mut self) {
        self.focus_area = if self.page == Page::Gear {
            match self.focus_area {
                FocusArea::Actions => {
                    self.gear_action_popup_open = false;
                    FocusArea::Items
                }
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
                    self.status =
                        format!("Choose gear for {} slot.", self.slots()[self.selected_slot]);
                }
                FocusArea::Items => {
                    self.focus_area = FocusArea::Actions;
                    self.selected_action = 0;
                    self.gear_action_popup_open = true;
                    self.status = format!(
                        "{} selected; choose an action.",
                        self.items()[self.selected_item]
                    );
                }
                FocusArea::Actions => self.activate_action(self.selected_action),
            },
            Page::Pack => {
                self.consume_pack_item(self.selected_pack);
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
        self.selected_status = self.selected_status.clamp(
            self.status_scroll,
            (self.status_scroll + STATUS_VISIBLE_ROWS - 1).min(count.saturating_sub(1)),
        );
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
            0 => self.equip_or_unequip_selected_item(),
            1 => {
                if self.selected_slot == 1 && self.selected_item == 0 {
                    self.try_toggle_iron_boots();
                } else {
                    let item = self.items()[self.selected_item];
                    let equipped = self.slot_value(self.selected_slot);
                    self.status = format!("Compare {item} with {equipped}.");
                    self.bump();
                }
            }
            _ => {
                let item = self.items()[self.selected_item];
                let detail = self.item_detail(self.selected_item);
                self.status = format!("{item}: {detail}.");
                self.bump();
            }
        }
    }

    fn equip_or_unequip_selected_item(&mut self) {
        let item = self.items()[self.selected_item];
        match self.selected_slot {
            0 => {
                self.equipped_weapon = self.selected_item;
                self.status = format!("{item} equipped as Weapon.");
            }
            1 => {
                if self.equipped_feet == Some(self.selected_item) {
                    self.equipped_feet = None;
                    self.iron_boots_active = false;
                    self.status = format!("{item} removed from Feet.");
                } else {
                    self.equipped_feet = Some(self.selected_item);
                    if self.selected_item != 0 {
                        self.iron_boots_active = false;
                    }
                    self.status = format!("{item} equipped to Feet.");
                }
            }
            2 => {
                if self.equipped_charm == Some(self.selected_item) {
                    self.equipped_charm = None;
                    self.status = format!("{item} charm removed.");
                } else {
                    self.equipped_charm = Some(self.selected_item);
                    self.status = format!("{item} charm equipped.");
                }
            }
            _ => {}
        }
        self.bump();
    }

    fn try_toggle_iron_boots(&mut self) {
        if self.equipped_feet == Some(0) {
            self.iron_boots_active = !self.iron_boots_active;
            self.status = if self.iron_boots_active {
                "Iron Boots active: anchored and heavy.".to_string()
            } else {
                "Iron Boots inactive: normal movement restored.".to_string()
            };
        } else {
            self.status = "Equip Iron Boots in the Feet slot before toggling them.".to_string();
        }
        self.bump();
    }

    fn consume_pack_item(&mut self, idx: usize) {
        self.selected_pack = idx;
        match idx {
            0 | 1 | 3 => {
                if self.pack_counts[idx] > 0 {
                    self.pack_counts[idx] -= 1;
                    self.status = pack_use_message(idx, self.pack_counts[idx]);
                } else {
                    self.status = format!("{} is empty.", pack_items()[idx].0);
                }
            }
            _ => {
                self.status = pack_status(idx, self);
            }
        }
        self.bump();
    }

    fn hover(&mut self, action: ClickAction) {
        let before = (
            self.page,
            self.focus_area,
            self.selected_tab,
            self.selected_slot,
            self.selected_item,
            self.selected_action,
            self.selected_pack,
            self.selected_map,
            self.selected_status,
            self.gear_action_popup_open,
        );
        match action {
            ClickAction::Goto(page) => {
                self.focus_area = FocusArea::Tabs;
                self.selected_tab = InventoryDemo::tab_index_for_page(page);
            }
            ClickAction::FocusArea(area) => {
                self.focus_area = area;
                if area == FocusArea::Actions {
                    self.gear_action_popup_open = true;
                }
            }
            ClickAction::SelectSlot(idx) => {
                self.page = Page::Gear;
                self.focus_area = FocusArea::Slots;
                self.selected_slot = idx;
                self.gear_action_popup_open = false;
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
                self.gear_action_popup_open = true;
            }
            ClickAction::PackItem(idx) => {
                self.page = Page::Pack;
                self.focus_area = FocusArea::Items;
                self.selected_pack = idx;
            }
            ClickAction::MapMarker(idx) => {
                self.page = Page::Map;
                self.focus_area = FocusArea::Items;
                self.selected_map = idx;
            }
            ClickAction::StatusRow(idx) => {
                self.page = Page::Status;
                self.focus_area = FocusArea::Items;
                self.selected_status = idx;
            }
            ClickAction::StatusScrollTo(slot) => {
                let max_start = status_row_count().saturating_sub(STATUS_VISIBLE_ROWS);
                self.page = Page::Status;
                self.focus_area = FocusArea::Items;
                self.status_scroll = slot.min(max_start);
                self.selected_status = self.selected_status.clamp(
                    self.status_scroll,
                    (self.status_scroll + STATUS_VISIBLE_ROWS - 1)
                        .min(status_row_count().saturating_sub(1)),
                );
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
        let after = (
            self.page,
            self.focus_area,
            self.selected_tab,
            self.selected_slot,
            self.selected_item,
            self.selected_action,
            self.selected_pack,
            self.selected_map,
            self.selected_status,
            self.gear_action_popup_open,
        );
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
                if area == FocusArea::Actions {
                    self.gear_action_popup_open = true;
                }
                self.bump();
            }
            ClickAction::SelectSlot(idx) => {
                self.page = Page::Gear;
                self.selected_slot = idx;
                self.selected_item = match idx {
                    0 => self.equipped_weapon,
                    1 => self.equipped_feet.unwrap_or(0),
                    2 => self.equipped_charm.unwrap_or(0),
                    _ => 0,
                };
                self.focus_area = FocusArea::Items;
                self.gear_action_popup_open = false;
                self.status = format!(
                    "{} slot selected; choose compatible gear.",
                    self.slots()[idx]
                );
                self.bump();
            }
            ClickAction::SelectItem(idx) => {
                self.page = Page::Gear;
                self.focus_area = FocusArea::Actions;
                self.selected_item = idx;
                self.selected_action = 0;
                self.gear_action_popup_open = true;
                self.status = format!("{} selected; choose an action.", self.items()[idx]);
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
                self.focus_area = FocusArea::Items;
                self.consume_pack_item(idx);
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
            ClickAction::StatusScrollTo(slot) => {
                let max_start = status_row_count().saturating_sub(STATUS_VISIBLE_ROWS);
                self.page = Page::Status;
                self.focus_area = FocusArea::Items;
                self.status_scroll = slot.min(max_start);
                self.selected_status = self.selected_status.clamp(
                    self.status_scroll,
                    (self.status_scroll + STATUS_VISIBLE_ROWS - 1)
                        .min(status_row_count().saturating_sub(1)),
                );
                self.status = format!("Status scroll: row {}.", self.status_scroll + 1);
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

impl From<MenuOpenCloseStyle> for OpenCloseStyle {
    fn from(value: MenuOpenCloseStyle) -> Self {
        match value {
            MenuOpenCloseStyle::SmoothScale => Self::SmoothScale,
            MenuOpenCloseStyle::OotPageFold => Self::OotPageFold,
        }
    }
}

impl From<OpenCloseStyle> for MenuOpenCloseStyle {
    fn from(value: OpenCloseStyle) -> Self {
        match value {
            OpenCloseStyle::SmoothScale => Self::SmoothScale,
            OpenCloseStyle::OotPageFold => Self::OotPageFold,
        }
    }
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
    StatusScrollTo(usize),
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
    asset_server: Res<AssetServer>,
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
            AmbitionMenuRoot,
            MenuRing,
            UiRoot3d,
            Transform::default(),
            Visibility::Visible,
        ))
        .id();

    commands.entity(ring).with_children(|ring| {
        spawn_all_faces(ring, &demo, &mut materials, &asset_server);
    });
}

fn rebuild_lunex_faces(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
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
            spawn_all_faces(ring, &demo, &mut materials, &asset_server);
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
            spawn_face(ring, demo.page, &demo, &mut materials, &asset_server);
        });
    }

    *last_revision = Some(demo.revision);
    *last_page = Some(demo.page);
}

fn spawn_all_faces(
    ring: &mut ChildSpawnerCommands,
    demo: &InventoryDemo,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
) {
    for page in InventoryDemo::pages() {
        spawn_face(ring, page, demo, materials, asset_server);
    }
}

fn spawn_face(
    ring: &mut ChildSpawnerCommands,
    page: Page,
    demo: &InventoryDemo,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
) {
    let (translation, rotation) = page_face_transform(page);
    let mut face = ring.spawn((
        Name::new(format!("{} Lunex face", page.label())),
        LunexFaceRoot,
        PageFace(page),
        AmbitionMenuPage {
            id: page,
            active: page == demo.page,
        },
        UiRoot3d,
        UiLayoutRoot::new_3d(),
        Dimension::from((PAGE_W, PAGE_H)),
        Transform::from_translation(translation)
            .with_rotation(rotation)
            .with_scale(Vec3::new(INSIDE_PAGE_X_FLIP, 1.0, 1.0)),
    ));
    if page == Page::Status {
        face.insert(MenuScrollPane {
            first_visible: demo.status_scroll,
            visible_rows: STATUS_VISIBLE_ROWS,
            total_rows: status_row_count(),
        });
    }
    face.with_children(|ui| {
        let active_face = page == demo.page;
        let model = build_page_model(page, demo, active_face);
        render_page_model(ui, materials, asset_server, &model);
    });
}

fn page_face_transform(page: Page) -> (Vec3, Quat) {
    // Inside-cube model: page centers sit one radius from the origin and the
    // page width is exactly 2 * radius, so neighboring walls share their
    // vertical edges. The root X flip applied in spawn_face corrects the
    // backface mirror so text reads normally from inside.
    match page {
        Page::Gear => (Vec3::new(0.0, 0.0, PAGE_RADIUS), Quat::IDENTITY),
        Page::Pack => (
            Vec3::new(PAGE_RADIUS, 0.0, 0.0),
            Quat::from_rotation_y(FRAC_PI_2),
        ),
        Page::Map => (Vec3::new(0.0, 0.0, -PAGE_RADIUS), Quat::from_rotation_y(PI)),
        Page::Status => (
            Vec3::new(-PAGE_RADIUS, 0.0, 0.0),
            Quat::from_rotation_y(-FRAC_PI_2),
        ),
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
    asset_server: &AssetServer,
    model: &MenuPageModel<Page, ClickAction>,
) {
    spawn_panel(
        ui,
        materials,
        0.0,
        0.0,
        100.0,
        100.0,
        menu_color(model.background),
        None,
    );
    spawn_cube_edge_frame(ui, materials);

    for node in &model.nodes {
        match node {
            MenuNode::Panel {
                rect,
                color,
                action,
            } => {
                spawn_panel(
                    ui,
                    materials,
                    rect.x,
                    rect.y,
                    rect.w,
                    rect.h,
                    menu_color(*color),
                    *action,
                );
            }
            MenuNode::Text {
                x,
                y,
                size,
                text,
                align,
                color,
            } => {
                spawn_text(
                    ui,
                    materials,
                    *x,
                    *y,
                    *size,
                    text,
                    menu_align(*align),
                    menu_srgba(*color),
                );
            }
            MenuNode::Control {
                rect,
                kind,
                label,
                detail,
                icon,
                selected,
                important,
                action,
            } => {
                spawn_control(
                    ui,
                    materials,
                    asset_server,
                    *rect,
                    *kind,
                    label,
                    detail.as_deref(),
                    icon.as_deref(),
                    *selected,
                    *important,
                    *action,
                );
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
    action: Option<ClickAction>,
) {
    let color = control_color(kind, selected, important);
    let depth = panel_depth(rect.w, rect.h, action.is_some());
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
        UiDepth::Set(depth),
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
            disabled: action.is_none(),
            ..Default::default()
        },
    ));

    if action.is_some() {
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

    let has_icon = icon.is_some();
    if let Some(icon_path) = icon {
        let icon_size = match kind {
            MenuControlKind::Tab => rect.h * 0.58,
            MenuControlKind::PopupAction => rect.h * 0.58,
            MenuControlKind::MapMarker => rect.h * 0.72,
            _ => rect.h.min(7.0) * 0.72,
        };
        let icon_x = rect.x
            + if matches!(
                kind,
                MenuControlKind::Tab | MenuControlKind::PopupAction | MenuControlKind::MapMarker
            ) {
                1.0
            } else {
                1.55
            };
        let icon_y = rect.y + (rect.h - icon_size) * 0.5;
        spawn_icon(
            ui,
            materials,
            asset_server,
            icon_x,
            icon_y,
            icon_size,
            icon_size,
            icon_path,
            icon_tint(kind, selected, important),
        );
    }

    let text_x = if has_icon {
        rect.x + rect.w * 0.60
    } else {
        rect.x + rect.w * 0.5
    };
    let label_y = if detail.is_some() {
        rect.y + rect.h * 0.30
    } else {
        rect.y + rect.h * 0.52
    };

    if let Some(detail) = detail {
        spawn_text(
            ui,
            materials,
            text_x,
            label_y,
            control_label_size(kind),
            label,
            TextAlign::Center,
            control_label_color(kind, selected, important),
        );
        spawn_text(
            ui,
            materials,
            text_x,
            rect.y + rect.h * 0.68,
            control_detail_size(kind),
            detail,
            TextAlign::Center,
            Srgba::rgb_u8(172, 190, 204),
        );
    } else {
        spawn_text(
            ui,
            materials,
            text_x,
            label_y,
            control_label_size(kind),
            label,
            TextAlign::Center,
            control_label_color(kind, selected, important),
        );
    }
}

fn spawn_icon(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    path: &str,
    tint: Color,
) {
    let material = materials.add(StandardMaterial {
        base_color: tint,
        base_color_texture: Some(asset_server.load(path.to_string())),
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        unlit: true,
        ..default()
    });
    ui.spawn((
        Name::new(format!("Lunex sprite icon {path}")),
        UiLayout::window()
            .x(Rl(x))
            .y(Rl(y))
            .width(Rl(w))
            .height(Rh(h))
            .anchor(Anchor::TOP_LEFT)
            .pack(),
        UiDepth::Set(DEPTH_TEXT + 0.10),
        UiMeshPlane3d,
        MeshMaterial3d(material),
        Pickable::IGNORE,
    ));
}

fn icon_tint(kind: MenuControlKind, selected: bool, important: bool) -> Color {
    match (kind, selected, important) {
        (MenuControlKind::Tab, true, _) => Color::srgba(0.22, 0.17, 0.10, 1.0),
        (_, true, _) => Color::srgba(1.0, 0.86, 0.52, 1.0),
        (_, _, true) => Color::srgba(0.90, 0.76, 0.48, 1.0),
        _ => Color::srgba(0.74, 0.78, 0.82, 1.0),
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
        MenuControlKind::Action | MenuControlKind::PopupAction => focus_color(selected, important),
        MenuControlKind::OptionToggle | MenuControlKind::OptionChoice => {
            if selected {
                Color::srgba(0.55, 0.50, 0.68, 0.94)
            } else {
                Color::srgba(0.13, 0.10, 0.12, 0.90)
            }
        }
        MenuControlKind::MapMarker => {
            if selected {
                Color::srgba(0.82, 0.58, 0.24, 0.96)
            } else {
                Color::srgba(0.18, 0.24, 0.18, 0.95)
            }
        }
        MenuControlKind::Scrollbar => {
            if selected {
                Color::srgba(0.70, 0.55, 0.26, 0.98)
            } else {
                Color::srgba(0.14, 0.12, 0.15, 0.96)
            }
        }
        MenuControlKind::PopupPanel => Color::srgba(0.035, 0.028, 0.045, 0.98),
        _ => focus_color(selected, important),
    }
}

fn control_label_size(kind: MenuControlKind) -> f32 {
    match kind {
        MenuControlKind::Tab => 2.7,
        MenuControlKind::MapMarker => 2.0,
        MenuControlKind::OptionToggle | MenuControlKind::OptionChoice => 2.35,
        MenuControlKind::Action | MenuControlKind::PopupAction => 2.8,
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
        MenuControlKind::OptionToggle | MenuControlKind::OptionChoice => {
            Srgba::rgb_u8(232, 228, 222)
        }
        _ => Srgba::rgb_u8(238, 229, 202),
    }
}

fn build_page_model(
    page: Page,
    demo: &InventoryDemo,
    active_face: bool,
) -> MenuPageModel<Page, ClickAction> {
    let mut model = MenuPageModel::new(page, page.label(), mc(page.face_color()));
    model.panel(
        MenuRect::new(3.0, 4.0, 94.0, 12.0),
        mc(Color::srgba(0.16, 0.13, 0.20, 0.92)),
        None,
    );
    model.text(
        50.0,
        9.5,
        7.2,
        page.label(),
        MenuTextAlign::Center,
        MenuColor::rgba(238.0 / 255.0, 222.0 / 255.0, 186.0 / 255.0, 1.0),
    );

    add_page_tabs(&mut model, demo, active_face);
    match page {
        Page::Gear => add_gear_nodes(&mut model, demo, active_face),
        Page::Pack => add_pack_nodes(&mut model, demo, active_face),
        Page::Map => add_map_nodes(&mut model, demo, active_face),
        Page::Status => add_status_nodes(&mut model, demo, active_face),
    }

    model.panel(
        MenuRect::new(5.0, 88.0, 90.0, 7.5),
        mc(Color::srgba(0.02, 0.018, 0.025, 0.88)),
        None,
    );
    model.text(
        50.0,
        91.8,
        3.4,
        demo.status.as_str(),
        MenuTextAlign::Center,
        MenuColor::rgba(198.0 / 255.0, 206.0 / 255.0, 218.0 / 255.0, 1.0),
    );
    model
}

fn add_page_tabs(
    model: &mut MenuPageModel<Page, ClickAction>,
    demo: &InventoryDemo,
    active_face: bool,
) {
    for (i, page) in InventoryDemo::tab_pages().iter().enumerate() {
        let active = *page == demo.page;
        let selected = demo.focus_area == FocusArea::Tabs && demo.selected_tab == i;
        model.control_with_icon(
            MenuRect::new(12.0 + i as f32 * 19.0, 18.0, 16.5, 6.5),
            MenuControlKind::Tab,
            page.label(),
            None,
            Some(page_icon(*page)),
            selected,
            active,
            active_face.then_some(ClickAction::Goto(*page)),
        );
    }
}

fn add_gear_nodes(
    model: &mut MenuPageModel<Page, ClickAction>,
    demo: &InventoryDemo,
    active_face: bool,
) {
    model.text(
        18.0,
        31.5,
        3.4,
        "Slots",
        MenuTextAlign::Center,
        MenuColor::rgba(235.0 / 255.0, 225.0 / 255.0, 200.0 / 255.0, 1.0),
    );
    model.text(
        50.0,
        31.5,
        3.4,
        "Compatible",
        MenuTextAlign::Center,
        MenuColor::rgba(235.0 / 255.0, 225.0 / 255.0, 200.0 / 255.0, 1.0),
    );
    model.text(
        82.0,
        31.5,
        3.0,
        "Action menu",
        MenuTextAlign::Center,
        MenuColor::rgba(190.0 / 255.0, 185.0 / 255.0, 176.0 / 255.0, 1.0),
    );

    for (i, slot) in demo.slots().iter().enumerate() {
        let y = 37.0 + i as f32 * 12.0;
        let selected = demo.focus_area == FocusArea::Slots && demo.selected_slot == i;
        model.control_with_icon(
            MenuRect::new(7.0, y, 23.0, 9.2),
            MenuControlKind::Slot,
            *slot,
            Some(demo.slot_value(i)),
            Some(slot_icon(i)),
            selected,
            i == 1,
            active_face.then_some(ClickAction::SelectSlot(i)),
        );
    }

    for (i, item) in demo.items().iter().enumerate() {
        let y = 37.0 + i as f32 * 12.0;
        model.control_with_icon(
            MenuRect::new(36.5, y, 27.0, 9.2),
            MenuControlKind::Item,
            *item,
            Some(demo.item_detail(i).to_string()),
            Some(gear_icon(demo.selected_slot, i)),
            demo.focus_area == FocusArea::Items && demo.selected_item == i,
            demo.is_selected_item_equipped() && demo.selected_item == i,
            active_face.then_some(ClickAction::SelectItem(i)),
        );
    }

    if demo.gear_action_popup_open || demo.focus_area == FocusArea::Actions {
        let popup_y = (34.2 + demo.selected_item as f32 * 12.0).clamp(34.2, 57.4);
        model.panel(
            MenuRect::new(66.0, popup_y, 28.0, 24.2),
            mc(Color::srgba(0.035, 0.028, 0.045, 0.98)),
            None,
        );
        model.text(
            80.0,
            popup_y + 3.2,
            2.4,
            demo.items()[demo.selected_item],
            MenuTextAlign::Center,
            MenuColor::rgba(225.0 / 255.0, 218.0 / 255.0, 198.0 / 255.0, 1.0),
        );
        for (i, action_label) in demo.actions().iter().enumerate() {
            let y = popup_y + 6.2 + i as f32 * 5.7;
            model.control_with_icon(
                MenuRect::new(68.0, y, 24.0, 4.9),
                MenuControlKind::PopupAction,
                *action_label,
                None,
                Some(action_icon(i, demo)),
                demo.focus_area == FocusArea::Actions && demo.selected_action == i,
                i < 2,
                active_face.then_some(ClickAction::Action(i)),
            );
        }
    } else {
        model.panel(
            MenuRect::new(68.5, 39.0, 25.0, 17.0),
            mc(Color::srgba(0.06, 0.055, 0.075, 0.80)),
            None,
        );
        model.text(
            81.0,
            44.0,
            2.25,
            "Select an item",
            MenuTextAlign::Center,
            MenuColor::rgba(188.0 / 255.0, 190.0 / 255.0, 205.0 / 255.0, 1.0),
        );
        model.text(
            81.0,
            49.0,
            2.05,
            "to open Equip /",
            MenuTextAlign::Center,
            MenuColor::rgba(164.0 / 255.0, 174.0 / 255.0, 190.0 / 255.0, 1.0),
        );
        model.text(
            81.0,
            52.5,
            2.05,
            "Compare / Inspect",
            MenuTextAlign::Center,
            MenuColor::rgba(164.0 / 255.0, 174.0 / 255.0, 190.0 / 255.0, 1.0),
        );
    }

    let boot_state = format!(
        "Weapon: {}   Feet: {}   Charm: {}",
        demo.slot_value(0),
        demo.slot_value(1),
        demo.slot_value(2),
    );
    model.panel(
        MenuRect::new(14.0, 74.0, 72.0, 9.0),
        mc(Color::srgba(0.08, 0.09, 0.12, 0.84)),
        active_face.then_some(ClickAction::FocusArea(FocusArea::Actions)),
    );
    model.text(
        50.0,
        78.6,
        3.0,
        boot_state.as_str(),
        MenuTextAlign::Center,
        MenuColor::rgba(221.0 / 255.0, 230.0 / 255.0, 236.0 / 255.0, 1.0),
    );
}

fn add_pack_nodes(
    model: &mut MenuPageModel<Page, ClickAction>,
    demo: &InventoryDemo,
    active_face: bool,
) {
    model.text(
        50.0,
        32.7,
        3.2,
        "Pack separates quick consumables, key items, and trade goods.",
        MenuTextAlign::Center,
        MenuColor::rgba(224.0 / 255.0, 226.0 / 255.0, 215.0 / 255.0, 1.0),
    );
    for (i, item) in pack_items().iter().enumerate() {
        let x = if i % 2 == 0 { 16.0 } else { 53.5 };
        let y = 41.0 + (i / 2) as f32 * 12.4;
        model.control_with_icon(
            MenuRect::new(x, y, 31.0, 9.5),
            MenuControlKind::Item,
            item.0,
            Some(pack_detail(i, demo)),
            Some(pack_icon(i)),
            demo.focus_area == FocusArea::Items && demo.selected_pack == i,
            item.2,
            active_face.then_some(ClickAction::PackItem(i)),
        );
    }
}

fn add_map_nodes(
    model: &mut MenuPageModel<Page, ClickAction>,
    demo: &InventoryDemo,
    active_face: bool,
) {
    model.text(
        50.0,
        33.5,
        3.4,
        "Map face: markers are controls, not a decorative image.",
        MenuTextAlign::Center,
        MenuColor::rgba(224.0 / 255.0, 232.0 / 255.0, 218.0 / 255.0, 1.0),
    );
    model.panel(
        MenuRect::new(18.0, 41.0, 64.0, 31.0),
        mc(Color::srgba(0.08, 0.13, 0.105, 0.93)),
        None,
    );
    for i in 0..5 {
        let y = 46.0 + i as f32 * 5.0;
        model.panel(
            MenuRect::new(24.0, y, 52.0 - i as f32 * 5.5, 1.2),
            mc(Color::srgba(0.38, 0.48, 0.38, 0.80)),
            None,
        );
    }
    for (i, (label, x, y)) in map_markers().iter().enumerate() {
        model.control_with_icon(
            MenuRect::new(*x, *y, 13.0, 6.0),
            MenuControlKind::MapMarker,
            *label,
            None,
            Some(map_icon(i)),
            demo.focus_area == FocusArea::Items && demo.selected_map == i,
            false,
            active_face.then_some(ClickAction::MapMarker(i)),
        );
    }
    model.text(
        50.0,
        78.0,
        2.9,
        "Select markers with arrows/D-pad or pointer.",
        MenuTextAlign::Center,
        MenuColor::rgba(185.0 / 255.0, 204.0 / 255.0, 188.0 / 255.0, 1.0),
    );
}

fn add_status_nodes(
    model: &mut MenuPageModel<Page, ClickAction>,
    demo: &InventoryDemo,
    active_face: bool,
) {
    model.text(
        50.0,
        34.0,
        3.4,
        "Character status / demo settings",
        MenuTextAlign::Center,
        MenuColor::rgba(235.0 / 255.0, 224.0 / 255.0, 220.0 / 255.0, 1.0),
    );
    let rows = status_rows(demo);
    let max_start = rows.len().saturating_sub(STATUS_VISIBLE_ROWS);
    let start = demo.status_scroll.min(max_start);
    let end = (start + STATUS_VISIBLE_ROWS).min(rows.len());

    model.panel(
        MenuRect::new(17.0, 39.0, 66.0, 45.0),
        mc(Color::srgba(0.065, 0.050, 0.062, 0.94)),
        None,
    );
    for (visible_idx, i) in (start..end).enumerate() {
        let (k, v, kind) = &rows[i];
        let y = 43.0 + visible_idx as f32 * 8.2;
        model.control_with_icon(
            MenuRect::new(20.0, y, 58.0, 6.8),
            *kind,
            *k,
            Some(v.clone()),
            Some(status_icon(i, *kind)),
            demo.focus_area == FocusArea::Items && demo.selected_status == i,
            matches!(
                kind,
                MenuControlKind::OptionToggle | MenuControlKind::OptionChoice
            ),
            active_face.then_some(ClickAction::StatusRow(i)),
        );
    }

    if rows.len() > STATUS_VISIBLE_ROWS {
        // Avoid the old overlapping track/thumb geometry. Thin transparent-ish
        // quads on a rotating 3D page were visually close enough to z-fight.
        // This segmented indicator never overlaps itself: one slot per scroll
        // position, with the active slot emphasized.
        let max_scroll = rows.len() - STATUS_VISIBLE_ROWS;
        let slot_h = 38.0 / (max_scroll + 1) as f32;
        for slot in 0..=max_scroll {
            let active = slot == demo.status_scroll.min(max_scroll);
            let y = 42.0 + slot as f32 * slot_h + 0.35;
            let h = (slot_h - 0.7).max(1.3);
            model.control(
                MenuRect::new(
                    if active { 79.95 } else { 80.35 },
                    y,
                    if active { 1.9 } else { 1.1 },
                    h,
                ),
                MenuControlKind::Scrollbar,
                "",
                None,
                active,
                active,
                active_face.then_some(ClickAction::StatusScrollTo(slot)),
            );
        }
        model.text(
            50.0,
            82.4,
            2.0,
            "Scroll pane: wheel, touch, or D-pad follows selection.",
            MenuTextAlign::Center,
            MenuColor::rgba(188.0 / 255.0, 190.0 / 255.0, 205.0 / 255.0, 1.0),
        );
    }
}

fn spawn_page_tabs(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    demo: &InventoryDemo,
    active_face: bool,
) {
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
            if active {
                Srgba::rgb_u8(35, 28, 21)
            } else {
                Srgba::rgb_u8(214, 207, 190)
            },
        );
    }
}

fn spawn_gear_page(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    demo: &InventoryDemo,
    active_face: bool,
) {
    spawn_text(
        ui,
        materials,
        18.0,
        31.5,
        3.4,
        "Slots",
        TextAlign::Center,
        Srgba::rgb_u8(235, 225, 200),
    );
    spawn_text(
        ui,
        materials,
        50.0,
        31.5,
        3.4,
        "Boots",
        TextAlign::Center,
        Srgba::rgb_u8(235, 225, 200),
    );
    spawn_text(
        ui,
        materials,
        82.0,
        31.5,
        3.4,
        "Actions",
        TextAlign::Center,
        Srgba::rgb_u8(235, 225, 200),
    );

    for (i, slot) in demo.slots().iter().enumerate() {
        let y = 37.0 + i as f32 * 12.0;
        let selected = demo.focus_area == FocusArea::Slots && demo.selected_slot == i;
        let color = focus_color(selected, i == 1);
        let action = active_face.then_some(ClickAction::SelectSlot(i));
        spawn_panel(ui, materials, 7.0, y, 23.0, 9.2, color, action);
        spawn_text(
            ui,
            materials,
            18.5,
            y + 2.6,
            2.7,
            slot,
            TextAlign::Center,
            Srgba::rgb_u8(238, 229, 202),
        );
        let value = demo.slot_value(i);
        spawn_text(
            ui,
            materials,
            18.5,
            y + 6.1,
            2.1,
            &value,
            TextAlign::Center,
            Srgba::rgb_u8(183, 192, 205),
        );
    }

    for (i, item) in demo.items().iter().enumerate() {
        let y = 37.0 + i as f32 * 12.0;
        let selected = demo.focus_area == FocusArea::Items && demo.selected_item == i;
        let is_iron = i == 0;
        let color = focus_color(selected, is_iron);
        let action = active_face.then_some(ClickAction::SelectItem(i));
        spawn_panel(ui, materials, 36.5, y, 27.0, 9.2, color, action);
        spawn_text(
            ui,
            materials,
            50.0,
            y + 2.7,
            2.8,
            item,
            TextAlign::Center,
            Srgba::rgb_u8(240, 229, 205),
        );
        let detail = match i {
            0 => "Heavy footing / current resist",
            1 => "Light steps / jump control",
            _ => "Wall contact / ledge grip",
        };
        spawn_text(
            ui,
            materials,
            50.0,
            y + 6.0,
            2.0,
            detail,
            TextAlign::Center,
            Srgba::rgb_u8(171, 185, 199),
        );
    }

    for (i, action_label) in demo.actions().iter().enumerate() {
        let y = 37.0 + i as f32 * 12.0;
        let selected = demo.focus_area == FocusArea::Actions && demo.selected_action == i;
        let color = focus_color(selected, i < 2);
        let click_action = active_face.then_some(ClickAction::Action(i));
        spawn_panel(ui, materials, 70.5, y, 23.0, 9.2, color, click_action);
        spawn_text(
            ui,
            materials,
            82.0,
            y + 4.8,
            2.8,
            action_label,
            TextAlign::Center,
            Srgba::rgb_u8(238, 229, 202),
        );
    }

    let boot_state = format!(
        "Weapon: {}   Feet: {}   Charm: {}",
        demo.slot_value(0),
        demo.slot_value(1),
        demo.slot_value(2),
    );
    let action = active_face.then_some(ClickAction::FocusArea(FocusArea::Actions));
    spawn_panel(
        ui,
        materials,
        14.0,
        74.0,
        72.0,
        9.0,
        Color::srgba(0.08, 0.09, 0.12, 0.84),
        action,
    );
    spawn_text(
        ui,
        materials,
        50.0,
        78.6,
        3.2,
        boot_state.as_str(),
        TextAlign::Center,
        Srgba::rgb_u8(221, 230, 236),
    );
}

fn spawn_pack_page(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    demo: &InventoryDemo,
    active_face: bool,
) {
    let items = pack_items();
    spawn_text(
        ui,
        materials,
        50.0,
        34.0,
        3.4,
        "Pack keeps consumables away from gear decisions.",
        TextAlign::Center,
        Srgba::rgb_u8(224, 226, 215),
    );
    for (i, (name, _detail, _important)) in items.iter().enumerate() {
        let x = if i % 2 == 0 { 17.0 } else { 53.0 };
        let y = 43.0 + (i / 2) as f32 * 14.0;
        let selected = demo.focus_area == FocusArea::Items && demo.selected_pack == i;
        let color = if selected {
            Color::srgba(0.55, 0.50, 0.68, 0.94)
        } else {
            Color::srgba(0.10, 0.13, 0.16, 0.92)
        };
        let action = active_face.then_some(ClickAction::PackItem(i));
        spawn_panel(ui, materials, x, y, 30.0, 10.0, color, action);
        spawn_text(
            ui,
            materials,
            x + 15.0,
            y + 3.4,
            2.8,
            name,
            TextAlign::Center,
            Srgba::rgb_u8(236, 236, 220),
        );
        let detail = pack_detail(i, demo);
        spawn_text(
            ui,
            materials,
            x + 15.0,
            y + 7.0,
            2.1,
            detail.as_str(),
            TextAlign::Center,
            Srgba::rgb_u8(172, 190, 204),
        );
    }
}

fn spawn_map_page(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    demo: &InventoryDemo,
    active_face: bool,
) {
    spawn_text(
        ui,
        materials,
        50.0,
        33.5,
        3.4,
        "Map face: real panels on the rotating volume.",
        TextAlign::Center,
        Srgba::rgb_u8(224, 232, 218),
    );
    spawn_panel(
        ui,
        materials,
        18.0,
        41.0,
        64.0,
        31.0,
        Color::srgba(0.08, 0.13, 0.105, 0.93),
        None,
    );
    for i in 0..5 {
        let y = 46.0 + i as f32 * 5.0;
        spawn_panel(
            ui,
            materials,
            24.0,
            y,
            52.0 - i as f32 * 5.5,
            1.2,
            Color::srgba(0.38, 0.48, 0.38, 0.80),
            None,
        );
    }
    let markers = map_markers();
    for (i, (label, x, y)) in markers.iter().enumerate() {
        let selected = demo.focus_area == FocusArea::Items && demo.selected_map == i;
        let color = if selected {
            Color::srgba(0.82, 0.58, 0.24, 0.96)
        } else {
            Color::srgba(0.18, 0.24, 0.18, 0.95)
        };
        let action = active_face.then_some(ClickAction::MapMarker(i));
        spawn_panel(ui, materials, *x, *y, 13.0, 6.0, color, action);
        spawn_text(
            ui,
            materials,
            *x + 6.5,
            *y + 3.1,
            2.0,
            label,
            TextAlign::Center,
            Srgba::rgb_u8(235, 240, 220),
        );
    }
    spawn_text(
        ui,
        materials,
        50.0,
        78.0,
        2.9,
        "Select markers with arrows/D-pad or pointer.",
        TextAlign::Center,
        Srgba::rgb_u8(185, 204, 188),
    );
}

fn spawn_status_page(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    demo: &InventoryDemo,
    active_face: bool,
) {
    spawn_text(
        ui,
        materials,
        50.0,
        34.0,
        3.4,
        "Character status",
        TextAlign::Center,
        Srgba::rgb_u8(235, 224, 220),
    );

    let rows = status_rows(demo);
    let max_start = rows.len().saturating_sub(STATUS_VISIBLE_ROWS);
    let start = demo.status_scroll.min(max_start);
    let end = (start + STATUS_VISIBLE_ROWS).min(rows.len());

    spawn_panel(
        ui,
        materials,
        17.0,
        39.0,
        66.0,
        45.0,
        Color::srgba(0.065, 0.050, 0.062, 0.94),
        None,
    );
    for (visible_idx, i) in (start..end).enumerate() {
        let (k, v, _kind) = &rows[i];
        let y = 43.0 + visible_idx as f32 * 8.2;
        let selected = demo.focus_area == FocusArea::Items && demo.selected_status == i;
        let color = if selected {
            Color::srgba(0.55, 0.50, 0.68, 0.94)
        } else {
            Color::srgba(0.13, 0.10, 0.12, 0.90)
        };
        let action = active_face.then_some(ClickAction::StatusRow(i));
        spawn_panel(ui, materials, 20.0, y, 58.0, 6.8, color, action);
        spawn_text(
            ui,
            materials,
            34.0,
            y + 3.6,
            2.35,
            k,
            TextAlign::Center,
            Srgba::rgb_u8(184, 190, 205),
        );
        spawn_text(
            ui,
            materials,
            62.0,
            y + 3.6,
            2.35,
            v,
            TextAlign::Center,
            Srgba::rgb_u8(240, 226, 218),
        );
    }

    if rows.len() > STATUS_VISIBLE_ROWS {
        let max_scroll = rows.len() - STATUS_VISIBLE_ROWS;
        let slot_h = 38.0 / (max_scroll + 1) as f32;
        for slot in 0..=max_scroll {
            let active = slot == demo.status_scroll.min(max_scroll);
            let y = 42.0 + slot as f32 * slot_h + 0.35;
            let h = (slot_h - 0.7).max(1.3);
            spawn_panel_at_depth(
                ui,
                materials,
                if active { 79.95 } else { 80.35 },
                y,
                if active { 1.9 } else { 1.1 },
                h,
                if active {
                    Color::srgba(0.70, 0.55, 0.26, 0.98)
                } else {
                    Color::srgba(0.14, 0.12, 0.15, 0.96)
                },
                DEPTH_ACTION,
            );
        }
        spawn_text(
            ui,
            materials,
            50.0,
            82.4,
            2.0,
            "Status is a scroll pane: wheel or D-pad scrolls the selected row.",
            TextAlign::Center,
            Srgba::rgb_u8(188, 190, 205),
        );
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

fn page_icon(page: Page) -> &'static str {
    match page {
        Page::Gear => "icons/tab_gear.png",
        Page::Pack => "icons/tab_pack.png",
        Page::Map => "icons/tab_map.png",
        Page::Status => "icons/tab_status.png",
    }
}

fn slot_icon(slot: usize) -> &'static str {
    match slot {
        0 => "icons/slot_weapon.png",
        1 => "icons/slot_feet.png",
        _ => "icons/slot_charm.png",
    }
}

fn gear_icon(slot: usize, idx: usize) -> &'static str {
    match (slot, idx) {
        (0, 0) => "icons/gear_sword.png",
        (0, 1) => "icons/gear_spear.png",
        (0, _) => "icons/gear_staff.png",
        (1, 0) => "icons/gear_iron_boots.png",
        (1, 1) => "icons/gear_feather_boots.png",
        (1, _) => "icons/gear_climbing_spikes.png",
        (2, 0) => "icons/charm_compass.png",
        (2, 1) => "icons/charm_river.png",
        _ => "icons/charm_guard.png",
    }
}

fn pack_icon(idx: usize) -> &'static str {
    match idx {
        0 => "icons/pack_tincture.png",
        1 => "icons/pack_glow_seed.png",
        2 => "icons/pack_key.png",
        3 => "icons/pack_ration.png",
        4 => "icons/pack_pearl.png",
        _ => "icons/pack_sketch_map.png",
    }
}

fn map_icon(idx: usize) -> &'static str {
    match idx {
        0 => "icons/map_gate.png",
        1 => "icons/map_falls.png",
        _ => "icons/map_forge.png",
    }
}

fn action_icon(idx: usize, demo: &InventoryDemo) -> &'static str {
    match idx {
        0 if demo.is_selected_item_equipped() => "icons/action_unequip.png",
        0 => "icons/action_equip.png",
        1 if demo.selected_slot == 1 && demo.selected_item == 0 => "icons/action_toggle.png",
        1 => "icons/action_compare.png",
        _ => "icons/action_inspect.png",
    }
}

fn status_icon(idx: usize, kind: MenuControlKind) -> &'static str {
    match kind {
        MenuControlKind::OptionToggle => "icons/status_checkbox.png",
        MenuControlKind::OptionChoice => "icons/status_radio.png",
        _ => match idx {
            0 => "icons/status_mobility.png",
            1 => "icons/slot_feet.png",
            8 => "icons/status_sfx.png",
            9 => "icons/status_music.png",
            10 => "icons/status_page_switch.png",
            11 => "icons/status_swipe.png",
            12 => "icons/status_cancel.png",
            13 => "icons/status_scroll.png",
            14 => "icons/tab_pack.png",
            _ => "icons/status_component.png",
        },
    }
}

fn gear_items_for_slot(slot: usize) -> [(&'static str, &'static str); 3] {
    match slot {
        0 => [
            ("Travel Sword", "Reliable blade / balanced reach"),
            ("Hook Spear", "Long reach / pulls light foes"),
            ("Ember Staff", "Fire channel / slow recovery"),
        ],
        1 => [
            ("Iron Boots", "Heavy footing / current resist"),
            ("Feather Boots", "Light steps / jump control"),
            ("Climbing Spikes", "Wall contact / ledge grip"),
        ],
        _ => [
            ("Compass Charm", "Nearby secrets shimmer on map"),
            ("River Charm", "Smoother swimming / water sense"),
            ("Guard Charm", "Small armor bonus while grounded"),
        ],
    }
}

fn pack_items() -> [(&'static str, &'static str, bool); 6] {
    [
        ("Healing Tincture", "Restores health", true),
        ("Glow Seed", "Drops a cavern light", false),
        ("Old Key", "Quest item / locked", true),
        ("Travel Ration", "Restores stamina", false),
        ("River Pearl", "Trade good", false),
        ("Sketch Map", "Field note", false),
    ]
}

fn pack_detail(idx: usize, demo: &InventoryDemo) -> String {
    let (_, base, _) = pack_items()[idx];
    match idx {
        0 | 1 | 3 => format!("{base} x{}", demo.pack_counts[idx]),
        4 => format!("{base} x{}", demo.pack_counts[idx]),
        _ => base.to_string(),
    }
}

fn pack_status(idx: usize, demo: &InventoryDemo) -> String {
    match idx {
        0 => format!("Healing Tincture: {} remaining.", demo.pack_counts[idx]),
        1 => format!("Glow Seed: {} remaining.", demo.pack_counts[idx]),
        2 => "Old Key: quest item, safe from selling.".to_string(),
        3 => format!("Travel Ration: {} remaining.", demo.pack_counts[idx]),
        4 => format!("River Pearl: trade good x{}.", demo.pack_counts[idx]),
        _ => "Sketch Map: field note linked to the Map page.".to_string(),
    }
}

fn pack_use_message(idx: usize, remaining: u8) -> String {
    match idx {
        0 => format!("Used Healing Tincture. {remaining} left."),
        1 => format!("Planted a Glow Seed. {remaining} left."),
        3 => format!("Ate a Travel Ration. {remaining} left."),
        _ => "Item inspected.".to_string(),
    }
}

fn map_markers() -> [(&'static str, f32, f32); 3] {
    [
        ("Gate", 24.0, 50.0),
        ("Falls", 47.0, 58.0),
        ("Forge", 61.0, 46.0),
    ]
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
        (
            "Mobility",
            if demo.iron_boots_active {
                "Anchored"
            } else {
                "Normal"
            }
            .to_string(),
            MenuControlKind::Decoration,
        ),
        (
            "Feet slot",
            if demo.equipped_feet == Some(0) {
                "Iron Boots"
            } else {
                "Empty"
            }
            .to_string(),
            MenuControlKind::Decoration,
        ),
        (
            "[ ] Input hints",
            checked_label(demo.input_hints_enabled),
            MenuControlKind::OptionToggle,
        ),
        (
            "Layout density",
            if demo.compact_layout {
                "Compact"
            } else {
                "Cozy"
            }
            .to_string(),
            MenuControlKind::OptionChoice,
        ),
        (
            "Detail level",
            demo.detail_level.label().to_string(),
            MenuControlKind::OptionChoice,
        ),
        (
            "Touch mode",
            if demo.touch_select_then_tap {
                "Select + tap"
            } else {
                "Instant tap"
            }
            .to_string(),
            MenuControlKind::OptionChoice,
        ),
        (
            "Menu toggle",
            demo.menu_toggle_binding.label().to_string(),
            MenuControlKind::OptionChoice,
        ),
        (
            "Open/close",
            demo.open_style.label().to_string(),
            MenuControlKind::OptionChoice,
        ),
        (
            "SFX hook",
            "Queued shell effects".to_string(),
            MenuControlKind::Decoration,
        ),
        (
            "Music hook",
            "Host can duck/muffle on Opened".to_string(),
            MenuControlKind::Decoration,
        ),
        (
            "Page switch",
            "Q/E, wheel, bumpers".to_string(),
            MenuControlKind::Decoration,
        ),
        (
            "Pointer swipe",
            "Drag left/right to change pages".to_string(),
            MenuControlKind::Decoration,
        ),
        (
            "Drag cancel",
            "Press, drag off, release cancels".to_string(),
            MenuControlKind::Decoration,
        ),
        (
            "Touch scroll",
            "Drag Status list vertically".to_string(),
            MenuControlKind::Decoration,
        ),
        (
            "Sprite icons",
            "Controls use asset-backed sprites".to_string(),
            MenuControlKind::Decoration,
        ),
        (
            "Component",
            "Lunex data-driven shell".to_string(),
            MenuControlKind::Decoration,
        ),
    ]
}

fn checked_label(enabled: bool) -> String {
    if enabled {
        "[x] Enabled"
    } else {
        "[ ] Disabled"
    }
    .to_string()
}

fn status_row_count() -> usize {
    16
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
            MenuNode::Panel {
                rect,
                action: Some(action),
                ..
            } => Some(HitTarget {
                rect: HitRect {
                    x: rect.x,
                    y: rect.y,
                    w: rect.w,
                    h: rect.h,
                },
                action: *action,
            }),
            MenuNode::Control {
                rect,
                action: Some(action),
                ..
            } => Some(HitTarget {
                rect: HitRect {
                    x: rect.x,
                    y: rect.y,
                    w: rect.w,
                    h: rect.h,
                },
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
    let gamepad_toggle = gamepads
        .iter()
        .any(|gamepad| demo.menu_toggle_binding.gamepad_pressed(gamepad));
    if keyboard_toggle || gamepad_toggle {
        shell.toggle();
    }
}

fn keyboard_navigation(
    keys: Res<ButtonInput<KeyCode>>,
    shell: Res<MenuShell>,
    mut demo: ResMut<InventoryDemo>,
    mut menu: ResMut<MenuAnimation>,
) {
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
    config: Res<MenuShellConfig>,
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

#[derive(Clone, Debug, Default)]
struct PointerDrag {
    active: bool,
    start_pos: Vec2,
    last_pos: Vec2,
    start_action: Option<ClickAction>,
    canceled: bool,
    scrolled: bool,
}

impl PointerDrag {
    fn begin(&mut self, pos: Vec2, action: Option<ClickAction>) {
        self.active = true;
        self.start_pos = pos;
        self.last_pos = pos;
        self.start_action = action;
        self.canceled = false;
        self.scrolled = false;
    }

    fn clear(&mut self) {
        *self = Self::default();
    }
}

fn update_pointer_drag(
    drag: &mut PointerDrag,
    pos: Vec2,
    hovered: Option<ClickAction>,
    demo: &mut InventoryDemo,
    drag_scroll_panes: bool,
    drag_off_cancels: bool,
) {
    if !drag.active {
        return;
    }
    let from_start = pos - drag.start_pos;
    let from_last = pos - drag.last_pos;

    // Modern touch-style scrolling: dragging up on the scrollable status list
    // moves deeper into the list; dragging down moves back toward the top.
    // This intentionally works with a mouse drag as a touch-gesture exerciser.
    if drag_scroll_panes
        && demo.page == Page::Status
        && from_last.y.abs() > 18.0
        && from_last.y.abs() > from_last.x.abs() * 1.15
    {
        let dragging_scrollbar = matches!(drag.start_action, Some(ClickAction::StatusScrollTo(_)));
        if dragging_scrollbar {
            // Thumb semantics: drag the scrollbar handle down to move deeper
            // into the list; drag it up to move toward the top.
            if from_last.y > 0.0 {
                demo.scroll_status(1);
            } else {
                demo.scroll_status(-1);
            }
        } else if from_last.y < 0.0 {
            // Content semantics: drag content up to reveal lower rows.
            demo.scroll_status(1);
        } else {
            demo.scroll_status(-1);
        }
        drag.last_pos = pos;
        drag.scrolled = true;
        drag.canceled = true;
        return;
    }

    // Press, hold, then drag off the original control cancels the activation.
    if drag_off_cancels && from_start.length() > 10.0 && hovered != drag.start_action {
        drag.canceled = true;
    }
    drag.last_pos = pos;
}

fn finish_pointer_drag(
    drag: &mut PointerDrag,
    pos: Vec2,
    hovered: Option<ClickAction>,
    demo: &mut InventoryDemo,
    touch_select_then_tap: bool,
    swipe_pages: bool,
    mut last_touch_selection: Option<&mut Option<ClickAction>>,
) {
    if !drag.active {
        return;
    }
    drag.last_pos = pos;
    let delta = drag.last_pos - drag.start_pos;

    if swipe_pages && delta.x.abs() > 74.0 && delta.x.abs() > delta.y.abs() * 1.25 {
        if let Some(last_touch) = last_touch_selection.as_deref_mut() {
            *last_touch = None;
        }
        if delta.x < 0.0 {
            demo.next_page();
        } else {
            demo.previous_page();
        }
        drag.clear();
        return;
    }

    if !drag.canceled && !drag.scrolled && drag.start_action == hovered {
        if let Some(action) = hovered {
            if let Some(last_touch) = last_touch_selection.as_deref_mut() {
                if touch_select_then_tap {
                    if *last_touch == Some(action) {
                        demo.click(action);
                        *last_touch = None;
                    } else {
                        demo.hover(action);
                        *last_touch = Some(action);
                    }
                } else {
                    demo.click(action);
                    *last_touch = None;
                }
            } else {
                demo.click(action);
            }
        }
    } else if drag.canceled && !drag.scrolled {
        demo.status = "Canceled drag.".to_string();
        demo.bump();
    }
    drag.clear();
}

fn pointer_hit_test(
    windows: Query<&Window, With<PrimaryWindow>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut touches: MessageReader<TouchInput>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    face_query: Query<(&PageFace, &GlobalTransform)>,
    shell: Res<MenuShell>,
    config: Res<MenuShellConfig>,
    mut demo: ResMut<InventoryDemo>,
    mut menu: ResMut<MenuAnimation>,
    mut last_touch_selection: Local<Option<ClickAction>>,
    mut last_mouse_hover: Local<Option<ClickAction>>,
    mut mouse_drag: Local<PointerDrag>,
    mut touch_drag: Local<PointerDrag>,
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

    if let Some(pos) = window.cursor_position() {
        let hovered = hit_test_action(pos, &demo, camera, camera_transform, face_transform);

        // Mouse hover should feel immediate, but only update focus when the
        // logical target changes. That keeps highlight feedback without forcing
        // full page rebuilds on every sub-pixel mouse movement.
        if hovered != *last_mouse_hover && !mouse_drag.active {
            if let Some(action) = hovered {
                demo.hover(action);
            }
            *last_mouse_hover = hovered;
        }

        if buttons.just_pressed(MouseButton::Left) {
            mouse_drag.begin(pos, hovered);
            if let Some(action) = hovered {
                demo.hover(action);
            }
        }
        if buttons.pressed(MouseButton::Left) {
            update_pointer_drag(
                &mut mouse_drag,
                pos,
                hovered,
                &mut demo,
                config.gestures.drag_scroll_panes,
                config.gestures.drag_off_cancels,
            );
        }
        if buttons.just_released(MouseButton::Left) {
            finish_pointer_drag(
                &mut mouse_drag,
                pos,
                hovered,
                &mut demo,
                false,
                config.gestures.swipe_pages,
                None,
            );
        }
    }

    for touch in touches.read() {
        let hovered = hit_test_action(
            touch.position,
            &demo,
            camera,
            camera_transform,
            face_transform,
        );
        match touch.phase {
            TouchPhase::Started => {
                touch_drag.begin(touch.position, hovered);
                if let Some(action) = hovered {
                    demo.hover(action);
                }
            }
            TouchPhase::Moved => {
                update_pointer_drag(
                    &mut touch_drag,
                    touch.position,
                    hovered,
                    &mut demo,
                    config.gestures.drag_scroll_panes,
                    config.gestures.drag_off_cancels,
                );
            }
            TouchPhase::Ended => {
                let select_then_tap = demo.touch_select_then_tap;
                finish_pointer_drag(
                    &mut touch_drag,
                    touch.position,
                    hovered,
                    &mut demo,
                    select_then_tap,
                    config.gestures.swipe_pages,
                    Some(&mut *last_touch_selection),
                );
            }
            TouchPhase::Canceled => {
                touch_drag.clear();
                *last_touch_selection = None;
            }
        }
    }

    if demo.page != before_page {
        menu.set_page(demo.page);
    }
}

fn gamepad_navigation(
    gamepads: Query<&Gamepad>,
    shell: Res<MenuShell>,
    mut demo: ResMut<InventoryDemo>,
    mut menu: ResMut<MenuAnimation>,
) {
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
    config: Res<MenuShellConfig>,
    mut menu: ResMut<MenuAnimation>,
    mut shell: ResMut<MenuShell>,
    mut effects: ResMut<MenuShellEffects>,
    mut last_phase: Local<Option<MenuShellPhase>>,
    mut ring_query: Query<
        (&mut Transform, &mut Visibility),
        (With<MenuRing>, Without<LunexFaceRoot>),
    >,
    mut face_query: Query<(&PageFace, &mut Transform), (With<LunexFaceRoot>, Without<MenuRing>)>,
) {
    let Ok((mut transform, mut visibility)) = ring_query.single_mut() else {
        return;
    };

    let phase_before = shell.phase();

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

    *visibility = if shell.is_visible() {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

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
