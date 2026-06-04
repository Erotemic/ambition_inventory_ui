fn render_overlay_model(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    model: &MenuPageModel<MockPage, MockAction>,
) {
    for node in &model.nodes {
        match node {
            MenuNode::Panel { rect, color, action } => spawn_hud_panel(ui, materials, *rect, menu_color(*color), *action),
            MenuNode::Text { x, y, size, text, align, color } => spawn_hud_text(ui, materials, *x, *y, *size, text, menu_align(*align), menu_srgba(*color)),
            MenuNode::Control { rect, kind, label, detail, selected, important, action, .. } => {
                spawn_hud_control(ui, materials, *rect, *kind, label, detail.as_deref(), *selected, *important, *action);
            }
        }
    }
}

fn render_page_model(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    model: &MenuPageModel<MockPage, MockAction>,
) {
    spawn_panel(ui, materials, MenuRect::new(0.0, 0.0, 100.0, 100.0), menu_color(model.background), None);
    spawn_cube_edge_frame(ui, materials);
    for node in &model.nodes {
        match node {
            MenuNode::Panel { rect, color, action } => spawn_panel(ui, materials, *rect, menu_color(*color), *action),
            MenuNode::Text { x, y, size, text, align, color } => spawn_text(ui, materials, *x, *y, *size, text, menu_align(*align), menu_srgba(*color)),
            MenuNode::Control { rect, kind, label, detail, selected, important, action, .. } => {
                spawn_control(ui, materials, *rect, *kind, label, detail.as_deref(), *selected, *important, *action);
            }
        }
    }
}

fn spawn_control(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    rect: MenuRect,
    kind: MenuControlKind,
    label: &str,
    detail: Option<&str>,
    selected: bool,
    important: bool,
    action: Option<MockAction>,
) {
    let disabled = action.is_none();
    let color = if disabled { disabled_control_color() } else { control_color(kind, selected, important) };
    let material = materials.add(StandardMaterial {
        base_color: color,
        alpha_mode: AlphaMode::Opaque,
        cull_mode: None,
        unlit: true,
        ..default()
    });
    let focus = MenuFocusKey {
        row: (rect.y * 10.0).round() as i32,
        col: (rect.x * 10.0).round() as i32,
        order: (rect.y * 100.0 + rect.x).round() as i32,
    };
    let mut entity = ui.spawn((
        Name::new(format!("{:?} control", kind)),
        UiLayout::window()
            .x(Rl(rect.x))
            .y(Rl(rect.y))
            .width(Rl(rect.w))
            .height(Rh(rect.h))
            .anchor(Anchor::TOP_LEFT)
            .pack(),
        UiDepth::Set(panel_depth(rect.w, rect.h, action.is_some())),
        UiMeshPlane3d,
        MeshMaterial3d(material),
        AmbitionMenuControl { kind, action, focus },
        MenuVisualState { focused: selected, selected, disabled, ..Default::default() },
    ));
    if action.is_none() {
        entity.insert(Pickable::IGNORE);
    }
    entity.with_children(|children| {
        if selected {
            spawn_selection_corners(children, materials);
        }
        let main_size = if matches!(kind, MenuControlKind::Item) { 20.0 } else { 22.0 };
        spawn_control_text(children, materials, 50.0, 44.0, main_size, label, TextAlign::Center, Srgba::rgb_u8(242, 234, 200));
        if let Some(detail) = detail {
            spawn_control_text(children, materials, 50.0, 76.0, 10.5, detail, TextAlign::Center, Srgba::rgb_u8(185, 196, 210));
        }
    });
}

fn spawn_hud_control(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    rect: MenuRect,
    kind: MenuControlKind,
    label: &str,
    detail: Option<&str>,
    selected: bool,
    important: bool,
    action: Option<MockAction>,
) {
    let color = control_color(kind, selected, important);
    let material = materials.add(StandardMaterial {
        base_color: color,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        unlit: true,
        ..default()
    });
    let mut entity = ui.spawn((
        Name::new(format!("HUD {:?} control", kind)),
        UiLayout::window().x(Rl(rect.x)).y(Rl(rect.y)).width(Rl(rect.w)).height(Rh(rect.h)).anchor(Anchor::TOP_LEFT).pack(),
        UiDepth::Set(DEPTH_HUD_PANEL),
        UiMeshPlane3d,
        MeshMaterial3d(material),
        AmbitionMenuControl { kind, action, focus: MenuFocusKey::default() },
        MenuVisualState { focused: selected, selected, disabled: action.is_none(), ..Default::default() },
        RenderLayers::layer(HUD_RENDER_LAYER),
    ));
    if action.is_none() {
        entity.insert(Pickable::IGNORE);
    }
    entity.with_children(|children| {
        spawn_hud_control_text(children, materials, 50.0, 45.0, 20.0, label, TextAlign::Center, Srgba::rgb_u8(240, 232, 198));
        if let Some(detail) = detail {
            spawn_hud_control_text(children, materials, 50.0, 75.0, 11.0, detail, TextAlign::Center, Srgba::rgb_u8(185, 196, 210));
        }
    });
}

fn spawn_panel(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    rect: MenuRect,
    color: Color,
    action: Option<MockAction>,
) {
    let material = materials.add(StandardMaterial { base_color: color, alpha_mode: AlphaMode::Opaque, cull_mode: None, unlit: true, ..default() });
    let mut entity = ui.spawn((
        Name::new("panel"),
        UiLayout::window().x(Rl(rect.x)).y(Rl(rect.y)).width(Rl(rect.w)).height(Rh(rect.h)).anchor(Anchor::TOP_LEFT).pack(),
        UiDepth::Set(panel_depth(rect.w, rect.h, action.is_some())),
        UiMeshPlane3d,
        MeshMaterial3d(material),
    ));
    if let Some(action) = action {
        entity.insert((
            AmbitionMenuControl { kind: MenuControlKind::Action, action: Some(action), focus: MenuFocusKey::default() },
            MenuVisualState::default(),
        ));
    } else {
        entity.insert(Pickable::IGNORE);
    }
}

fn spawn_hud_panel(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    rect: MenuRect,
    color: Color,
    action: Option<MockAction>,
) {
    let material = materials.add(StandardMaterial { base_color: color, alpha_mode: AlphaMode::Blend, cull_mode: None, unlit: true, ..default() });
    let mut entity = ui.spawn((
        Name::new("HUD panel"),
        UiLayout::window().x(Rl(rect.x)).y(Rl(rect.y)).width(Rl(rect.w)).height(Rh(rect.h)).anchor(Anchor::TOP_LEFT).pack(),
        UiDepth::Set(DEPTH_HUD_PANEL),
        UiMeshPlane3d,
        MeshMaterial3d(material),
        RenderLayers::layer(HUD_RENDER_LAYER),
    ));
    if action.is_none() {
        entity.insert(Pickable::IGNORE);
    }
}

fn spawn_text(ui: &mut ChildSpawnerCommands, materials: &mut Assets<StandardMaterial>, x: f32, y: f32, size: f32, text: impl Into<String>, align: TextAlign, color: Srgba) {
    let material = materials.add(StandardMaterial { base_color_texture: Some(TextAtlas::DEFAULT_IMAGE), alpha_mode: AlphaMode::Blend, cull_mode: None, unlit: true, ..default() });
    ui.spawn((
        Name::new("text"),
        UiLayout::window().x(Rl(x)).y(Rl(y)).anchor(Anchor::CENTER).pack(),
        UiDepth::Set(DEPTH_TEXT_TOP),
        UiTextSize::from(Rh(size)),
        Text3d::new(text.into()),
        Text3dStyling { size: 64.0, color, align, font: Arc::from(FONT_FAMILY), weight: Weight::BOLD, ..Default::default() },
        MeshMaterial3d(material),
        Mesh3d::default(),
        Pickable::IGNORE,
    ));
}

fn spawn_hud_text(ui: &mut ChildSpawnerCommands, materials: &mut Assets<StandardMaterial>, x: f32, y: f32, size: f32, text: impl Into<String>, align: TextAlign, color: Srgba) {
    let material = materials.add(StandardMaterial { base_color_texture: Some(TextAtlas::DEFAULT_IMAGE), alpha_mode: AlphaMode::Blend, cull_mode: None, unlit: true, ..default() });
    ui.spawn((
        Name::new("HUD text"),
        UiLayout::window().x(Rl(x)).y(Rl(y)).anchor(Anchor::CENTER).pack(),
        UiDepth::Set(DEPTH_HUD_TEXT),
        UiTextSize::from(Rh(size)),
        Text3d::new(text.into()),
        Text3dStyling { size: 64.0, color, align, font: Arc::from(FONT_FAMILY), weight: Weight::BOLD, ..Default::default() },
        MeshMaterial3d(material),
        Mesh3d::default(),
        Pickable::IGNORE,
        RenderLayers::layer(HUD_RENDER_LAYER),
    ));
}

fn spawn_control_text(ui: &mut ChildSpawnerCommands, materials: &mut Assets<StandardMaterial>, x: f32, y: f32, size: f32, text: &str, align: TextAlign, color: Srgba) {
    let material = materials.add(StandardMaterial { base_color_texture: Some(TextAtlas::DEFAULT_IMAGE), alpha_mode: AlphaMode::Blend, cull_mode: None, unlit: true, ..default() });
    ui.spawn((
        Name::new("control text"),
        UiLayout::window().x(Rl(x)).y(Rl(y)).anchor(Anchor::CENTER).pack(),
        UiDepth::Set(DEPTH_TEXT_TOP),
        UiTextSize::from(Rh(size)),
        Text3d::new(text),
        Text3dStyling { size: 64.0, color, align, font: Arc::from(FONT_FAMILY), weight: Weight::BOLD, ..Default::default() },
        MeshMaterial3d(material),
        Mesh3d::default(),
        Pickable::IGNORE,
    ));
}

fn spawn_hud_control_text(ui: &mut ChildSpawnerCommands, materials: &mut Assets<StandardMaterial>, x: f32, y: f32, size: f32, text: &str, align: TextAlign, color: Srgba) {
    let material = materials.add(StandardMaterial { base_color_texture: Some(TextAtlas::DEFAULT_IMAGE), alpha_mode: AlphaMode::Blend, cull_mode: None, unlit: true, ..default() });
    ui.spawn((
        Name::new("HUD control text"),
        UiLayout::window().x(Rl(x)).y(Rl(y)).anchor(Anchor::CENTER).pack(),
        UiDepth::Set(DEPTH_HUD_TEXT),
        UiTextSize::from(Rh(size)),
        Text3d::new(text),
        Text3dStyling { size: 64.0, color, align, font: Arc::from(FONT_FAMILY), weight: Weight::BOLD, ..Default::default() },
        MeshMaterial3d(material),
        Mesh3d::default(),
        Pickable::IGNORE,
        RenderLayers::layer(HUD_RENDER_LAYER),
    ));
}

fn spawn_selection_corners(ui: &mut ChildSpawnerCommands, materials: &mut Assets<StandardMaterial>) {
    let color = Color::WHITE;
    let l = 23.0;
    let t = 6.0;
    spawn_corner_piece(ui, materials, 0.0, 0.0, l, t, color);
    spawn_corner_piece(ui, materials, 0.0, 0.0, t, l, color);
    spawn_corner_piece(ui, materials, 100.0 - l, 0.0, l, t, color);
    spawn_corner_piece(ui, materials, 100.0 - t, 0.0, t, l, color);
    spawn_corner_piece(ui, materials, 0.0, 100.0 - t, l, t, color);
    spawn_corner_piece(ui, materials, 0.0, 100.0 - l, t, l, color);
    spawn_corner_piece(ui, materials, 100.0 - l, 100.0 - t, l, t, color);
    spawn_corner_piece(ui, materials, 100.0 - t, 100.0 - l, t, l, color);
}

fn spawn_corner_piece(ui: &mut ChildSpawnerCommands, materials: &mut Assets<StandardMaterial>, x: f32, y: f32, w: f32, h: f32, color: Color) {
    let material = materials.add(StandardMaterial { base_color: color, alpha_mode: AlphaMode::Opaque, cull_mode: None, unlit: true, ..default() });
    ui.spawn((
        Name::new("selection corner"),
        UiLayout::window().x(Rl(x)).y(Rl(y)).width(Rl(w)).height(Rh(h)).anchor(Anchor::TOP_LEFT).pack(),
        UiDepth::Set(DEPTH_EDGE),
        UiMeshPlane3d,
        MeshMaterial3d(material),
        Pickable::IGNORE,
    ));
}

fn spawn_cube_edge_frame(ui: &mut ChildSpawnerCommands, materials: &mut Assets<StandardMaterial>) {
    let color = Color::srgba(0.80, 0.92, 1.0, 0.62);
    spawn_panel(ui, materials, MenuRect::new(0.0, 0.0, 100.0, 0.7), color, None);
    spawn_panel(ui, materials, MenuRect::new(0.0, 99.3, 100.0, 0.7), color, None);
    spawn_panel(ui, materials, MenuRect::new(0.0, 0.0, 0.7, 100.0), color, None);
    spawn_panel(ui, materials, MenuRect::new(99.3, 0.0, 0.7, 100.0), color, None);
}

fn panel_depth(w: f32, h: f32, actionable: bool) -> f32 {
    if actionable { DEPTH_ACTION } else if w > 40.0 || h > 35.0 { DEPTH_LARGE_PANEL } else { DEPTH_CARD }
}

fn control_color(kind: MenuControlKind, selected: bool, important: bool) -> Color {
    if selected {
        Color::srgba(0.98, 0.76, 0.26, 0.96)
    } else if important {
        Color::srgba(0.13, 0.34, 0.28, 0.96)
    } else {
        match kind {
            MenuControlKind::Item => Color::srgba(0.055, 0.074, 0.155, 0.96),
            MenuControlKind::Scrollbar => Color::srgba(0.42, 0.32, 0.08, 0.92),
            MenuControlKind::Action => Color::srgba(0.09, 0.12, 0.26, 0.96),
            _ => Color::srgba(0.055, 0.070, 0.145, 0.96),
        }
    }
}

fn disabled_control_color() -> Color {
    Color::srgba(0.040, 0.045, 0.075, 0.72)
}

fn menu_color(color: MenuColor) -> Color {
    Color::srgba(color.r, color.g, color.b, color.a)
}

fn menu_srgba(color: MenuColor) -> Srgba {
    Srgba::new(color.r, color.g, color.b, color.a)
}

fn menu_align(align: MenuTextAlign) -> TextAlign {
    match align {
        MenuTextAlign::Left => TextAlign::Left,
        MenuTextAlign::Center => TextAlign::Center,
        MenuTextAlign::Right => TextAlign::Right,
    }
}
