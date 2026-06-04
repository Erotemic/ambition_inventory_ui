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
    active_face: bool,
) {
    spawn_panel(ui, materials, MenuRect::new(0.0, 0.0, 100.0, 100.0), menu_color(model.background), None, active_face);
    spawn_cube_edge_frame(ui, materials, active_face);
    for node in &model.nodes {
        match node {
            MenuNode::Panel { rect, color, action } => spawn_panel(ui, materials, *rect, menu_color(*color), *action, active_face),
            MenuNode::Text { x, y, size, text, align, color } => spawn_text(ui, materials, *x, *y, *size, text, menu_align(*align), menu_srgba(*color), active_face),
            MenuNode::Control { rect, kind, label, detail, selected, important, action, .. } => {
                spawn_control(ui, materials, *rect, *kind, label, detail.as_deref(), *selected, *important, *action, active_face);
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
    active_face: bool,
) {
    let disabled = action.is_none();
    let color = if disabled { disabled_control_color() } else { control_color(kind, selected, important) };
    let material = materials.add(StandardMaterial {
        base_color: opaque_color(color),
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
        UiDepth::Set(page_depth(panel_depth(rect, action.is_some()), active_face)),
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
            spawn_selection_corners(children, materials, active_face);
        }
        let main_size = if matches!(kind, MenuControlKind::Item) { 20.0 } else { 22.0 };
        spawn_control_text(children, materials, 50.0, 44.0, main_size, label, TextAlign::Center, Srgba::rgb_u8(242, 234, 200), active_face);
        if let Some(detail) = detail {
            spawn_control_text(children, materials, 50.0, 76.0, 10.5, detail, TextAlign::Center, Srgba::rgb_u8(185, 196, 210), active_face);
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
    active_face: bool,
) {
    spawn_panel_at_depth(ui, materials, rect, color, action, panel_depth(rect, action.is_some()), active_face);
}

fn spawn_panel_at_depth(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    rect: MenuRect,
    color: Color,
    action: Option<MockAction>,
    depth: f32,
    active_face: bool,
) {
    let material = materials.add(StandardMaterial { base_color: opaque_color(color), alpha_mode: AlphaMode::Opaque, cull_mode: None, unlit: true, ..default() });
    let mut entity = ui.spawn((
        Name::new("panel"),
        UiLayout::window().x(Rl(rect.x)).y(Rl(rect.y)).width(Rl(rect.w)).height(Rh(rect.h)).anchor(Anchor::TOP_LEFT).pack(),
        UiDepth::Set(page_depth(depth, active_face)),
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

fn spawn_text(ui: &mut ChildSpawnerCommands, materials: &mut Assets<StandardMaterial>, x: f32, y: f32, size: f32, text: impl Into<String>, align: TextAlign, color: Srgba, active_face: bool) {
    let material = materials.add(StandardMaterial { base_color_texture: Some(TextAtlas::DEFAULT_IMAGE), alpha_mode: AlphaMode::Blend, cull_mode: None, unlit: true, ..default() });
    ui.spawn((
        Name::new("text"),
        UiLayout::window().x(Rl(x)).y(Rl(y)).anchor(Anchor::CENTER).pack(),
        UiDepth::Set(page_depth(text_depth(y), active_face)),
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

fn spawn_control_text(ui: &mut ChildSpawnerCommands, materials: &mut Assets<StandardMaterial>, x: f32, y: f32, size: f32, text: &str, align: TextAlign, color: Srgba, active_face: bool) {
    let material = materials.add(StandardMaterial { base_color_texture: Some(TextAtlas::DEFAULT_IMAGE), alpha_mode: AlphaMode::Blend, cull_mode: None, unlit: true, ..default() });
    ui.spawn((
        Name::new("control text"),
        UiLayout::window().x(Rl(x)).y(Rl(y)).anchor(Anchor::CENTER).pack(),
        UiDepth::Set(page_depth(text_depth(y), active_face)),
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

fn spawn_selection_corners(ui: &mut ChildSpawnerCommands, materials: &mut Assets<StandardMaterial>, active_face: bool) {
    let color = Color::WHITE;
    let l = 23.0;
    let t = 6.0;
    spawn_corner_piece(ui, materials, 0.0, 0.0, l, t, color, active_face);
    spawn_corner_piece(ui, materials, 0.0, 0.0, t, l, color, active_face);
    spawn_corner_piece(ui, materials, 100.0 - l, 0.0, l, t, color, active_face);
    spawn_corner_piece(ui, materials, 100.0 - t, 0.0, t, l, color, active_face);
    spawn_corner_piece(ui, materials, 0.0, 100.0 - t, l, t, color, active_face);
    spawn_corner_piece(ui, materials, 0.0, 100.0 - l, t, l, color, active_face);
    spawn_corner_piece(ui, materials, 100.0 - l, 100.0 - t, l, t, color, active_face);
    spawn_corner_piece(ui, materials, 100.0 - t, 100.0 - l, t, l, color, active_face);
}

fn spawn_corner_piece(ui: &mut ChildSpawnerCommands, materials: &mut Assets<StandardMaterial>, x: f32, y: f32, w: f32, h: f32, color: Color, active_face: bool) {
    let material = materials.add(StandardMaterial { base_color: opaque_color(color), alpha_mode: AlphaMode::Opaque, cull_mode: None, unlit: true, ..default() });
    ui.spawn((
        Name::new("selection corner"),
        UiLayout::window().x(Rl(x)).y(Rl(y)).width(Rl(w)).height(Rh(h)).anchor(Anchor::TOP_LEFT).pack(),
        UiDepth::Set(page_depth(DEPTH_SELECTION, active_face)),
        UiMeshPlane3d,
        MeshMaterial3d(material),
        Pickable::IGNORE,
    ));
}

fn spawn_cube_edge_frame(ui: &mut ChildSpawnerCommands, materials: &mut Assets<StandardMaterial>, active_face: bool) {
    let color = Color::srgba(0.80, 0.92, 1.0, 0.62);
    // Cube borders must not share the large-panel depth. They sit in their own
    // deterministic band, otherwise the border and the page/panel edges shimmer
    // against each other during rotation.
    spawn_panel_at_depth(ui, materials, MenuRect::new(0.0, 0.0, 100.0, 0.7), color, None, DEPTH_EDGE, active_face);
    spawn_panel_at_depth(ui, materials, MenuRect::new(0.0, 99.3, 100.0, 0.7), color, None, DEPTH_EDGE, active_face);
    spawn_panel_at_depth(ui, materials, MenuRect::new(0.0, 0.0, 0.7, 100.0), color, None, DEPTH_EDGE, active_face);
    spawn_panel_at_depth(ui, materials, MenuRect::new(99.3, 0.0, 0.7, 100.0), color, None, DEPTH_EDGE, active_face);
}

fn page_depth(depth: f32, active_face: bool) -> f32 {
    if active_face {
        depth
    } else {
        // Inactive side faces are decorative while the cube is rotating. Keep
        // their child planes close to the physical face so their text/panel
        // layers do not protrude far enough to fight the active face near cube
        // seams. The relative order is preserved, only the extrusion is reduced.
        depth * 0.28
    }
}

fn text_depth(y: f32) -> f32 {
    // Text planes are alpha-blended. Bias them very slightly by their y position
    // so unrelated labels do not share exactly the same transparent sort depth.
    DEPTH_TEXT_TOP - (y.round() % 37.0) * 0.0008
}

fn opaque_color(color: Color) -> Color {
    let s = color.to_srgba();
    Color::srgb(s.red, s.green, s.blue)
}

fn panel_depth(rect: MenuRect, actionable: bool) -> f32 {
    if actionable {
        return DEPTH_ACTION;
    }
    let near_full_page = rect.w > 98.0 && rect.h > 98.0;
    let edge_bar = rect.w < 1.5 || rect.h < 1.5;
    if near_full_page {
        DEPTH_BACKGROUND
    } else if edge_bar {
        DEPTH_EDGE
    } else if rect.w > 40.0 || rect.h > 35.0 {
        DEPTH_LARGE_PANEL
    } else {
        DEPTH_CARD
    }
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
