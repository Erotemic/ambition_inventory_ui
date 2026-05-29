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

    let page_actions_enabled = active_face && !demo.save_prompt_open && demo.save_flip_target <= 0.001;
    add_edge_buttons(&mut model, page, page_actions_enabled, demo.selected);
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

fn add_edge_buttons(model: &mut MenuPageModel<OotPage, OotAction>, _page: OotPage, active_face: bool, selected: OotAction) {
    model.control_with_icon(
        MenuRect::new(1.2, 38.0, 10.0, 24.0),
        MenuControlKind::Tab,
        "",
        Some("L".to_string()),
        Some("icons/oot/edge_left.png"),
        active_face && selected == OotAction::EdgeLeft,
        true,
        active_face.then_some(OotAction::EdgeLeft),
    );
    model.control_with_icon(
        MenuRect::new(88.8, 38.0, 10.0, 24.0),
        MenuControlKind::Tab,
        "",
        Some("R".to_string()),
        Some("icons/oot/edge_right.png"),
        active_face && selected == OotAction::EdgeRight,
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
        let usable = item.usable_by_current_link();
        let action = active_face.then_some(OotAction::Item(i));
        let detail = if usable { item.detail.map(|s| s.to_string()) } else { Some("child".to_string()) };
        model.control_with_icon(
            MenuRect::new(x, y, cell_w, cell_h),
            MenuControlKind::Item,
            "",
            detail,
            Some(item.icon),
            demo.selected == OotAction::Item(i),
            item.important && usable,
            action,
        );
    }
}

fn add_save_prompt_panel(model: &mut MenuPageModel<OotPage, OotAction>, demo: &OotDemo) {
    // Prompt contents are the only contents on the active face after the flip
    // midpoint. Keep this opaque and sparse to avoid z-fighting with the normal
    // inventory/equipment/map/quest controls.
    model.panel(MenuRect::new(18.0, 24.0, 64.0, 46.0), mc(Color::srgba(0.006, 0.008, 0.025, 1.0)), None);
    model.panel(MenuRect::new(24.0, 31.0, 52.0, 29.0), mc(Color::srgba(0.022, 0.026, 0.060, 1.0)), None);
    if demo.save_complete {
        model.text(50.0, 38.5, 3.6, "Saved.", MenuTextAlign::Center, mc(Color::srgb(0.94, 0.86, 0.55)));
        model.text(50.0, 45.0, 1.8, "Press A/B/Start to return", MenuTextAlign::Center, mc(Color::srgb(0.78, 0.84, 0.92)));
        model.control_with_icon(MenuRect::new(43.0, 51.0, 14.0, 7.8), MenuControlKind::Action, "OK", None, None::<String>, true, true, Some(OotAction::SaveNo));
    } else {
        model.text(50.0, 38.5, 3.2, "Would you like to save?", MenuTextAlign::Center, mc(Color::srgb(0.94, 0.86, 0.55)));
        model.control_with_icon(MenuRect::new(34.0, 47.0, 13.5, 7.8), MenuControlKind::Action, "YES", None, None::<String>, demo.selected == OotAction::SaveYes, true, Some(OotAction::SaveYes));
        model.control_with_icon(MenuRect::new(52.5, 47.0, 13.5, 7.8), MenuControlKind::Action, "NO", None, None::<String>, demo.selected == OotAction::SaveNo, true, Some(OotAction::SaveNo));
    }
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
        None,
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
        None,
        Some("icons/oot/hud_button_b.png"),
        false,
        true,
        Some(if in_prompt { OotAction::SaveNo } else { OotAction::Save }),
    );
    model.control_with_icon(
        A_BUTTON_RECT,
        MenuControlKind::Action,
        "",
        None,
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
        (demo.c_left, C_LEFT_RECT, CButton::Left, "icons/oot/hud_button_c_left.png"),
        (demo.c_down, C_DOWN_RECT, CButton::Down, "icons/oot/hud_button_c_down.png"),
        (demo.c_right, C_RIGHT_RECT, CButton::Right, "icons/oot/hud_button_c_right.png"),
    ];
    for (idx, rect, button, fallback_icon) in assignments {
        let item = oot_items()[idx];
        // C targets show arrows only when empty. Assigned items replace the
        // arrow art entirely and sit on an opaque yellow button plate, so the
        // arrow does not ghost through transparent pixels of the item icon.
        let icon = if item.name.is_empty() { fallback_icon } else { item.icon };
        model.control_with_icon(
            rect,
            MenuControlKind::Action,
            "",
            None,
            Some(icon),
            false,
            true,
            (!demo.save_modal_active()).then_some(OotAction::AssignC(button)),
        );
    }
}

fn add_equipment_page(model: &mut MenuPageModel<OotPage, OotAction>, demo: &OotDemo, active_face: bool) {
    model.panel(MenuRect::new(14.0, 20.0, 72.0, 58.0), mc(Color::srgba(0.055, 0.042, 0.025, 1.0)), None);

    // Closer to OoT's equipment page: an upgrades column at far left, a player preview
    // in the left-center, and the 3-choice equipment grid on the right.
    model.panel(MenuRect::new(29.0, 25.0, 16.0, 43.0), mc(equipment_preview_backing_color(demo)), None);
    model.control_with_icon(
        MenuRect::new(30.7, 27.0, 12.6, 29.0),
        MenuControlKind::Decoration,
        "",
        None,
        Some(equipped_player_icon(demo)),
        false,
        false,
        None,
    );
    add_equipped_preview_badges(model, demo);

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
            let usable = choice.usable_by_current_link();
            let detail = if equipped { Some("E".to_string()) } else if !usable { Some("child".to_string()) } else { None };
            model.control_with_icon(
                MenuRect::new(col_x[choice_idx], row_y[slot_idx], 9.5, 9.5),
                MenuControlKind::Item,
                "",
                detail,
                Some(choice.icon),
                demo.selected == action,
                equipped && usable,
                active_face.then_some(action),
            );
        }
    }
    model.text(50.0, 78.7, 2.5, "Equipment grid: upgrades / player preview / 3 choices per slot", MenuTextAlign::Center, mc(Color::srgb(0.82, 0.72, 0.48)));
}


fn equipped_player_icon(demo: &OotDemo) -> &'static str {
    match demo.equipped_tunic {
        1 => "icons/oot/player_goron_tunic.png",
        2 => "icons/oot/player_zora_tunic.png",
        _ => "icons/oot/player_kokiri_tunic.png",
    }
}

fn equipment_preview_backing_color(demo: &OotDemo) -> Color {
    match demo.equipped_tunic {
        1 => Color::srgba(0.115, 0.045, 0.030, 1.0),
        2 => Color::srgba(0.030, 0.075, 0.125, 1.0),
        _ => Color::srgba(0.045, 0.100, 0.065, 1.0),
    }
}

fn add_equipped_preview_badges(model: &mut MenuPageModel<OotPage, OotAction>, demo: &OotDemo) {
    let slots = equip_slots();
    let sword = slots[0].choices[demo.equipped_sword];
    let shield = slots[1].choices[demo.equipped_shield];
    let boots = slots[3].choices[demo.equipped_boots];
    let badges = [
        ("Sword", sword.icon, MenuRect::new(29.9, 57.8, 5.4, 5.4)),
        ("Shield", shield.icon, MenuRect::new(34.3, 60.6, 5.4, 5.4)),
        ("Boots", boots.icon, MenuRect::new(38.7, 57.8, 5.4, 5.4)),
    ];
    for (_label, icon, rect) in badges {
        model.control_with_icon(
            rect,
            MenuControlKind::Decoration,
            "",
            None,
            Some(icon),
            false,
            true,
            None,
        );
    }
    model.text(
        37.0,
        67.4,
        1.55,
        format!("{} / {} / {}", sword.name, shield.name, boots.name),
        MenuTextAlign::Center,
        mc(Color::srgb(0.83, 0.88, 0.74)),
    );
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

    // Heart-piece reminder. OoT shows collected heart pieces as a compact 2x2
    // group near the top-middle of the Quest Status page, separate from the
    // medallion/stone cluster. Keep these decorative and non-focusable.
    for row in 0..2 {
        for col in 0..2 {
            model.control_with_icon(
                MenuRect::new(45.2 + col as f32 * 5.4, 25.2 + row as f32 * 5.4, 4.9, 4.9),
                MenuControlKind::Decoration,
                "",
                None,
                Some("icons/oot/heart_piece.png"),
                false,
                false,
                None,
            );
        }
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

