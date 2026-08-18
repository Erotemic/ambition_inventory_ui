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
        if !demo.save_prompt_open {
            return;
        }
        if demo.save_complete {
            if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::KeyB) || keys.just_pressed(KeyCode::Escape) {
                demo.close_save_prompt("Returned to the pause menu.");
            }
            return;
        }
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
        if !demo.save_prompt_open {
            c_stick.active = None;
            nav_stick.active = None;
            return;
        }
        if demo.save_complete {
            for gamepad in &gamepads {
                if gamepad.just_pressed(GamepadButton::South) || gamepad.just_pressed(GamepadButton::East) || gamepad.just_pressed(GamepadButton::Start) {
                    demo.close_save_prompt("Returned to the pause menu.");
                }
            }
            c_stick.active = None;
            nav_stick.active = None;
            return;
        }
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


fn animate_equip_and_save(
    time: Res<Time>,
    readme_capture: Option<Res<ReadmeCapture>>,
    shell: Res<MenuShell>,
    mut demo: ResMut<OotDemo>,
) {
    if !shell.is_visible() {
        return;
    }
    let delta_secs = readme_animation_delta_secs(&time, readme_capture.as_deref());
    let save_step = 1.0 - (-SAVE_FLIP_SPEED * delta_secs).exp();
    let next_save = demo.save_flip + (demo.save_flip_target - demo.save_flip) * save_step;
    if (next_save - demo.save_flip).abs() > 0.001 {
        let was_prompt_face = demo.save_prompt_face_visible();
        demo.save_flip = next_save;
        if (demo.save_flip - demo.save_flip_target).abs() < 0.004 {
            demo.save_flip = demo.save_flip_target;
        }
        let is_prompt_face = demo.save_prompt_face_visible();
        if was_prompt_face && !is_prompt_face && !demo.save_prompt_open {
            demo.restore_normal_selection_after_save();
        }
        if demo.save_flip == 0.0 && !demo.save_prompt_open {
            if matches!(demo.selected, OotAction::SaveYes | OotAction::SaveNo | OotAction::Save) {
                demo.restore_normal_selection_after_save();
            }
            demo.save_complete = false;
        } else if !demo.save_prompt_open
            && demo.save_flip_target <= 0.001
            && !demo.save_prompt_face_visible()
            && matches!(demo.selected, OotAction::SaveYes | OotAction::SaveNo | OotAction::Save)
        {
            demo.restore_normal_selection_after_save();
        }
    }

    if let Some(mut anim) = demo.equip_anim {
        let speed = match anim.phase {
            EquipAnimPhase::ItemToButton => 4.5,
            EquipAnimPhase::ArrowGlowToBow => 5.8,
            EquipAnimPhase::ArrowBowHold => 3.5,
            EquipAnimPhase::BowToButton => 4.5,
        };
        anim.progress += delta_secs * speed;
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
    }
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
    readme_capture: Option<Res<ReadmeCapture>>,
    config: Res<MenuShellConfig>,
    mut menu: ResMut<MenuAnimation>,
    mut shell: ResMut<MenuShell>,
    demo: Res<OotDemo>,
    mut ring_query: Query<(&mut Transform, &mut Visibility), (With<MenuRing>, Without<LunexFaceRoot>)>,
    mut face_query: Query<(&PageFace, &mut Transform), (With<LunexFaceRoot>, Without<MenuRing>)>,
    mut hud_query: Query<(&mut Transform, &mut Visibility), (With<HudOverlayRoot>, Without<MenuRing>, Without<LunexFaceRoot>)>,
) {
    let Ok((mut transform, mut visibility)) = ring_query.single_mut() else { return; };
    let delta_secs = readme_animation_delta_secs(&time, readme_capture.as_deref());
    let delta = shortest_angle_delta(menu.current_angle, menu.target_angle);
    let rotate_step = 1.0 - (-config.page_rotate_speed * delta_secs).exp();
    menu.current_angle += delta * rotate_step;
    if delta.abs() < 0.001 {
        menu.current_angle = menu.target_angle;
    }
    let target = if shell.target_open { 1.0 } else { 0.0 };
    let open_step = 1.0 - (-config.open_close_speed * delta_secs).exp();
    shell.openness += (target - shell.openness) * open_step;
    if (shell.openness - target).abs() < 0.002 {
        shell.openness = target;
    }
    visibility.set_if_neq(if shell.is_visible() { Visibility::Visible } else { Visibility::Hidden });
    let desired_hud_transform = Transform::from_translation(Vec3::new(
        0.0,
        0.0,
        PAGE_RADIUS - HUD_Z_OFFSET_TOWARD_CAMERA,
    ))
    .with_scale(Vec3::new(HUD_SCREEN_X_FLIP, 1.0, 1.0));
    for (mut hud_transform, mut hud_visibility) in &mut hud_query {
        hud_visibility.set_if_neq(Visibility::Visible);
        hud_transform.set_if_neq(desired_hud_transform.clone());
    }
    let open = smoothstep(shell.openness.clamp(0.0, 1.0));
    let desired_ring_transform = Transform::from_translation(Vec3::new(
        0.0,
        -0.10 * (1.0 - open),
        0.0,
    ))
    .with_rotation(Quat::from_rotation_y(menu.current_angle));
    transform.set_if_neq(desired_ring_transform);
    let fold = OOT_PAGE_FOLD_RADIANS * (1.0 - open);
    for (face, mut face_transform) in &mut face_query {
        let desired = desired_face_transform(face.0, demo.page, fold, demo.save_flip);
        face_transform.set_if_neq(desired);
    }
}

#[derive(Clone, Copy, Debug)]
struct HitRect { x: f32, y: f32, w: f32, h: f32 }
impl HitRect {
    fn center(self) -> Vec2 { Vec2::new(self.x + self.w * 0.5, self.y + self.h * 0.5) }
}
#[derive(Clone, Copy, Debug)]
struct HitTarget { rect: HitRect, action: OotAction }

fn model_hit_targets(model: &MenuPageModel<OotAction>) -> Vec<HitTarget> {
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

fn active_page_focus_targets(demo: &OotDemo) -> Vec<HitTarget> {
    active_page_hit_targets(demo)
        .into_iter()
        .filter(|target| target.action.is_focusable_for(demo))
        .collect()
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

fn default_item_action_index() -> usize {
    // Adult demo starts on the Fairy Bow, which is the central assignable Adult
    // item and avoids landing the cursor on child-only row-one entries.
    bow_item_index()
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
