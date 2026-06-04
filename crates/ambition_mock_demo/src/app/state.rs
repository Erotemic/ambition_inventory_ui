#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
enum MockPage {
    Items,
    Map,
    Quest,
    System,
}

impl MockPage {
    const ALL: [Self; 4] = [Self::Items, Self::Map, Self::Quest, Self::System];

    const fn index(self) -> i32 {
        match self {
            Self::Items => 0,
            Self::Map => 1,
            Self::Quest => 2,
            Self::System => 3,
        }
    }

    fn from_index(index: i32) -> Self {
        let wrapped = index.rem_euclid(Self::ALL.len() as i32) as usize;
        Self::ALL[wrapped]
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Items => "Items",
            Self::Map => "Map",
            Self::Quest => "Quest",
            Self::System => "System",
        }
    }

    const fn subtitle(self) -> &'static str {
        match self {
            Self::Items => "equip / use items",
            Self::Map => "placeholder map",
            Self::Quest => "placeholder quest log",
            Self::System => "settings / pause options",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
enum PageTurn {
    ViewerLeft,
    ViewerRight,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
enum MockAction {
    EdgeLeft,
    EdgeRight,
    Item(usize),
    DetailScrollUp,
    DetailScrollDown,
    Placeholder(usize),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
enum EquipSlot {
    HeldItem,
    Body,
}

impl EquipSlot {
    const fn label(self) -> &'static str {
        match self {
            Self::HeldItem => "held item",
            Self::Body => "body",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
enum ItemKind {
    Equippable(EquipSlot),
    Consumable,
    KeyItem,
    Reserved,
}

impl ItemKind {
    const fn is_unique(self) -> bool {
        !matches!(self, Self::Consumable)
    }
}

#[derive(Resource, Clone, Debug)]
struct MockDemo {
    page: MockPage,
    selected: MockAction,
    counts: [u32; ITEM_COUNT],
    held_item: Option<usize>,
    body_item: Option<usize>,
    detail_scroll: usize,
    status: String,
    revision: u64,
}

impl Default for MockDemo {
    fn default() -> Self {
        Self::starter()
    }
}

impl MockDemo {
    fn starter() -> Self {
        let mut state = Self {
            page: MockPage::Items,
            selected: MockAction::Item(1),
            counts: [0; ITEM_COUNT],
            held_item: None,
            body_item: None,
            detail_scroll: 0,
            status: "Dummy game is paused. This is the Ambition mock inventory shell.".to_string(),
            revision: 0,
        };
        for (idx, item) in mock_items().iter().enumerate() {
            if item.start_count > 0 {
                state.grant(idx, item.start_count);
            }
        }
        state
    }

    fn bump(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }

    fn page_on_viewer_left(page: MockPage) -> MockPage {
        // This is the same inside-the-cube convention used by oot_pause_demo:
        // the face physically to the viewer's left is the next ring index.
        MockPage::from_index(page.index() + 1)
    }

    fn page_on_viewer_right(page: MockPage) -> MockPage {
        MockPage::from_index(page.index() - 1)
    }

    fn default_action_for_page(page: MockPage) -> MockAction {
        match page {
            MockPage::Items => MockAction::Item(1),
            MockPage::Map => MockAction::Placeholder(0),
            MockPage::Quest => MockAction::Placeholder(0),
            MockPage::System => MockAction::Placeholder(0),
        }
    }

    fn goto_page(&mut self, page: MockPage) {
        if self.page != page {
            self.page = page;
            self.selected = Self::default_action_for_page(page);
            self.detail_scroll = 0;
            self.status = format!("{} face: {}.", page.label(), page.subtitle());
            self.bump();
        }
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
            self.selected = match direction {
                PageTurn::ViewerLeft => MockAction::EdgeRight,
                PageTurn::ViewerRight => MockAction::EdgeLeft,
            };
            self.detail_scroll = 0;
            self.status = format!("{} face: {}.", target.label(), target.subtitle());
            self.bump();
        }
    }

    fn count(&self, idx: usize) -> u32 {
        self.counts.get(idx).copied().unwrap_or(0)
    }

    fn has(&self, idx: usize) -> bool {
        self.count(idx) > 0
    }

    fn grant(&mut self, idx: usize, n: u32) {
        let Some(item) = mock_items().get(idx) else { return; };
        let slot = &mut self.counts[idx];
        let next = slot.saturating_add(n);
        *slot = if item.kind.is_unique() { next.min(1) } else { next };
    }

    fn equipped_in(&self, slot: EquipSlot) -> Option<usize> {
        match slot {
            EquipSlot::HeldItem => self.held_item,
            EquipSlot::Body => self.body_item,
        }
    }

    fn set_equipped_in(&mut self, slot: EquipSlot, idx: Option<usize>) {
        match slot {
            EquipSlot::HeldItem => self.held_item = idx,
            EquipSlot::Body => self.body_item = idx,
        }
    }

    fn is_equipped(&self, idx: usize) -> bool {
        let Some(item) = mock_items().get(idx) else { return false; };
        match item.kind {
            ItemKind::Equippable(slot) => self.equipped_in(slot) == Some(idx),
            _ => false,
        }
    }

    fn selected_index(&self) -> usize {
        match self.selected {
            MockAction::Item(idx) => idx.min(ITEM_COUNT - 1),
            _ => 0,
        }
    }

    fn move_spatial(&mut self, dx: i32, dy: i32) {
        if self.page != MockPage::Items {
            if dx < 0 { self.turn_page_from_edge(PageTurn::ViewerLeft); }
            if dx > 0 { self.turn_page_from_edge(PageTurn::ViewerRight); }
            return;
        }
        match (self.selected, dx, dy) {
            (MockAction::EdgeLeft, d, 0) if d < 0 => {
                self.turn_page_from_edge(PageTurn::ViewerLeft);
                return;
            }
            (MockAction::EdgeRight, d, 0) if d > 0 => {
                self.turn_page_from_edge(PageTurn::ViewerRight);
                return;
            }
            _ => {}
        }
        let idx = self.selected_index();
        let row = idx / ITEM_GRID_COLS;
        let col = idx % ITEM_GRID_COLS;
        if dx < 0 && col == 0 {
            self.selected = MockAction::EdgeLeft;
            self.status = "Left page button: rotate to the next face on the left.".to_string();
            self.bump();
            return;
        }
        if dx > 0 && col == ITEM_GRID_COLS - 1 {
            self.selected = MockAction::EdgeRight;
            self.status = "Right page button: rotate to the next face on the right.".to_string();
            self.bump();
            return;
        }
        let next_col = (col as i32 + dx).clamp(0, (ITEM_GRID_COLS - 1) as i32) as usize;
        let next_row = (row as i32 + dy).clamp(0, (ITEM_GRID_ROWS - 1) as i32) as usize;
        let next = next_row * ITEM_GRID_COLS + next_col;
        if next != idx || !matches!(self.selected, MockAction::Item(_)) {
            self.selected = MockAction::Item(next);
            self.detail_scroll = 0;
            self.status = format!("{} selected.", mock_items()[next].name);
            self.bump();
        }
    }

    fn hover(&mut self, action: MockAction) {
        if self.selected != action {
            self.selected = action;
            self.detail_scroll = 0;
        }
        self.status = self.describe_action(action);
        self.bump();
    }

    fn describe_action(&self, action: MockAction) -> String {
        match action {
            MockAction::EdgeLeft => format!("Turn left to {}.", Self::page_on_viewer_left(self.page).label()),
            MockAction::EdgeRight => format!("Turn right to {}.", Self::page_on_viewer_right(self.page).label()),
            MockAction::Item(idx) => {
                let item = &mock_items()[idx];
                if !self.has(idx) {
                    format!("{} is visible but not acquired.", item.name)
                } else if self.is_equipped(idx) {
                    format!("{} is equipped. Activate to unequip.", item.name)
                } else {
                    match item.kind {
                        ItemKind::Equippable(slot) => format!("Equip {} to the {} slot.", item.name, slot.label()),
                        ItemKind::Consumable => format!("Use {}. Count: {}.", item.name, self.count(idx)),
                        ItemKind::KeyItem => format!("{} is a key item with no direct action.", item.name),
                        ItemKind::Reserved => "Reserved slot.".to_string(),
                    }
                }
            }
            MockAction::DetailScrollUp => "Scroll item description up.".to_string(),
            MockAction::DetailScrollDown => "Scroll item description down.".to_string(),
            MockAction::Placeholder(_) => format!("{} placeholder action.", self.page.label()),
        }
    }

    fn click(&mut self, action: MockAction) {
        self.selected = action;
        match action {
            MockAction::EdgeLeft => self.turn_page_from_edge(PageTurn::ViewerLeft),
            MockAction::EdgeRight => self.turn_page_from_edge(PageTurn::ViewerRight),
            MockAction::Item(idx) => self.activate_item(idx),
            MockAction::DetailScrollUp => self.scroll_detail(-1),
            MockAction::DetailScrollDown => self.scroll_detail(1),
            MockAction::Placeholder(idx) => {
                self.status = format!("{} placeholder option {idx} activated. Real Ambition data stays host-owned.", self.page.label());
                self.bump();
            }
        }
    }

    fn activate_selected(&mut self) {
        self.click(self.selected);
    }

    fn activate_item(&mut self, idx: usize) {
        let Some(item) = mock_items().get(idx) else { return; };
        if !self.has(idx) {
            self.status = format!("{} is not owned, so activation is ignored.", item.name);
            self.bump();
            return;
        }
        match item.kind {
            ItemKind::Equippable(slot) => {
                if self.equipped_in(slot) == Some(idx) {
                    self.set_equipped_in(slot, None);
                    self.status = format!("Unequipped {} from the {} slot.", item.name, slot.label());
                } else {
                    let previous = self.equipped_in(slot);
                    self.set_equipped_in(slot, Some(idx));
                    self.status = match previous.and_then(|old| mock_items().get(old)) {
                        Some(old) => format!("Equipped {} to the {} slot, replacing {}.", item.name, slot.label(), old.name),
                        None => format!("Equipped {} to the {} slot.", item.name, slot.label()),
                    };
                }
            }
            ItemKind::Consumable => {
                self.counts[idx] = self.counts[idx].saturating_sub(1);
                self.status = format!("Used {}. Remaining: {}.", item.name, self.count(idx));
            }
            ItemKind::KeyItem => {
                self.status = format!("{} is a key item; it has no direct inventory action.", item.name);
            }
            ItemKind::Reserved => {
                self.status = "Reserved slot has no action.".to_string();
            }
        }
        self.bump();
    }

    fn scroll_detail(&mut self, delta: i32) {
        let max_start = detail_lines(self, self.selected_index()).len().saturating_sub(DETAIL_VISIBLE_LINES);
        if delta < 0 {
            self.detail_scroll = self.detail_scroll.saturating_sub(delta.unsigned_abs() as usize);
        } else {
            self.detail_scroll = self.detail_scroll.saturating_add(delta as usize).min(max_start);
        }
        self.status = "Scrolled selected item description.".to_string();
        self.bump();
    }
}

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
    fn set_page(&mut self, page: MockPage) {
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
struct PageFace(MockPage);
#[derive(Component)]
struct FpsDebugText;
#[derive(Component)]
struct HudOverlayRoot;
#[derive(Component)]
struct MainPauseCamera;
#[derive(Component)]
struct DummyUnpausedOverlay;

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
