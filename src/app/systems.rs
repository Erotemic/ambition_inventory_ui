fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    demo: Res<OotDemo>,
    readme_capture: Option<Res<ReadmeCapture>>,
) {
    commands.spawn((
        DirectionalLight { illuminance: 2800.0, shadow_maps_enabled: false, ..default() },
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
    if readme_capture.is_none() {
        commands.spawn((
            FpsDebugText,
            Text::new("fps: collecting..."),
            TextFont { font_size: FontSize::Px(14.0), ..default() },
            TextColor(Color::srgba(0.86, 0.95, 0.88, 0.92)),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(8.0),
                ..default()
            },
        ));
    }
    let ring = commands
        .spawn((
            Name::new("OoT-style Lunex pause room"),
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

fn request_readme_capture_frame(
    mut commands: Commands,
    readme_capture: Option<ResMut<ReadmeCapture>>,
) {
    let Some(mut readme_capture) = readme_capture else { return; };
    if readme_capture.is_complete() || readme_capture.waiting_for_capture {
        return;
    }
    if readme_capture.warmup_frames_remaining > 0 {
        readme_capture.warmup_frames_remaining -= 1;
        return;
    }
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(readme_capture.current_frame_path()));
    readme_capture.waiting_for_capture = true;
    readme_capture.capture_started = false;
}

fn advance_readme_capture_frame(
    readme_capture: Option<ResMut<ReadmeCapture>>,
    screenshot_saving: Query<Entity, With<Capturing>>,
    mut app_exit: MessageWriter<AppExit>,
) {
    let Some(mut readme_capture) = readme_capture else { return; };
    if !readme_capture.waiting_for_capture {
        return;
    }
    if !readme_capture.capture_started {
        readme_capture.capture_started = true;
        return;
    }
    if !screenshot_saving.is_empty() || !readme_capture.current_frame_path().exists() {
        return;
    }
    readme_capture.waiting_for_capture = false;
    readme_capture.next_frame += 1;
    if readme_capture.is_complete() {
        app_exit.write(AppExit::Success);
    }
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

fn sync_page_content_visibility(
    demo: Res<OotDemo>,
    mut query: Query<(
        &mut Visibility,
        Option<&NormalPageContent>,
        Option<&SavePromptChoiceContent>,
        Option<&SavePromptCompleteContent>,
    )>,
) {
    let prompt_visible = demo.save_prompt_face_visible();

    for (mut visibility, normal, choice, complete) in &mut query {
        let desired = if let Some(content) = normal {
            if content.0 == demo.page && prompt_visible {
                Visibility::Hidden
            } else {
                Visibility::Visible
            }
        } else if let Some(content) = choice {
            if content.0 == demo.page && prompt_visible && !demo.save_complete {
                Visibility::Visible
            } else {
                Visibility::Hidden
            }
        } else if let Some(content) = complete {
            if content.0 == demo.page && prompt_visible && demo.save_complete {
                Visibility::Visible
            } else {
                Visibility::Hidden
            }
        } else {
            continue;
        };

        visibility.set_if_neq(desired);
    }
}

fn sync_selection_cursors(
    demo: Res<OotDemo>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<&SelectionCursor>,
    mut last_selection: Local<Option<(OotPage, OotAction)>>,
) {
    let selection = (demo.page, demo.selected);
    if *last_selection == Some(selection) {
        return;
    }
    for cursor in &query {
        let desired = if cursor.page == demo.page && cursor.action == demo.selected {
            Color::WHITE
        } else {
            Color::NONE
        };
        set_material_color_if_changed(&mut materials, &cursor.material, desired);
    }
    *last_selection = Some(selection);
}

fn sync_page_status_text(
    demo: Res<OotDemo>,
    mut query: Query<&mut Text3d, With<PageStatusText>>,
    mut last_status: Local<Option<String>>,
) {
    if last_status.as_deref() == Some(demo.status.as_str()) {
        return;
    }
    for mut text in &mut query {
        *text = Text3d::new(demo.status.clone());
    }
    *last_status = Some(demo.status.clone());
}

fn sync_equipment_choice_visuals(
    demo: Res<OotDemo>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<(&EquipmentChoiceVisual, &MeshMaterial3d<StandardMaterial>)>,
    mut last_equipment: Local<Option<[usize; 4]>>,
) {
    let equipment = [
        demo.equipped_sword,
        demo.equipped_shield,
        demo.equipped_tunic,
        demo.equipped_boots,
    ];
    if *last_equipment == Some(equipment) {
        return;
    }
    for (visual, material_handle) in &query {
        let equipped = equipment[visual.slot] == visual.choice;
        let desired = if visual.disabled {
            disabled_control_color()
        } else {
            control_color(MenuControlKind::Item, false, equipped)
        };
        set_material_color_if_changed(&mut materials, &material_handle.0, desired);
    }
    *last_equipment = Some(equipment);
}

fn sync_equipment_preview(
    demo: Res<OotDemo>,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    backing_query: Query<&MeshMaterial3d<StandardMaterial>, With<EquipmentPreviewBacking>>,
    player_query: Query<&MeshMaterial3d<StandardMaterial>, With<EquipmentPlayerPreview>>,
    badge_query: Query<(&EquipmentPreviewBadge, &MeshMaterial3d<StandardMaterial>)>,
    mut text_query: Query<&mut Text3d, With<EquipmentPreviewText>>,
    mut last_equipment: Local<Option<[usize; 4]>>,
) {
    let equipment = [
        demo.equipped_sword,
        demo.equipped_shield,
        demo.equipped_tunic,
        demo.equipped_boots,
    ];
    if *last_equipment == Some(equipment) {
        return;
    }

    for handle in &backing_query {
        set_material_color_if_changed(
            &mut materials,
            &handle.0,
            equipment_preview_backing_color(&demo),
        );
    }
    let player_texture = asset_server.load(equipped_player_icon(&demo).to_string());
    for handle in &player_query {
        set_material_texture_if_changed(&mut materials, &handle.0, player_texture.clone());
    }

    let slots = equip_slots();
    for (badge, handle) in &badge_query {
        let choice = equipment[badge.0];
        let texture = asset_server.load(slots[badge.0].choices[choice].icon.to_string());
        set_material_texture_if_changed(&mut materials, &handle.0, texture);
    }

    let sword = slots[0].choices[equipment[0]];
    let shield = slots[1].choices[equipment[1]];
    let boots = slots[3].choices[equipment[3]];
    let label = format!("{} / {} / {}", sword.name, shield.name, boots.name);
    for mut text in &mut text_query {
        *text = Text3d::new(label.clone());
    }
    *last_equipment = Some(equipment);
}

fn sync_hud_c_icons(
    demo: Res<OotDemo>,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<(&HudActionIcon, &MeshMaterial3d<StandardMaterial>)>,
    mut last_slots: Local<Option<[usize; 3]>>,
) {
    let slots = [demo.c_left, demo.c_down, demo.c_right];
    if *last_slots == Some(slots) {
        return;
    }
    for (icon, material_handle) in &query {
        let OotAction::AssignC(button) = icon.0 else { continue; };
        let item_idx = slots[button.index()];
        let item = oot_items()[item_idx];
        let fallback = match button {
            CButton::Left => "icons/oot/hud_button_c_left.png",
            CButton::Down => "icons/oot/hud_button_c_down.png",
            CButton::Right => "icons/oot/hud_button_c_right.png",
        };
        let path = if item.name.is_empty() { fallback } else { item.icon };
        let texture = asset_server.load(path.to_string());
        set_material_texture_if_changed(&mut materials, &material_handle.0, texture);
    }
    *last_slots = Some(slots);
}

fn sync_equip_animation_visual(
    demo: Res<OotDemo>,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut visual_query: Query<(
        &mut UiLayout,
        &MeshMaterial3d<StandardMaterial>,
    ), With<EquipAnimationVisual>>,
    icon_query: Query<&MeshMaterial3d<StandardMaterial>, With<EquipAnimationIcon>>,
    mut glow_query: Query<&mut Text3d, With<EquipAnimationGlowText>>,
    mut last_glow_label: Local<Option<&'static str>>,
) {
    let Ok((mut layout, panel_material)) = visual_query.single_mut() else { return; };
    let Ok(icon_material) = icon_query.single() else { return; };
    let Ok(mut glow_text) = glow_query.single_mut() else { return; };

    let Some(anim) = demo.equip_anim else {
        set_material_color_if_changed(&mut materials, &panel_material.0, Color::NONE);
        set_material_color_if_changed(&mut materials, &icon_material.0, Color::NONE);
        if *last_glow_label != Some("") {
            *glow_text = Text3d::new("");
            *last_glow_label = Some("");
        }
        return;
    };

    let t = anim.progress.clamp(0.0, 1.0);
    let (pos, icon, label, size) = match anim.phase {
        EquipAnimPhase::ItemToButton => (
            lerp_vec2(anim.from, anim.to, t),
            oot_items()[anim.item_idx].icon,
            "",
            7.0,
        ),
        EquipAnimPhase::ArrowGlowToBow => {
            let kind = arrow_kind(anim.item_idx).unwrap_or(ArrowKind::Fire);
            (
                lerp_vec2(anim.from, anim.via, t),
                kind.glow_icon(),
                "glow",
                7.2 + 1.6 * (1.0 - t),
            )
        }
        EquipAnimPhase::ArrowBowHold => {
            let kind = arrow_kind(anim.item_idx).unwrap_or(ArrowKind::Fire);
            (
                anim.via,
                kind.glow_icon(),
                "glow",
                8.2 + (t * PI * 4.0).sin().abs(),
            )
        }
        EquipAnimPhase::BowToButton => (
            lerp_vec2(anim.via, anim.to, t),
            oot_items()[anim.item_idx].icon,
            "",
            7.0,
        ),
    };

    let desired_layout = UiLayout::window()
        .x(Rl(pos.x - size * 0.5))
        .y(Rl(pos.y - size * 0.5))
        .width(Rl(size))
        .height(Rh(size))
        .anchor(Anchor::TOP_LEFT)
        .pack();
    layout.set_if_neq(desired_layout);

    set_material_color_if_changed(
        &mut materials,
        &panel_material.0,
        Color::srgba(1.0, 1.0, 1.0, 0.02),
    );
    set_material_color_if_changed(&mut materials, &icon_material.0, Color::WHITE);
    let texture = asset_server.load(icon.to_string());
    set_material_texture_if_changed(&mut materials, &icon_material.0, texture);

    if *last_glow_label != Some(label) {
        *glow_text = Text3d::new(label);
        *last_glow_label = Some(label);
    }
}

fn set_material_color_if_changed(
    materials: &mut Assets<StandardMaterial>,
    handle: &Handle<StandardMaterial>,
    desired: Color,
) {
    let needs_update = materials
        .get(handle)
        .is_some_and(|material| material.base_color != desired);
    if needs_update {
        if let Some(mut material) = materials.get_mut(handle) {
            material.base_color = desired;
        }
    }
}

fn set_material_texture_if_changed(
    materials: &mut Assets<StandardMaterial>,
    handle: &Handle<StandardMaterial>,
    desired: Handle<Image>,
) {
    let needs_update = materials
        .get(handle)
        .is_some_and(|material| material.base_color_texture.as_ref() != Some(&desired));
    if needs_update {
        if let Some(mut material) = materials.get_mut(handle) {
            material.base_color_texture = Some(desired);
        }
    }
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
        Transform::from_translation(Vec3::new(0.0, 0.0, PAGE_RADIUS - HUD_Z_OFFSET_TOWARD_CAMERA))
            .with_scale(Vec3::new(HUD_SCREEN_X_FLIP, 1.0, 1.0)),
        Visibility::Visible,
        RenderLayers::layer(HUD_RENDER_LAYER),
    )).with_children(|ui| {
        render_overlay_model(ui, materials, asset_server, &model);
        spawn_equip_animation_visual(ui, materials, asset_server);
    });
}

fn spawn_all_faces(
    ring: &mut ChildSpawnerCommands,
    demo: &OotDemo,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
) {
    for page in OotDemo::pages() {
        spawn_face(ring, page, demo, materials, asset_server);
    }
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
        UiRoot3d,
        UiLayoutRoot::new_3d(),
        Dimension::from((PAGE_W, PAGE_H)),
        Transform::from_translation(translation)
            .with_rotation(rotation)
            .with_scale(Vec3::new(INSIDE_PAGE_X_FLIP, 1.0, 1.0)),
    ));
    face.with_children(|face| {
        let normal_model = build_static_page_model(page, demo);
        let mut normal = face.spawn((
            Name::new(format!("{} normal contents", page.label())),
            NormalPageContent(page),
            UiLayout::window().full().pack(),
            Visibility::Visible,
        ));
        normal.with_children(|ui| {
            render_page_model(ui, materials, asset_server, page, &normal_model);
            if page == OotPage::Equipment {
                spawn_equipment_preview(ui, materials, asset_server, demo);
            }
            spawn_page_status_band(ui, materials, &demo.status);
        });

        let choice_model = build_static_save_prompt_model(false);
        let mut choice = face.spawn((
            Name::new(format!("{} save prompt contents", page.label())),
            SavePromptChoiceContent(page),
            UiLayout::window().full().pack(),
            Visibility::Hidden,
        ));
        choice.with_children(|ui| {
            render_page_model(ui, materials, asset_server, page, &choice_model);
        });

        let complete_model = build_static_save_prompt_model(true);
        let mut complete = face.spawn((
            Name::new(format!("{} saved prompt contents", page.label())),
            SavePromptCompleteContent(page),
            UiLayout::window().full().pack(),
            Visibility::Hidden,
        ));
        complete.with_children(|ui| {
            render_page_model(ui, materials, asset_server, page, &complete_model);
        });
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

fn desired_face_transform(
    page: OotPage,
    active_page: OotPage,
    fold: f32,
    save_flip: f32,
) -> Transform {
    let (base_translation, base_rotation) = page_face_transform(page);
    let fold_rotation = match page {
        OotPage::Items => Quat::from_rotation_x(fold),
        OotPage::Quest => Quat::from_rotation_x(-fold),
        OotPage::Map => Quat::from_rotation_z(-fold),
        OotPage::Equipment => Quat::from_rotation_z(fold),
    };
    let mut rotation = fold_rotation * base_rotation;
    let hinge_local = Vec3::new(0.0, -PAGE_H * 0.5, 0.0);
    let hinge_world = base_translation + base_rotation * hinge_local;
    let translation = hinge_world - rotation * hinge_local;

    if page == active_page && save_flip > 0.001 {
        let t = save_flip.clamp(0.0, 1.0);
        let half = if t < 0.5 { t * 2.0 } else { (1.0 - t) * 2.0 };
        rotation *= Quat::from_rotation_x(FRAC_PI_2 * smoothstep(half));
    }

    Transform::from_translation(translation)
        .with_rotation(rotation)
        .with_scale(Vec3::new(INSIDE_PAGE_X_FLIP, 1.0, 1.0))
}
