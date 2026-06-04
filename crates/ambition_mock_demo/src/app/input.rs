fn menu_toggle_input(keys: Res<ButtonInput<KeyCode>>, mut shell: ResMut<MenuShell>, mut demo: ResMut<MockDemo>) {
    let keyboard_pause = keys.just_pressed(KeyCode::KeyP);
    let keyboard_cancel = keys.just_pressed(KeyCode::Escape);
    if keyboard_pause || keyboard_cancel {
        shell.toggle();
        demo.status = if shell.target_open {
            "Paused dummy game. Reconstructing OoT-style cube shell.".to_string()
        } else {
            "Unpaused dummy game. Cube shell is collapsing; press P/Esc to bring it back.".to_string()
        };
        demo.bump();
    }
}

fn keyboard_navigation(keys: Res<ButtonInput<KeyCode>>, shell: Res<MenuShell>, mut demo: ResMut<MockDemo>) {
    if !shell.is_interactive() {
        return;
    }
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
    if keys.just_pressed(KeyCode::KeyR) {
        *demo = MockDemo::starter();
    }
    if keys.just_pressed(KeyCode::Digit1) {
        demo.goto_page(MockPage::Items);
    }
    if keys.just_pressed(KeyCode::Digit2) {
        demo.goto_page(MockPage::Map);
    }
    if keys.just_pressed(KeyCode::Digit3) {
        demo.goto_page(MockPage::Quest);
    }
    if keys.just_pressed(KeyCode::Digit4) {
        demo.goto_page(MockPage::System);
    }
    if keys.just_pressed(KeyCode::BracketLeft) {
        demo.scroll_detail(-1);
    }
    if keys.just_pressed(KeyCode::BracketRight) {
        demo.scroll_detail(1);
    }
    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::NumpadEnter) || keys.just_pressed(KeyCode::Space) {
        demo.activate_selected();
    }
}

fn mouse_navigation(mut wheel: MessageReader<MouseWheel>, shell: Res<MenuShell>, mut demo: ResMut<MockDemo>) {
    if !shell.is_interactive() {
        for _ in wheel.read() {}
        return;
    }
    for ev in wheel.read() {
        if ev.y > 0.0 {
            demo.turn_page(PageTurn::ViewerRight);
        } else if ev.y < 0.0 {
            demo.turn_page(PageTurn::ViewerLeft);
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct HitRect { x: f32, y: f32, w: f32, h: f32 }

#[derive(Clone, Copy, Debug)]
struct HitTarget { rect: HitRect, action: MockAction }

fn model_hit_targets(model: &MenuPageModel<MockPage, MockAction>) -> Vec<HitTarget> {
    model.nodes.iter().filter_map(|node| match node {
        MenuNode::Panel { rect, action: Some(action), .. } => Some(HitTarget { rect: HitRect { x: rect.x, y: rect.y, w: rect.w, h: rect.h }, action: *action }),
        MenuNode::Control { rect, action: Some(action), .. } => Some(HitTarget { rect: HitRect { x: rect.x, y: rect.y, w: rect.w, h: rect.h }, action: *action }),
        _ => None,
    }).collect()
}

fn active_page_hit_targets(demo: &MockDemo) -> Vec<HitTarget> {
    let model = build_page_model(demo.page, demo, true);
    model_hit_targets(&model)
}

fn active_hud_hit_targets(demo: &MockDemo, shell: &MenuShell) -> Vec<HitTarget> {
    let model = build_pause_hud_model(demo, shell);
    model_hit_targets(&model)
}

fn pointer_hit_test(
    windows: Query<&Window, With<PrimaryWindow>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut touches: MessageReader<TouchInput>,
    camera_query: Query<(&Camera, &GlobalTransform), With<CubePauseCamera>>,
    face_query: Query<(&AmbitionMenuPage<MockPage>, &GlobalTransform), With<CubeFace>>,
    hud_query: Query<&GlobalTransform, With<HudOverlayRoot>>,
    shell: Res<MenuShell>,
    mut demo: ResMut<MockDemo>,
    mut last_mouse_hover: Local<Option<MockAction>>,
) {
    if !shell.is_interactive() { return; }
    let Ok(window) = windows.single() else { return; };
    let Ok((camera, camera_transform)) = camera_query.single() else { return; };
    let Some((_, face_transform)) = face_query.iter().find(|(face, _)| face.id == demo.page) else { return; };
    let hud_transform = hud_query.single().ok();

    if let Some(pos) = window.cursor_position() {
        let hovered = hud_transform
            .and_then(|hud| hit_test_targets(pos, &active_hud_hit_targets(&demo, &shell), camera, camera_transform, hud))
            .or_else(|| hit_test_targets(pos, &active_page_hit_targets(&demo), camera, camera_transform, face_transform));
        if hovered != *last_mouse_hover {
            if let Some(action) = hovered { demo.hover(action); }
            *last_mouse_hover = hovered;
        }
        if buttons.just_released(MouseButton::Left) {
            if let Some(action) = hovered { demo.click(action); }
        }
    }
    for touch in touches.read() {
        if touch.phase == TouchPhase::Ended {
            let action = hud_transform
                .and_then(|hud| hit_test_targets(touch.position, &active_hud_hit_targets(&demo, &shell), camera, camera_transform, hud))
                .or_else(|| hit_test_targets(touch.position, &active_page_hit_targets(&demo), camera, camera_transform, face_transform));
            if let Some(action) = action { demo.click(action); }
        }
    }
}

fn hit_test_targets(cursor: Vec2, targets: &[HitTarget], camera: &Camera, camera_transform: &GlobalTransform, face_transform: &GlobalTransform) -> Option<MockAction> {
    let mut best: Option<(f32, MockAction)> = None;
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
