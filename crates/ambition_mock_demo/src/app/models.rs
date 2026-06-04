fn publish_mock_page_models(
    demo: Res<MockDemo>,
    shell: Res<MenuShell>,
    mut pages: ResMut<ActiveMenuPages<MockPage, MockAction>>,
) {
    if !demo.is_changed() && !shell.is_changed() {
        return;
    }
    let models = MockPage::ALL
        .into_iter()
        .map(|page| build_page_model(page, &demo, page == demo.page))
        .collect();
    pages.replace_pages(models, demo.page);
    pages.visible = shell.is_visible();
}

fn build_page_model(page: MockPage, demo: &MockDemo, active_face: bool) -> MenuPageModel<MockPage, MockAction> {
    let mut model = MenuPageModel::new(
        page,
        format!("Ambition {}", page.label()),
        page_background(page),
    );
    model.panel(MenuRect::new(0.0, 0.0, 100.0, 100.0), page_background(page), None);
    add_edge_buttons(&mut model, demo, active_face);
    match page {
        MockPage::Items => add_items_page(&mut model, demo, active_face),
        MockPage::Map => add_placeholder_page(&mut model, page, "Map face placeholder", "Area map, discovered rooms, anchors, portal markers, and fast-travel affordances will be host-owned resources. The UI crate only renders this face and emits page/selection actions."),
        MockPage::Quest => add_placeholder_page(&mut model, page, "Quest face placeholder", "Quest status, key items, notes, and reminders should be built from Ambition save data. This placeholder proves a non-items page can live in the same cube seam."),
        MockPage::System => add_system_page(&mut model, demo, active_face),
    }
    model
}

fn add_edge_buttons(model: &mut MenuPageModel<MockPage, MockAction>, demo: &MockDemo, active_face: bool) {
    let left_page = MockDemo::page_on_viewer_left(demo.page);
    let right_page = MockDemo::page_on_viewer_right(demo.page);
    model.control(
        MenuRect::new(1.8, 43.5, 7.5, 13.0),
        MenuControlKind::Action,
        format!("<\n{}", left_page.label()),
        Some("turn cube left".to_string()),
        active_face && demo.selected == MockAction::EdgeLeft,
        active_face,
        active_face.then_some(MockAction::EdgeLeft),
    );
    model.control(
        MenuRect::new(90.7, 43.5, 7.5, 13.0),
        MenuControlKind::Action,
        format!(">\n{}", right_page.label()),
        Some("turn cube right".to_string()),
        active_face && demo.selected == MockAction::EdgeRight,
        active_face,
        active_face.then_some(MockAction::EdgeRight),
    );
}

fn add_items_page(model: &mut MenuPageModel<MockPage, MockAction>, demo: &MockDemo, active_face: bool) {
    model.text(50.0, 7.0, 6.0, "ITEMS", MenuTextAlign::Center, MenuColor::WHITE);
    model.text(50.0, 13.5, 2.3, "Host-owned counts/equipment -> page model -> generic UI actions", MenuTextAlign::Center, MenuColor::rgba(0.75, 0.86, 1.0, 0.95));

    model.panel(MenuRect::new(11.0, 19.0, 58.0, 55.0), MenuColor::rgba(0.025, 0.034, 0.095, 0.96), None);
    model.panel(MenuRect::new(70.5, 19.0, 18.2, 24.0), MenuColor::rgba(0.035, 0.046, 0.105, 0.96), None);
    model.panel(MenuRect::new(70.5, 45.0, 18.2, 29.0), MenuColor::rgba(0.035, 0.046, 0.105, 0.96), None);
    model.panel(MenuRect::new(11.0, 77.0, 77.7, 17.0), MenuColor::rgba(0.012, 0.018, 0.055, 0.96), None);

    let cell_w = 8.3;
    let cell_h = 10.2;
    let gap_x = 1.0;
    let gap_y = 1.3;
    let start_x = 14.0;
    let start_y = 23.0;
    for idx in 0..ITEM_COUNT {
        let item = &mock_items()[idx];
        let col = idx % ITEM_GRID_COLS;
        let row = idx / ITEM_GRID_COLS;
        let rect = MenuRect::new(
            start_x + col as f32 * (cell_w + gap_x),
            start_y + row as f32 * (cell_h + gap_y),
            cell_w,
            cell_h,
        );
        let owned = demo.has(idx);
        let selected = active_face && demo.selected == MockAction::Item(idx);
        let equipped = demo.is_equipped(idx);
        let detail = item_slot_detail(demo, idx);
        model.control(
            rect,
            MenuControlKind::Item,
            item_slot_label(demo, idx),
            Some(detail),
            selected,
            equipped,
            (active_face && owned && !matches!(item.kind, ItemKind::Reserved)).then_some(MockAction::Item(idx)),
        );
    }

    let held = demo
        .held_item
        .and_then(|idx| mock_items().get(idx).map(|item| item.name))
        .unwrap_or("<empty>");
    let body = demo
        .body_item
        .and_then(|idx| mock_items().get(idx).map(|item| item.name))
        .unwrap_or("<empty>");
    model.text(79.6, 24.0, 2.6, "EQUIPMENT", MenuTextAlign::Center, MenuColor::rgba(1.0, 0.84, 0.38, 1.0));
    model.text(72.5, 31.0, 2.1, "Held", MenuTextAlign::Left, MenuColor::rgba(0.75, 0.86, 1.0, 0.95));
    model.text(87.0, 31.0, 2.1, held, MenuTextAlign::Right, MenuColor::WHITE);
    model.text(72.5, 37.0, 2.1, "Body", MenuTextAlign::Left, MenuColor::rgba(0.75, 0.86, 1.0, 0.95));
    model.text(87.0, 37.0, 2.1, body, MenuTextAlign::Right, MenuColor::WHITE);

    model.text(79.6, 50.0, 2.6, "SELECTED", MenuTextAlign::Center, MenuColor::rgba(1.0, 0.84, 0.38, 1.0));
    for (line_idx, line) in selected_detail_lines(demo).into_iter().enumerate() {
        model.text(72.0, 55.5 + line_idx as f32 * 3.6, 1.85, line, MenuTextAlign::Left, MenuColor::rgba(0.88, 0.94, 1.0, 0.96));
    }
    let total_lines = detail_lines(demo, demo.selected_index()).len();
    if total_lines > DETAIL_VISIBLE_LINES {
        model.control(MenuRect::new(85.6, 47.6, 2.6, 4.2), MenuControlKind::Scrollbar, "^", None, demo.selected == MockAction::DetailScrollUp, false, active_face.then_some(MockAction::DetailScrollUp));
        model.control(MenuRect::new(85.6, 69.2, 2.6, 4.2), MenuControlKind::Scrollbar, "v", None, demo.selected == MockAction::DetailScrollDown, false, active_face.then_some(MockAction::DetailScrollDown));
        let frac = (DETAIL_VISIBLE_LINES as f32 / total_lines as f32).clamp(0.22, 1.0);
        let max_start = total_lines.saturating_sub(DETAIL_VISIBLE_LINES).max(1);
        let top = 52.5 + (demo.detail_scroll.min(max_start) as f32 / max_start as f32) * (14.5 * (1.0 - frac));
        model.panel(MenuRect::new(86.2, top, 1.2, 14.5 * frac), MenuColor::rgba(1.0, 0.78, 0.28, 0.88), None);
    }

    model.text(12.5, 83.0, 2.1, "Last host effect", MenuTextAlign::Left, MenuColor::rgba(1.0, 0.84, 0.38, 1.0));
    for (idx, line) in wrap_text(&demo.status, 82).into_iter().take(3).enumerate() {
        model.text(12.5, 87.0 + idx as f32 * 3.2, 1.85, line, MenuTextAlign::Left, MenuColor::rgba(0.82, 1.0, 0.82, 0.96));
    }
}

fn add_placeholder_page(model: &mut MenuPageModel<MockPage, MockAction>, page: MockPage, title: &str, body: &str) {
    model.text(50.0, 8.5, 6.0, page.label().to_uppercase(), MenuTextAlign::Center, MenuColor::WHITE);
    model.panel(MenuRect::new(14.0, 22.0, 72.0, 57.0), MenuColor::rgba(0.025, 0.034, 0.095, 0.96), None);
    model.text(50.0, 31.0, 4.2, title, MenuTextAlign::Center, MenuColor::rgba(1.0, 0.84, 0.38, 1.0));
    for (idx, line) in wrap_text(body, 66).into_iter().take(8).enumerate() {
        model.text(50.0, 43.0 + idx as f32 * 4.0, 2.2, line, MenuTextAlign::Center, MenuColor::rgba(0.84, 0.91, 1.0, 0.96));
    }
}

fn add_system_page(model: &mut MenuPageModel<MockPage, MockAction>, demo: &MockDemo, active_face: bool) {
    add_placeholder_page(model, MockPage::System, "System menu placeholder", "The fourth face is reserved for Ambition settings and pause options rather than OoT C-buttons or B-to-save. Video, audio, controls, return-to-title, and quit can land here as host actions.");
    let labels = ["Video settings", "Audio settings", "Controls", "Return to title"];
    for (idx, label) in labels.iter().enumerate() {
        model.control(
            MenuRect::new(32.0, 53.0 + idx as f32 * 7.4, 36.0, 5.5),
            MenuControlKind::OptionChoice,
            *label,
            Some("placeholder host action".to_string()),
            active_face && demo.selected == MockAction::Placeholder(idx),
            false,
            active_face.then_some(MockAction::Placeholder(idx)),
        );
    }
}

fn build_pause_hud_model(demo: &MockDemo, shell: &MenuShell) -> MenuPageModel<MockPage, MockAction> {
    let mut model = MenuPageModel::new(demo.page, "Ambition mock HUD", MenuColor::TRANSPARENT);
    model.panel(MenuRect::new(12.0, 4.8, 76.0, 7.2), MenuColor::rgba(0.01, 0.012, 0.045, 0.70), None);
    let pause_state = if shell.target_open { "PAUSED" } else { "UNPAUSING" };
    model.text(50.0, 8.6, 2.2, format!("{pause_state}  |  P/Esc pause toggle  |  Q/E or side arrows rotate  |  Enter/Space activate"), MenuTextAlign::Center, MenuColor::rgba(0.86, 0.94, 1.0, 0.96));
    model
}

fn page_spec(demo: &MockDemo) -> ItemsOnlyPageSpec<MockPage, MockAction> {
    let mut spec = ItemsOnlyPageSpec::new(MockPage::Items, "Ambition mock items")
        .selected_slot(Some(InventorySlotId(demo.selected_index())));
    for (idx, item) in mock_items().iter().enumerate() {
        let owned = demo.has(idx);
        let mut node = if owned {
            InventoryItemNode::new(idx, item.name)
        } else {
            InventoryItemNode::unowned(idx, item.name)
        }
        .detail(item.description)
        .selected(demo.selected == MockAction::Item(idx));
        if let ItemKind::Equippable(slot) = item.kind {
            node = node.equip_slot_label(slot.label()).action_label(if demo.is_equipped(idx) { "Unequip" } else { "Equip" });
            if demo.is_equipped(idx) {
                node = node.equipped(true);
            } else if let Some(current) = demo.equipped_in(slot).and_then(|old| mock_items().get(old)) {
                node = node.equip_conflict(format!("will replace {}", current.name));
            }
        }
        if matches!(item.kind, ItemKind::Consumable) && owned {
            node = node.count(demo.count(idx)).action_label("Use");
        }
        if owned && !matches!(item.kind, ItemKind::KeyItem | ItemKind::Reserved) {
            node = node.action(MockAction::Item(idx));
        }
        spec.push_cell(node);
    }
    spec
}

fn page_background(page: MockPage) -> MenuColor {
    match page {
        MockPage::Items => MenuColor::rgba(0.025, 0.033, 0.100, 0.98),
        MockPage::Map => MenuColor::rgba(0.020, 0.055, 0.085, 0.98),
        MockPage::Quest => MenuColor::rgba(0.052, 0.035, 0.088, 0.98),
        MockPage::System => MenuColor::rgba(0.030, 0.062, 0.050, 0.98),
    }
}

fn item_slot_label(demo: &MockDemo, idx: usize) -> String {
    let item = &mock_items()[idx];
    let prefix = if demo.is_equipped(idx) { "*" } else { item.glyph };
    let count = demo.count(idx);
    if count == 0 {
        format!("{prefix}\n{}\n--", item.short)
    } else if matches!(item.kind, ItemKind::Consumable) {
        format!("{prefix}\n{}\nx{count}", item.short)
    } else {
        format!("{prefix}\n{}", item.short)
    }
}

fn item_slot_detail(demo: &MockDemo, idx: usize) -> String {
    let item = &mock_items()[idx];
    if demo.count(idx) == 0 {
        "not owned".to_string()
    } else if demo.is_equipped(idx) {
        "equipped".to_string()
    } else {
        match item.kind {
            ItemKind::Equippable(slot) => match demo.equipped_in(slot).and_then(|old| mock_items().get(old)) {
                Some(old) => format!("replaces {}", old.short),
                None => "equip".to_string(),
            },
            ItemKind::Consumable => "use".to_string(),
            ItemKind::KeyItem => "key item".to_string(),
            ItemKind::Reserved => "reserved".to_string(),
        }
    }
}

fn selected_detail_lines(demo: &MockDemo) -> Vec<String> {
    let lines = detail_lines(demo, demo.selected_index());
    let total = lines.len().max(1);
    let visible = DETAIL_VISIBLE_LINES.min(total);
    let max_start = total.saturating_sub(visible);
    let start = demo.detail_scroll.min(max_start);
    let mut out = lines[start..(start + visible).min(total)].to_vec();
    if total > visible {
        out.push(format!("[{}/{}] PgUp/PgDn", start + 1, max_start + 1));
    }
    out
}

fn detail_lines(demo: &MockDemo, idx: usize) -> Vec<String> {
    let item = &mock_items()[idx];
    let mut lines = Vec::new();
    lines.push(item.name.to_string());
    lines.extend(wrap_text(item.description, DETAIL_WRAP_COLS));
    lines.push(String::new());
    lines.extend(wrap_text(&demo.describe_action(MockAction::Item(idx)), DETAIL_WRAP_COLS));
    lines
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    for paragraph in text.split('\n') {
        if paragraph.trim().is_empty() {
            out.push(String::new());
            continue;
        }
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let needs_space = !line.is_empty();
            let next_len = line.len() + word.len() + usize::from(needs_space);
            if next_len > width && !line.is_empty() {
                out.push(line);
                line = String::new();
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
        out.push(line);
    }
    out
}
