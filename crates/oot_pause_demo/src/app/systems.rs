fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    demo: Res<OotDemo>,
) {
    commands.spawn((
        DirectionalLight { illuminance: 2800.0, shadows_enabled: false, ..default() },
        Transform::from_xyz(1.5, 3.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    let mut cube_camera = commands.spawn((
        Name::new("OoT pause cube camera"),
        MainPauseCamera,
        Camera3d::default(),
        Camera { order: 0, ..default() },
        RenderLayers::layer(0),
        Msaa::Off,
        Transform::from_translation(CAMERA_EYE).looking_at(CAMERA_LOOK, Vec3::Y),
    ));
    // The idle flamegraph points at steady-state Bevy PBR/render-view work, not
    // Lunex layout itself. Keep the costly post-processing/transparent sorting
    // features opt-in while profiling; the demo art uses mostly opaque quads and
    // stable depth bands, so OIT/FXAA are not required for correctness.
    if std::env::var_os("OOT_ENABLE_OIT").is_some() {
        cube_camera.insert(OrderIndependentTransparencySettings::default());
    }
    if std::env::var_os("OOT_ENABLE_FXAA").is_some() {
        cube_camera.insert(Fxaa::default());
    }
    let mut hud_camera = commands.spawn((
        Name::new("OoT pause HUD overlay camera"),
        Camera3d::default(),
        Camera {
            order: 10,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        Msaa::Off,
        RenderLayers::layer(HUD_RENDER_LAYER),
        Transform::from_translation(CAMERA_EYE).looking_at(CAMERA_LOOK, Vec3::Y),
    ));
    if std::env::var_os("OOT_ENABLE_FXAA").is_some() {
        hud_camera.insert(Fxaa::default());
    }
    commands.spawn((
        FpsDebugText,
        Text::new("fps: collecting..."),
        TextFont { font_size: 14.0, ..default() },
        TextColor(Color::srgba(0.86, 0.95, 0.88, 0.92)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            top: Val::Px(8.0),
            ..default()
        },
    ));
    let ring = commands
        .spawn((
            Name::new("OoT-style Lunex pause room"),
            AmbitionMenuRoot,
            MenuRing,
            UiRoot3d,
            Transform::default(),
            Visibility::Visible,
            RenderLayers::layer(0),
        ))
        .id();
    commands.entity(ring).with_children(|ring| {
        spawn_all_faces(ring, &demo, &mut materials, &asset_server);
    });
    spawn_hud_overlay(&mut commands, &demo, &mut materials, &asset_server);
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
    asset_server: Res<AssetServer>,
    demo: Res<OotDemo>,
    ring_query: Query<Entity, With<MenuRing>>,
    face_query: Query<(Entity, &PageFace), With<LunexFaceRoot>>,
    hud_query: Query<Entity, With<HudOverlayRoot>>,
    mut last_revision: Local<Option<u64>>,
    mut last_page: Local<Option<OotPage>>,
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
        commands.entity(ring).with_children(|ring| spawn_all_faces(ring, &demo, &mut materials, &asset_server));
    } else {
        for (entity, face) in &face_query {
            if face.0 == demo.page {
                commands.entity(entity).despawn();
            }
        }
        commands.entity(ring).with_children(|ring| spawn_face(ring, demo.page, &demo, &mut materials, &asset_server));
    }
    for entity in &hud_query {
        commands.entity(entity).despawn();
    }
    spawn_hud_overlay(&mut commands, &demo, &mut materials, &asset_server);
    *last_revision = Some(demo.revision);
    *last_page = Some(demo.page);
}

fn spawn_hud_overlay(
    commands: &mut Commands,
    demo: &OotDemo,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
) {
    let model = build_pause_hud_model(demo);
    commands.spawn((
        Name::new("OoT pause HUD overlay"),
        HudOverlayRoot,
        UiRoot3d,
        UiLayoutRoot::new_3d(),
        Dimension::from((PAGE_W, PAGE_H)),
        // The HUD is not a child of MenuRing, so it does not rotate with the
        // cube or with the save-prompt flip. It sits just in front of the active
        // face. Because the pause camera is viewing the inside/back side of the
        // page plane, raw local +X projects as visual-left; keep HUD models
        // authored in normal screen coordinates and flip the overlay root once.
        Transform::from_translation(Vec3::new(0.0, 0.0, PAGE_RADIUS - HUD_Z_OFFSET_TOWARD_CAMERA))
            .with_scale(Vec3::new(HUD_SCREEN_X_FLIP, 1.0, 1.0)),
        Visibility::Visible,
        RenderLayers::layer(HUD_RENDER_LAYER),
    )).with_children(|ui| render_overlay_model(ui, materials, asset_server, &model));
}


fn tag_hud_render_layers(
    mut commands: Commands,
    hud_roots: Query<Entity, With<HudOverlayRoot>>,
    children_query: Query<&Children>,
    unlayered: Query<Entity, Without<RenderLayers>>,
) {
    for root in &hud_roots {
        tag_hud_entity_recursive(root, &mut commands, &children_query, &unlayered);
    }
}

fn tag_hud_entity_recursive(
    entity: Entity,
    commands: &mut Commands,
    children_query: &Query<&Children>,
    unlayered: &Query<Entity, Without<RenderLayers>>,
) {
    if unlayered.get(entity).is_ok() {
        commands.entity(entity).insert(RenderLayers::layer(HUD_RENDER_LAYER));
    }
    if let Ok(children) = children_query.get(entity) {
        for child in children.iter() {
            tag_hud_entity_recursive(child, commands, children_query, unlayered);
        }
    }
}

fn spawn_all_faces(
    ring: &mut ChildSpawnerCommands,
    demo: &OotDemo,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
) {
    // Only three faces can be visible in the inside-the-cube camera: active,
    // viewer-left, and viewer-right. The back/opposite face still contributed a
    // full set of PBR mesh/material entities and showed up as steady-state render
    // overhead in the idle flamegraph, so do not keep it alive.
    for page in visible_face_pages(demo.page) {
        spawn_face(ring, page, demo, materials, asset_server);
    }
}

fn visible_face_pages(active: OotPage) -> [OotPage; 3] {
    [
        OotDemo::page_on_viewer_left(active),
        active,
        OotDemo::page_on_viewer_right(active),
    ]
}

fn spawn_face(
    ring: &mut ChildSpawnerCommands,
    page: OotPage,
    demo: &OotDemo,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
) {
    let (translation, rotation) = page_face_transform(page);
    let mut face = ring.spawn((
        Name::new(format!("{} OoT face", page.label())),
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
        let active_face = page == demo.page;
        let model = build_page_model(page, demo, active_face);
        render_page_model(ui, materials, asset_server, &model);
    });
}

fn page_face_transform(page: OotPage) -> (Vec3, Quat) {
    match page {
        OotPage::Items => (Vec3::new(0.0, 0.0, PAGE_RADIUS), Quat::IDENTITY),
        OotPage::Map => (Vec3::new(PAGE_RADIUS, 0.0, 0.0), Quat::from_rotation_y(FRAC_PI_2)),
        OotPage::Quest => (Vec3::new(0.0, 0.0, -PAGE_RADIUS), Quat::from_rotation_y(PI)),
        OotPage::Equipment => (Vec3::new(-PAGE_RADIUS, 0.0, 0.0), Quat::from_rotation_y(-FRAC_PI_2)),
    }
}

fn reset_face_transform(page: OotPage, transform: &mut Transform) {
    let (translation, rotation) = page_face_transform(page);
    transform.translation = translation;
    transform.rotation = rotation;
    transform.scale = Vec3::new(INSIDE_PAGE_X_FLIP, 1.0, 1.0);
}

fn apply_oot_open_fold(page: OotPage, fold: f32, transform: &mut Transform) {
    let (base_translation, base_rotation) = page_face_transform(page);
    // Matches the source transform idea: pages are fixed around the origin,
    // fold around their lower edge, and side pages use Z-pitch before their Y-facing rotation.
    let fold_rotation = match page {
        OotPage::Items => Quat::from_rotation_x(fold),
        OotPage::Quest => Quat::from_rotation_x(-fold),
        OotPage::Map => Quat::from_rotation_z(-fold),
        OotPage::Equipment => Quat::from_rotation_z(fold),
    };
    let rotation = fold_rotation * base_rotation;
    let hinge_local = Vec3::new(0.0, -PAGE_H * 0.5, 0.0);
    let hinge_world = base_translation + base_rotation * hinge_local;
    let translation = hinge_world - rotation * hinge_local;
    transform.translation = translation;
    transform.rotation = rotation;
    transform.scale = Vec3::new(INSIDE_PAGE_X_FLIP, 1.0, 1.0);
}

