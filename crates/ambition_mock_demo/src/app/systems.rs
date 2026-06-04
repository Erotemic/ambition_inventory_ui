fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    demo: Res<MockDemo>,
    shell: Res<MenuShell>,
) {
    commands.spawn((
        DirectionalLight { illuminance: 2800.0, shadows_enabled: false, ..default() },
        Transform::from_xyz(1.5, 3.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Name::new("Ambition mock pause cube camera"),
        MainPauseCamera,
        Camera3d::default(),
        Camera { order: 0, ..default() },
        RenderLayers::layer(0),
        Msaa::Off,
        Transform::from_translation(CAMERA_EYE).looking_at(CAMERA_LOOK, Vec3::Y),
    ));
    commands.spawn((
        Name::new("Ambition mock HUD overlay camera"),
        Camera3d::default(),
        Camera { order: 10, clear_color: ClearColorConfig::None, ..default() },
        Msaa::Off,
        RenderLayers::layer(HUD_RENDER_LAYER),
        Transform::from_translation(CAMERA_EYE).looking_at(CAMERA_LOOK, Vec3::Y),
    ));
    commands.spawn((
        FpsDebugText,
        Text::new("fps: collecting..."),
        TextFont { font_size: 14.0, ..default() },
        TextColor(Color::srgba(0.86, 0.95, 0.88, 0.92)),
        Node { position_type: PositionType::Absolute, left: Val::Px(10.0), top: Val::Px(8.0), ..default() },
    ));
    commands.spawn((
        DummyUnpausedOverlay,
        Text::new("DUMMY UNPAUSED MODE\nNothing happens here yet. Press P or Esc to pause and reconstruct the cube menu."),
        TextFont { font_size: 24.0, ..default() },
        TextColor(Color::srgba(0.82, 0.90, 1.0, 0.88)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(40.0),
            bottom: Val::Px(38.0),
            ..default()
        },
        Visibility::Hidden,
    ));

    let ring = commands
        .spawn((
            Name::new("OoT-style Lunex pause room - Ambition mock"),
            AmbitionMenuRoot,
            MenuRing,
            UiRoot3d,
            Transform::default(),
            Visibility::Visible,
            RenderLayers::layer(0),
        ))
        .id();
    commands.entity(ring).with_children(|ring| {
        spawn_all_faces(ring, &demo, &mut materials);
    });
    spawn_hud_overlay(&mut commands, &demo, &shell, &mut materials);
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
    fps.display_timer += delta;
    if fps.display_timer < FPS_OVERLAY_UPDATE_SECS {
        return;
    }
    fps.display_timer = 0.0;
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
    demo: Res<MockDemo>,
    shell: Res<MenuShell>,
    ring_query: Query<Entity, With<MenuRing>>,
    face_query: Query<(Entity, &PageFace), With<LunexFaceRoot>>,
    hud_query: Query<Entity, With<HudOverlayRoot>>,
    mut last_revision: Local<Option<u64>>,
    mut last_page: Local<Option<MockPage>>,
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
        commands.entity(ring).with_children(|ring| spawn_all_faces(ring, &demo, &mut materials));
    } else {
        for (entity, face) in &face_query {
            if face.0 == demo.page {
                commands.entity(entity).despawn();
            }
        }
        commands.entity(ring).with_children(|ring| spawn_face(ring, demo.page, &demo, &mut materials));
    }
    for entity in &hud_query {
        commands.entity(entity).despawn();
    }
    spawn_hud_overlay(&mut commands, &demo, &shell, &mut materials);
    *last_revision = Some(demo.revision);
    *last_page = Some(demo.page);
}

fn spawn_hud_overlay(
    commands: &mut Commands,
    demo: &MockDemo,
    shell: &MenuShell,
    materials: &mut Assets<StandardMaterial>,
) {
    let model = build_pause_hud_model(demo, shell);
    commands.spawn((
        Name::new("Ambition mock pause HUD overlay"),
        HudOverlayRoot,
        UiRoot3d,
        UiLayoutRoot::new_3d(),
        Dimension::from((PAGE_W, PAGE_H)),
        Transform::from_translation(Vec3::new(0.0, 0.0, PAGE_RADIUS - HUD_Z_OFFSET_TOWARD_CAMERA))
            .with_scale(Vec3::new(HUD_SCREEN_X_FLIP, 1.0, 1.0)),
        Visibility::Visible,
        RenderLayers::layer(HUD_RENDER_LAYER),
    )).with_children(|ui| render_overlay_model(ui, materials, &model));
}

fn spawn_all_faces(
    ring: &mut ChildSpawnerCommands,
    demo: &MockDemo,
    materials: &mut Assets<StandardMaterial>,
) {
    for page in visible_face_pages(demo.page) {
        spawn_face(ring, page, demo, materials);
    }
}

fn visible_face_pages(active: MockPage) -> [MockPage; 3] {
    [
        MockDemo::page_on_viewer_left(active),
        active,
        MockDemo::page_on_viewer_right(active),
    ]
}

fn spawn_face(
    ring: &mut ChildSpawnerCommands,
    page: MockPage,
    demo: &MockDemo,
    materials: &mut Assets<StandardMaterial>,
) {
    let (translation, rotation) = page_face_transform(page);
    let mut face = ring.spawn((
        Name::new(format!("{} Ambition mock face", page.label())),
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
        let model = build_page_model(page, demo, page == demo.page);
        render_page_model(ui, materials, &model, page == demo.page);
    });
}

fn page_face_transform(page: MockPage) -> (Vec3, Quat) {
    match page {
        MockPage::Items => (Vec3::new(0.0, 0.0, PAGE_RADIUS), Quat::IDENTITY),
        MockPage::Map => (Vec3::new(PAGE_RADIUS, 0.0, 0.0), Quat::from_rotation_y(FRAC_PI_2)),
        MockPage::Quest => (Vec3::new(0.0, 0.0, -PAGE_RADIUS), Quat::from_rotation_y(PI)),
        MockPage::System => (Vec3::new(-PAGE_RADIUS, 0.0, 0.0), Quat::from_rotation_y(-FRAC_PI_2)),
    }
}

fn reset_face_transform(page: MockPage, transform: &mut Transform) {
    let (translation, rotation) = page_face_transform(page);
    transform.translation = translation;
    transform.rotation = rotation;
    transform.scale = Vec3::new(INSIDE_PAGE_X_FLIP, 1.0, 1.0);
}

fn apply_oot_open_fold(page: MockPage, fold: f32, transform: &mut Transform) {
    let (base_translation, base_rotation) = page_face_transform(page);
    let fold_rotation = match page {
        MockPage::Items => Quat::from_rotation_x(fold),
        MockPage::Quest => Quat::from_rotation_x(-fold),
        MockPage::Map => Quat::from_rotation_z(-fold),
        MockPage::System => Quat::from_rotation_z(fold),
    };
    let rotation = fold_rotation * base_rotation;
    let hinge_local = Vec3::new(0.0, -PAGE_H * 0.5, 0.0);
    let hinge_world = base_translation + base_rotation * hinge_local;
    let translation = hinge_world - rotation * hinge_local;
    transform.translation = translation;
    transform.rotation = rotation;
    transform.scale = Vec3::new(INSIDE_PAGE_X_FLIP, 1.0, 1.0);
}

fn animate_menu_ring(
    time: Res<Time>,
    config: Res<MenuShellConfig>,
    mut menu: ResMut<MenuAnimation>,
    mut shell: ResMut<MenuShell>,
    mut effects: ResMut<MenuShellEffects>,
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
        hud_transform.translation = Vec3::new(0.0, 0.0, PAGE_RADIUS - HUD_Z_OFFSET_TOWARD_CAMERA);
        hud_transform.scale = Vec3::new(HUD_SCREEN_X_FLIP, 1.0, 1.0);
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
            }
        }
        MenuOpenCloseStyle::OotPageFold => {
            transform.scale = Vec3::ONE;
            transform.translation = Vec3::new(0.0, -0.10 * (1.0 - open), 0.0);
            let fold = OOT_PAGE_FOLD_RADIANS * (1.0 - open);
            for (face, mut t) in &mut face_query {
                apply_oot_open_fold(face.0, fold, &mut t);
            }
        }
    }
}

fn sync_dummy_unpaused_overlay(
    shell: Res<MenuShell>,
    mut overlays: Query<&mut Visibility, With<DummyUnpausedOverlay>>,
) {
    if !shell.is_changed() {
        return;
    }
    for mut visibility in &mut overlays {
        *visibility = if shell.is_visible() { Visibility::Hidden } else { Visibility::Visible };
    }
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn shortest_angle_delta(current: f32, target: f32) -> f32 {
    let two_pi = PI * 2.0;
    (target - current + PI).rem_euclid(two_pi) - PI
}

fn page_pct_to_local(x: f32, y: f32) -> Vec3 {
    Vec3::new((x / 100.0 - 0.5) * PAGE_W, (0.5 - y / 100.0) * PAGE_H, 0.0)
}

fn run_smoke() {
    let mut demo = MockDemo::starter();
    let page = build_page_model(MockPage::Items, &demo, true);
    println!("page: {}", page.title);
    println!("actionable nodes: {}", page.actionable_nodes().count());
    demo.click(MockAction::Item(1));
    assert_eq!(demo.held_item, Some(1));
    demo.click(MockAction::Item(2));
    assert_eq!(demo.held_item, Some(2));
    demo.click(MockAction::Item(2));
    assert_eq!(demo.held_item, None);
    demo.click(MockAction::Item(7));
    assert_eq!(demo.body_item, Some(7));
    demo.click(MockAction::Item(9));
    assert_eq!(demo.body_item, Some(9));
    let before = demo.count(12);
    demo.click(MockAction::Item(12));
    assert_eq!(demo.count(12), before - 1);
    let spec = page_spec(&demo);
    assert_eq!(spec.cells.len(), ITEM_COUNT);
    println!("mock smoke ok");
}
