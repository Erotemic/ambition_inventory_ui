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
    save_complete: bool,
    save_flip: f32,
    save_flip_target: f32,
    save_return_selection: OotAction,
    equip_anim: Option<EquipAnim>,
    status: String,
    revision: u64,
}

impl Default for OotDemo {
    fn default() -> Self {
        Self {
            page: OotPage::Items,
            selected: OotAction::Item(3),
            equipped_sword: 1,
            equipped_shield: 1,
            equipped_tunic: 0,
            equipped_boots: 0,
            c_left: 9,
            c_down: 7,
            c_right: 3,
            save_prompt_open: false,
            save_complete: false,
            save_flip: 0.0,
            save_flip_target: 0.0,
            save_return_selection: OotAction::Item(3),
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
        // Treat the save prompt as modal while it is opening, visible, or still
        // on the prompt side of the closing flip. Once the closing flip crosses
        // back past edge-on, the normal pause page is visible again and should
        // immediately be interactive instead of staying in a disabled-looking
        // limbo until the last few degrees of rotation finish.
        self.save_prompt_open || self.save_flip_target > 0.001 || self.save_prompt_face_visible()
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
            OotPage::Items => OotAction::Item(default_item_action_index()),
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

    fn turn_page_from_edge(&mut self, direction: PageTurn) {
        let target = match direction {
            PageTurn::ViewerLeft => Self::page_on_viewer_left(self.page),
            PageTurn::ViewerRight => Self::page_on_viewer_right(self.page),
        };
        if self.page != target {
            self.page = target;
            // When the cursor crosses an edge prompt, OoT leaves the cursor on
            // the facing neighbor prompt of the newly visible page. Example:
            // moving right through R lands on L of the next page.
            self.selected = match direction {
                PageTurn::ViewerLeft => OotAction::EdgeRight,
                PageTurn::ViewerRight => OotAction::EdgeLeft,
            };
            self.status = format!("{} page", target.label());
            self.bump();
        }
    }

    fn next_page(&mut self) {
        self.turn_page(PageTurn::ViewerRight);
    }

    fn previous_page(&mut self) {
        self.turn_page(PageTurn::ViewerLeft);
    }

    fn focusable_action_or_default(&self, action: OotAction) -> OotAction {
        if action.is_focusable_for(self) {
            action
        } else {
            Self::default_action_for_page(self.page)
        }
    }

    fn restore_normal_selection_after_save(&mut self) {
        self.selected = self.focusable_action_or_default(self.save_return_selection);
    }

    fn hover(&mut self, action: OotAction) {
        let old_selected = self.selected;
        let old_status = self.status.clone();
        if action.is_focusable_for(self) {
            self.selected = action;
        }
        self.status = action.describe_hover(self);
        if self.selected != old_selected || self.status != old_status {
            self.bump();
        }
    }

    fn click(&mut self, action: OotAction) {
        let previous_selected = self.selected;
        if action.is_focusable_for(self) {
            self.selected = action;
        } else {
            match action {
                OotAction::Item(idx) => {
                    let item = oot_items()[idx];
                    self.status = format!("{} is child-only and disabled for Adult Link.", item.name);
                    self.bump();
                    return;
                }
                OotAction::EquipChoice { slot, choice } => {
                    let option = equip_slots()[slot].choices[choice];
                    self.status = format!("{} is child-only and disabled for Adult Link.", option.name);
                    self.bump();
                    return;
                }
                _ => {}
            }
        }
        match action {
            // OoT-style edge prompts: left/right are physical directions from the player's view.
            OotAction::EdgeLeft => self.turn_page_from_edge(PageTurn::ViewerLeft),
            OotAction::EdgeRight => self.turn_page_from_edge(PageTurn::ViewerRight),
            OotAction::Item(idx) => {
                let item = oot_items()[idx];
                if !item.usable_by_current_link() {
                    self.status = format!("{} is visible for layout accuracy, but Adult Link cannot use it.", item.name);
                } else {
                    self.status = format!("{} selected. Press Z/X/C to assign.", item.name);
                }
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
                self.open_save_prompt();
            }
            OotAction::SaveYes => {
                if self.save_complete {
                    self.close_save_prompt("Returned to the pause menu.");
                } else {
                    self.confirm_save();
                }
            }
            OotAction::SaveNo => {
                if self.save_complete {
                    self.close_save_prompt("Returned to the pause menu.");
                } else {
                    self.close_save_prompt("Save cancelled. Returning to the pause menu.");
                }
            }
            OotAction::EquipChoice { slot, choice } => {
                let option = equip_slots()[slot].choices[choice];
                if !option.usable_by_current_link() {
                    self.status = format!("{} is child-only in this Adult Link demo.", option.name);
                    self.bump();
                    return;
                }
                match slot {
                    0 => self.equipped_sword = choice,
                    1 => self.equipped_shield = choice,
                    2 => self.equipped_tunic = choice,
                    _ => self.equipped_boots = choice,
                }
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


    fn open_save_prompt(&mut self) {
        if self.save_prompt_open || self.save_flip_target > 0.0 {
            return;
        }
        self.save_return_selection = self.focusable_action_or_default(self.selected);
        self.save_prompt_open = true;
        self.save_complete = false;
        self.save_flip_target = 1.0;
        self.selected = OotAction::SaveYes;
        self.status = "Save? Choose Yes or No. The active page flips around its horizontal center line.".to_string();
        self.bump();
    }

    fn confirm_save(&mut self) {
        if !self.save_prompt_open {
            return;
        }
        // Match the source flow more closely: confirming YES shows a stable
        // saved acknowledgement on the prompt face instead of immediately
        // dropping into a closing animation where all normal inputs appear dead.
        self.save_complete = true;
        self.selected = OotAction::SaveNo;
        self.status = "Saved. Press A, B, Start, Enter, or Space to return.".to_string();
        self.bump();
    }

    fn close_save_prompt(&mut self, status: &str) {
        if !self.save_modal_active() {
            return;
        }
        self.save_prompt_open = false;
        // If the player confirmed YES, keep the Saved. acknowledgement visible
        // while the face flips away. Reset save_complete only after the normal
        // page has returned, otherwise the prompt visibly snaps back to YES/NO
        // during the closing half of the animation.
        self.save_flip_target = 0.0;
        // Keep SaveYes/SaveNo focused while the prompt side of the flip is still
        // visible. The normal page focus is restored when the flip crosses back
        // through the edge-on midpoint, so the prompt does not disappear with a
        // stale item cursor and the normal pane does not show a save selection.
        if !matches!(self.selected, OotAction::SaveYes | OotAction::SaveNo) {
            self.selected = OotAction::SaveNo;
        }
        self.status = status.to_string();
        self.bump();
    }

    fn toggle_save_prompt(&mut self) {
        if self.save_prompt_open || self.save_flip_target > 0.0 {
            self.close_save_prompt("Returned to the pause menu.");
        } else {
            self.open_save_prompt();
        }
    }

    fn start_c_button_equip(&mut self, item_idx: usize, button: CButton) {
        let item = oot_items()[item_idx];
        if !item.usable_by_current_link() {
            self.status = format!("{} cannot be assigned by Adult Link.", item.name);
            self.bump();
            return;
        }
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
            let item = oot_items()[idx];
            if !item.usable_by_current_link() {
                self.status = format!("{} is disabled for Adult Link and cannot be assigned.", item.name);
                self.bump();
                return;
            }
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
        if dy == 0 {
            match (self.selected, dx) {
                (OotAction::EdgeLeft, d) if d < 0 => {
                    self.turn_page_from_edge(PageTurn::ViewerLeft);
                    return;
                }
                (OotAction::EdgeRight, d) if d > 0 => {
                    self.turn_page_from_edge(PageTurn::ViewerRight);
                    return;
                }
                _ => {}
            }
        }
        let targets = active_page_focus_targets(self);
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
            // Edge prompts are horizontal navigation sentinels. They should be
            // reachable by moving left/right, but never steal focus from a
            // normal grid item when the player presses up/down near the page
            // edge. OoT keeps vertical item navigation inside the grid.
            if dy != 0 && matches!(target.action, OotAction::EdgeLeft | OotAction::EdgeRight) {
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
            // Favor the straight-line neighbor in the requested direction. The
            // previous forward-heavy score made diagonal edge prompts beat the
            // item directly above/right below in the 6x4 item grid, e.g. pressing
            // up from Nayru's Love could select R instead of Farore's Wind.
            let score = perp * 4.0 + forward;
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
            OotAction::Item(idx) => {
                let item = oot_items()[idx];
                if item.usable_by_current_link() {
                    item.name.to_string()
                } else {
                    format!("{} is child-only and disabled for Adult Link.", item.name)
                }
            }
            OotAction::EquipChoice { slot, choice } => {
                let option = equip_slots()[slot].choices[choice];
                if option.usable_by_current_link() {
                    format!("{}: {}", equip_slots()[slot].name, option.name)
                } else {
                    format!("{} is child-only and disabled for Adult Link.", option.name)
                }
            }
            OotAction::MapMarker(idx) => map_markers()[idx].name.to_string(),
            OotAction::QuestIcon(idx) => all_quest_icons()[idx].name.to_string(),
            OotAction::Song(idx) => songs()[idx].name.to_string(),
        }
    }

    fn is_focusable_for(self, demo: &OotDemo) -> bool {
        match self {
            // Edge prompts are focusable sentinels: arrowing onto L/R highlights
            // them, and arrowing farther past them rotates to the neighbor page
            // while keeping focus on the opposite prompt. HUD C/A/B/Start are
            // still actionable indicators, not focus targets.
            OotAction::EdgeLeft | OotAction::EdgeRight => !demo.save_modal_active(),
            OotAction::AssignC(_) | OotAction::Save => false,
            OotAction::SaveYes | OotAction::SaveNo => demo.save_prompt_face_visible(),
            // Source-like Adult Link behavior: child-only entries stay in the
            // grid and can receive the cursor so the layout remains navigable,
            // but activation/assignment/equip is blocked elsewhere.
            OotAction::Item(_) => demo.page == OotPage::Items,
            OotAction::EquipChoice { .. } => demo.page == OotPage::Equipment,
            OotAction::MapMarker(_) => demo.page == OotPage::Map,
            OotAction::QuestIcon(_) | OotAction::Song(_) => demo.page == OotPage::Quest,
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

#[derive(Resource, Clone, Debug)]
struct ReadmeCapture {
    output_dir: std::path::PathBuf,
    frame_count: u32,
    next_frame: u32,
    warmup_frames_remaining: u32,
    waiting_for_capture: bool,
    capture_started: bool,
    window_width: u32,
    window_height: u32,
}

impl ReadmeCapture {
    fn from_env() -> Option<Self> {
        let output_dir = std::env::var_os("OOT_CAPTURE_FRAMES_DIR")
            .map(std::path::PathBuf::from)?;
        let frame_count = std::env::var("OOT_CAPTURE_FRAME_COUNT")
            .ok()
            .and_then(|value| value.parse().ok())
            .filter(|count: &u32| *count > 0)
            .unwrap_or(60);
        let warmup_frames_remaining = std::env::var("OOT_CAPTURE_WARMUP_FRAMES")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(30);
        let window_width = std::env::var("OOT_CAPTURE_WINDOW_WIDTH")
            .ok()
            .and_then(|value| value.parse().ok())
            .filter(|width: &u32| *width > 0)
            .unwrap_or(1180);
        let window_height = std::env::var("OOT_CAPTURE_WINDOW_HEIGHT")
            .ok()
            .and_then(|value| value.parse().ok())
            .filter(|height: &u32| *height > 0)
            .unwrap_or(760);
        std::fs::create_dir_all(&output_dir)
            .expect("failed to create README animation capture directory");
        Some(Self {
            output_dir,
            frame_count,
            next_frame: 0,
            warmup_frames_remaining,
            waiting_for_capture: false,
            capture_started: false,
            window_width,
            window_height,
        })
    }

    fn window_resolution(&self) -> (u32, u32) {
        (self.window_width, self.window_height)
    }

    fn is_complete(&self) -> bool {
        self.next_frame >= self.frame_count
    }

    fn current_angle(&self) -> f32 {
        -(self.next_frame as f32 / self.frame_count as f32) * std::f32::consts::TAU
    }

    fn current_frame_path(&self) -> std::path::PathBuf {
        self.output_dir.join(format!("frame_{:04}.png", self.next_frame))
    }
}

#[derive(Resource, Debug)]
struct FpsWindow {
    samples: VecDeque<f32>,
    display_timer: f32,
}

impl Default for FpsWindow {
    fn default() -> Self {
        Self {
            samples: VecDeque::with_capacity(FPS_WINDOW_SAMPLES),
            display_timer: FPS_OVERLAY_UPDATE_SECS,
        }
    }
}

#[derive(Resource, Default, Debug)]
struct GamepadCStickState {
    active: Option<CButton>,
}

#[derive(Resource, Default, Debug)]
struct GamepadNavStickState {
    active: Option<(i32, i32)>,
}


