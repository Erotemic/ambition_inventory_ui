// App shell for the standalone mock demo. The cube itself (camera, ring, faces,
// fold, rotation, page rendering) lives in `ambition_inventory_ui::cube` and is
// driven via `ActiveMenuPages` + `CubeOpenState`. Everything here is app-only:
// the HUD overlay, the FPS counter, the dummy-unpaused banner, and the bridge
// from the demo's `MenuShell` to the lib's `CubeOpenState`.

fn setup_app_shell(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    demo: Res<MockDemo>,
    shell: Res<MenuShell>,
) {
    commands.spawn((
        DirectionalLight { illuminance: 2800.0, shadows_enabled: false, ..default() },
        Transform::from_xyz(1.5, 3.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    // App-only HUD overlay camera (its own render layer). The cube's pause camera
    // is spawned by the lib plugin.
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

/// Rebuild the app-only HUD overlay when the demo or shell changes.
fn rebuild_hud_overlay(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    demo: Res<MockDemo>,
    shell: Res<MenuShell>,
    hud_query: Query<Entity, With<HudOverlayRoot>>,
    mut last_revision: Local<Option<u64>>,
) {
    if !shell.is_changed() && *last_revision == Some(demo.revision) {
        return;
    }
    *last_revision = Some(demo.revision);
    for entity in &hud_query {
        commands.entity(entity).despawn();
    }
    spawn_hud_overlay(&mut commands, &demo, &shell, &mut materials);
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
        if shell.is_visible() { Visibility::Visible } else { Visibility::Hidden },
        RenderLayers::layer(HUD_RENDER_LAYER),
    )).with_children(|ui| render_overlay_model(ui, materials, &model));
}

/// Bridge the demo's `MenuShell` to the lib's cube: ease the open target, push
/// shell lifecycle effects, and toggle the HUD overlay visibility. The lib owns
/// the ring rotation + fold; this only feeds it the open target and reads back
/// the eased amount to gate the HUD.
fn drive_cube_open(
    mut open_state: ResMut<CubeOpenState>,
    mut shell: ResMut<MenuShell>,
    mut effects: ResMut<MenuShellEffects>,
    mut last_phase: Local<Option<MenuShellPhase>>,
    mut hud_query: Query<&mut Visibility, With<HudOverlayRoot>>,
) {
    // Feed the open target to the lib (it eases CubeOpenState.amount toward it).
    open_state.target = if shell.target_open { 1.0 } else { 0.0 };
    // Mirror the eased amount back into the shell so phase()/is_visible() track
    // the lib's animation.
    shell.openness = open_state.amount;
    let visible = shell.is_visible();
    for mut hud_visibility in &mut hud_query {
        *hud_visibility = if visible { Visibility::Visible } else { Visibility::Hidden };
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
