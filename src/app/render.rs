fn render_overlay_model(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
    model: &MenuPageModel<OotAction>,
) {
    for node in &model.nodes {
        match node {
            MenuNode::Panel { rect, color, action } => spawn_hud_panel(ui, materials, rect.x, rect.y, rect.w, rect.h, menu_color(*color), *action),
            MenuNode::Text { x, y, size, text, align, color } => spawn_hud_text(ui, materials, *x, *y, *size, text, menu_align(*align), menu_srgba(*color)),
            MenuNode::Control { rect, kind, label, detail, icon, selected, important, action } => {
                spawn_hud_control(ui, materials, asset_server, *rect, *kind, label, detail.as_deref(), icon.as_deref(), *selected, *important, *action);
            }
        }
    }
}


fn spawn_hud_control(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
    rect: MenuRect,
    kind: MenuControlKind,
    label: &str,
    detail: Option<&str>,
    icon: Option<&str>,
    selected: bool,
    important: bool,
    action: Option<OotAction>,
) {
    let color = if matches!(action, Some(OotAction::AssignC(_))) {
        Color::srgba(0.92, 0.70, 0.10, 0.96)
    } else if icon.is_some() {
        Color::srgba(1.0, 1.0, 1.0, 0.02)
    } else {
        control_color(kind, selected, important)
    };
    let material = materials.add(StandardMaterial {
        base_color: color,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        unlit: true,
        ..default()
    });
    let mut entity = ui.spawn((
        Name::new(format!("HUD {:?} control", kind)),
        UiLayout::window()
            .x(Rl(rect.x))
            .y(Rl(rect.y))
            .width(Rl(rect.w))
            .height(Rh(rect.h))
            .anchor(Anchor::TOP_LEFT)
            .pack(),
        UiDepth::Set(DEPTH_HUD_PANEL),
        UiMeshPlane3d,
        MeshMaterial3d(material),
        RenderLayers::layer(HUD_RENDER_LAYER),
    ));
    if action.is_some() {
        entity.insert((
            OnHoverSetCursor::new(SystemCursorIcon::Pointer),
            UiHover::new().forward_speed(18.0).backward_speed(10.0),
            UiColor::new(vec![(UiBase::id(), color), (UiHover::id(), hover_panel_color())]),
        ));
    } else {
        entity.insert((UiColor::from(color), Pickable::IGNORE));
    }
    entity.with_children(|children| {
        if let Some(icon_path) = icon {
            spawn_hud_icon(children, materials, asset_server, icon_path);
        }
        if !label.is_empty() {
            spawn_hud_control_text(children, materials, if icon.is_some() { 62.0 } else { 50.0 }, 45.0, 22.0, label, TextAlign::Center, Srgba::rgb_u8(240, 232, 198));
        }
        if let Some(detail) = detail {
            let y = if label.is_empty() { 82.0 } else { 76.0 };
            spawn_hud_control_text(children, materials, if icon.is_some() { 62.0 } else { 50.0 }, y, 10.5, detail, TextAlign::Center, Srgba::rgb_u8(185, 196, 210));
        }
    });
}

fn spawn_hud_icon(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
    icon: &str,
) {
    let texture = asset_server.load(icon.to_string());
    let material = materials.add(StandardMaterial {
        base_color_texture: Some(texture),
        base_color: Color::WHITE,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        unlit: true,
        ..default()
    });
    ui.spawn((
        Name::new(format!("HUD icon {icon}")),
        UiLayout::window().x(Rl(50.0)).y(Rl(50.0)).width(Rl(92.0)).height(Rh(92.0)).anchor(Anchor::CENTER).pack(),
        UiDepth::Set(DEPTH_HUD_ICON),
        UiMeshPlane3d,
        MeshMaterial3d(material),
        Pickable::IGNORE,
        RenderLayers::layer(HUD_RENDER_LAYER),
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

fn spawn_hud_panel(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: Color,
    action: Option<OotAction>,
) {
    let material = materials.add(StandardMaterial { base_color: color, alpha_mode: AlphaMode::Opaque, cull_mode: None, unlit: true, ..default() });
    let depth = if w < 25.0 && h < 2.0 { DEPTH_HUD_ICON } else { DEPTH_HUD_PANEL };
    let mut entity = ui.spawn((
        Name::new("HUD panel"),
        UiLayout::window().x(Rl(x)).y(Rl(y)).width(Rl(w)).height(Rh(h)).anchor(Anchor::TOP_LEFT).pack(),
        UiDepth::Set(depth),
        UiMeshPlane3d,
        MeshMaterial3d(material),
        RenderLayers::layer(HUD_RENDER_LAYER),
    ));
    if action.is_some() {
        entity.insert((OnHoverSetCursor::new(SystemCursorIcon::Pointer), UiHover::new().forward_speed(18.0).backward_speed(10.0), UiColor::new(vec![(UiBase::id(), color), (UiHover::id(), hover_panel_color())])));
    } else {
        entity.insert((UiColor::from(color), Pickable::IGNORE));
    }
}

fn spawn_hud_text(ui: &mut ChildSpawnerCommands, materials: &mut Assets<StandardMaterial>, x: f32, y: f32, size: f32, text: &str, align: TextAlign, color: Srgba) {
    let material = materials.add(StandardMaterial { base_color_texture: Some(TextAtlas::DEFAULT_IMAGE), alpha_mode: AlphaMode::Blend, cull_mode: None, unlit: true, ..default() });
    ui.spawn((
        Name::new("HUD text"),
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

fn render_page_model(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
    model: &MenuPageModel<OotAction>,
) {
    spawn_panel(ui, materials, 0.0, 0.0, 100.0, 100.0, menu_color(model.background), None);
    spawn_cube_edge_frame(ui, materials);
    for node in &model.nodes {
        match node {
            MenuNode::Panel { rect, color, action } => spawn_panel(ui, materials, rect.x, rect.y, rect.w, rect.h, menu_color(*color), *action),
            MenuNode::Text { x, y, size, text, align, color } => spawn_text(ui, materials, *x, *y, *size, text, menu_align(*align), menu_srgba(*color)),
            MenuNode::Control { rect, kind, label, detail, icon, selected, important, action } => {
                spawn_control(ui, materials, asset_server, *rect, *kind, label, detail.as_deref(), icon.as_deref(), *selected, *important, *action);
            }
        }
    }
}

fn spawn_control(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
    rect: MenuRect,
    kind: MenuControlKind,
    label: &str,
    detail: Option<&str>,
    icon: Option<&str>,
    selected: bool,
    important: bool,
    action: Option<OotAction>,
) {
    let disabled = is_disabled_control(kind, action);
    let color = if disabled { disabled_control_color() } else { control_color(kind, selected, important) };
    let material = materials.add(StandardMaterial {
        base_color: color,
        alpha_mode: AlphaMode::Opaque,
        cull_mode: None,
        unlit: true,
        ..default()
    });
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
    ));
    if action.is_some() {
        entity.insert((
            OnHoverSetCursor::new(SystemCursorIcon::Pointer),
            UiHover::new().forward_speed(18.0).backward_speed(10.0),
            UiColor::new(vec![(UiBase::id(), color), (UiHover::id(), hover_panel_color())]),
        ));
    } else {
        entity.insert((UiColor::from(color), Pickable::IGNORE));
    }
    entity.with_children(|children| {
        let icon_is_primary = matches!(kind, MenuControlKind::Item | MenuControlKind::MapMarker | MenuControlKind::Decoration);
        if let Some(icon_path) = icon {
            spawn_icon(children, materials, asset_server, icon_path, icon_is_primary, disabled);
        }
        if selected {
            spawn_selection_corners(children, materials);
        }
        if icon_is_primary {
            if !label.is_empty() {
                spawn_control_text(children, materials, 50.0, 86.0, 14.0, label, TextAlign::Center, Srgba::rgb_u8(240, 232, 198));
            }
            if let Some(detail) = detail {
                spawn_control_text(children, materials, 50.0, 108.0, 10.5, detail, TextAlign::Center, Srgba::rgb_u8(185, 196, 210));
            }
        } else {
            let text_x = if icon.is_some() { 62.0 } else { 50.0 };
            let size = if rect.h < 8.5 { 20.0 } else { 22.0 };
            spawn_control_text(children, materials, text_x, 45.0, size, label, TextAlign::Center, Srgba::rgb_u8(240, 232, 198));
            if let Some(detail) = detail {
                spawn_control_text(children, materials, text_x, 76.0, size * 0.72, detail, TextAlign::Center, Srgba::rgb_u8(185, 196, 210));
            }
        }
    });
}

fn spawn_icon(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
    icon: &str,
    primary: bool,
    disabled: bool,
) {
    // This function is called as a child of a control. Its layout is therefore in
    // control-local percentages, not page percentages. The earlier demo used page
    // coordinates here, which made icons tiny and off-center inside the buttons.
    let icon_size = if primary { 86.0 } else { 58.0 };
    let x = if primary { 50.0 } else { 23.0 };
    let y = if primary { 47.0 } else { 50.0 };
    let texture = asset_server.load(icon.to_string());
    let material = materials.add(StandardMaterial {
        base_color_texture: Some(texture),
        base_color: if disabled { Color::srgba(0.38, 0.38, 0.42, 0.55) } else { Color::WHITE },
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        unlit: true,
        ..default()
    });
    ui.spawn((
        Name::new(format!("Icon {icon}")),
        UiLayout::window()
            .x(Rl(x))
            .y(Rl(y))
            .width(Rl(icon_size))
            .height(Rh(icon_size))
            .anchor(Anchor::CENTER)
            .pack(),
        UiDepth::Set(DEPTH_ICON),
        UiMeshPlane3d,
        MeshMaterial3d(material),
        Pickable::IGNORE,
    ));
}

fn spawn_selection_corners(ui: &mut ChildSpawnerCommands, materials: &mut Assets<StandardMaterial>) {
    let color = Color::WHITE;
    let l = 22.0;
    let t = 5.8;
    // OoT-style focus selection: white square corner brackets. This is separate
    // from the warm fill used for hover/equipped state.
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
        Name::new("OoT selection corner"),
        UiLayout::window().x(Rl(x)).y(Rl(y)).width(Rl(w)).height(Rh(h)).anchor(Anchor::TOP_LEFT).pack(),
        UiDepth::Set(DEPTH_TEXT_TOP - 0.03),
        UiMeshPlane3d,
        MeshMaterial3d(material),
        UiColor::from(color),
        Pickable::IGNORE,
    ));
}

fn spawn_control_text(ui: &mut ChildSpawnerCommands, materials: &mut Assets<StandardMaterial>, x: f32, y: f32, size: f32, text: &str, align: TextAlign, color: Srgba) {
    let material = materials.add(StandardMaterial { base_color_texture: Some(TextAtlas::DEFAULT_IMAGE), alpha_mode: AlphaMode::Blend, cull_mode: None, unlit: true, ..default() });
    ui.spawn((
        Name::new("OoT control text"),
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

fn spawn_cube_edge_frame(ui: &mut ChildSpawnerCommands, materials: &mut Assets<StandardMaterial>) {
    let edge = Color::srgba(0.76, 0.58, 0.24, 0.98);
    spawn_panel_at_depth(ui, materials, 0.0, 0.0, 1.0, 100.0, edge, DEPTH_EDGE);
    spawn_panel_at_depth(ui, materials, 99.0, 0.0, 1.0, 100.0, edge, DEPTH_EDGE);
    spawn_panel_at_depth(ui, materials, 0.0, 0.0, 100.0, 0.8, edge, DEPTH_EDGE);
    spawn_panel_at_depth(ui, materials, 0.0, 99.2, 100.0, 0.8, edge, DEPTH_EDGE);
}

fn spawn_panel(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: Color,
    action: Option<OotAction>,
) {
    let material = materials.add(StandardMaterial { base_color: color, alpha_mode: AlphaMode::Opaque, cull_mode: None, unlit: true, ..default() });
    let mut entity = ui.spawn((
        Name::new("OoT panel"),
        UiLayout::window().x(Rl(x)).y(Rl(y)).width(Rl(w)).height(Rh(h)).anchor(Anchor::TOP_LEFT).pack(),
        UiDepth::Set(panel_depth_at(x, y, w, h, action.is_some())),
        UiMeshPlane3d,
        MeshMaterial3d(material),
    ));
    if action.is_some() {
        entity.insert((OnHoverSetCursor::new(SystemCursorIcon::Pointer), UiHover::new().forward_speed(18.0).backward_speed(10.0), UiColor::new(vec![(UiBase::id(), color), (UiHover::id(), hover_panel_color())])));
    } else {
        entity.insert((UiColor::from(color), Pickable::IGNORE));
    }
}

fn spawn_panel_at_depth(ui: &mut ChildSpawnerCommands, materials: &mut Assets<StandardMaterial>, x: f32, y: f32, w: f32, h: f32, color: Color, depth: f32) {
    let material = materials.add(StandardMaterial { base_color: color, alpha_mode: AlphaMode::Opaque, cull_mode: None, unlit: true, ..default() });
    ui.spawn((
        Name::new("OoT depth panel"),
        UiLayout::window().x(Rl(x)).y(Rl(y)).width(Rl(w)).height(Rh(h)).anchor(Anchor::TOP_LEFT).pack(),
        UiDepth::Set(depth),
        UiMeshPlane3d,
        MeshMaterial3d(material),
        UiColor::from(color),
        Pickable::IGNORE,
    ));
}

fn spawn_text(ui: &mut ChildSpawnerCommands, materials: &mut Assets<StandardMaterial>, x: f32, y: f32, size: f32, text: &str, align: TextAlign, color: Srgba) {
    let material = materials.add(StandardMaterial { base_color_texture: Some(TextAtlas::DEFAULT_IMAGE), alpha_mode: AlphaMode::Blend, cull_mode: None, unlit: true, ..default() });
    ui.spawn((
        Name::new("OoT text"),
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

fn is_disabled_control(kind: MenuControlKind, action: Option<OotAction>) -> bool {
    match action {
        Some(OotAction::Item(idx)) => !oot_items()[idx].usable_by_current_link(),
        Some(OotAction::EquipChoice { slot, choice }) => !equip_slots()[slot].choices[choice].usable_by_current_link(),
        None => matches!(kind, MenuControlKind::Item | MenuControlKind::MapMarker | MenuControlKind::Action),
        _ => false,
    }
}

fn disabled_control_color() -> Color {
    Color::srgba(0.045, 0.045, 0.060, 0.82)
}

fn control_color(kind: MenuControlKind, selected: bool, important: bool) -> Color {
    match kind {
        MenuControlKind::Tab if selected => Color::srgba(0.78, 0.55, 0.20, 0.98),
        MenuControlKind::Tab => Color::srgba(0.10, 0.08, 0.12, 0.95),
        _ => focus_color(selected, important),
    }
}

fn focus_color(selected: bool, important: bool) -> Color {
    match (selected, important) {
        (true, true) => Color::srgba(0.82, 0.58, 0.20, 0.98),
        (true, false) => Color::srgba(0.45, 0.48, 0.68, 0.96),
        (false, true) => Color::srgba(0.18, 0.15, 0.09, 0.94),
        (false, false) => Color::srgba(0.08, 0.08, 0.12, 0.92),
    }
}

fn hover_panel_color() -> Color {
    Color::srgba(0.88, 0.70, 0.28, 0.99)
}

fn panel_depth(w: f32, h: f32, actionable: bool) -> f32 {
    panel_depth_at(0.0, 0.0, w, h, actionable)
}

fn panel_depth_at(x: f32, y: f32, w: f32, h: f32, actionable: bool) -> f32 {
    if actionable {
        return DEPTH_ACTION;
    }
    let area = w * h;
    // Avoid z-fighting between nested non-action panels. Small HUD bars are
    // intentionally biased by position/size so the magic fill and backing never
    // occupy the exact same plane.
    let base = if area > 8_000.0 {
        DEPTH_BACKGROUND
    } else if area > 3_000.0 {
        DEPTH_LARGE_PANEL
    } else if area > 1_200.0 {
        DEPTH_LARGE_PANEL - 0.08
    } else if area > 500.0 {
        DEPTH_CARD
    } else {
        DEPTH_CARD - 0.05
    };
    let stable_bias = ((x * 13.0 + y * 17.0 + w * 19.0 + h * 23.0).round() % 97.0) * 0.00005;
    base - stable_bias
}

fn mc(color: Color) -> MenuColor {
    let srgba = color.to_srgba();
    MenuColor::rgba(srgba.red, srgba.green, srgba.blue, srgba.alpha)
}

fn menu_color(color: MenuColor) -> Color {
    Color::srgba(color.r, color.g, color.b, color.a)
}

fn menu_srgba(color: MenuColor) -> Srgba {
    let r = (color.r.clamp(0.0, 1.0) * 255.0).round() as u8;
    let g = (color.g.clamp(0.0, 1.0) * 255.0).round() as u8;
    let b = (color.b.clamp(0.0, 1.0) * 255.0).round() as u8;
    Srgba::rgb_u8(r, g, b)
}

fn menu_align(align: MenuTextAlign) -> TextAlign {
    match align {
        MenuTextAlign::Left => TextAlign::Left,
        MenuTextAlign::Center => TextAlign::Center,
        MenuTextAlign::Right => TextAlign::Right,
    }
}

