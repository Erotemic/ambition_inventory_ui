//! Small renderer-neutral menu model used by the demo.

use bevy::prelude::Resource;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MenuRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl MenuRect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MenuColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl MenuColor {
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuTextAlign {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuControlKind {
    Tab,
    Item,
    Action,
    MapMarker,
    Decoration,
}

#[derive(Clone, Debug)]
pub enum MenuNode<Action> {
    Panel {
        rect: MenuRect,
        color: MenuColor,
        action: Option<Action>,
    },
    Text {
        x: f32,
        y: f32,
        size: f32,
        text: String,
        align: MenuTextAlign,
        color: MenuColor,
    },
    Control {
        rect: MenuRect,
        kind: MenuControlKind,
        label: String,
        detail: Option<String>,
        icon: Option<String>,
        selected: bool,
        important: bool,
        action: Option<Action>,
    },
}

#[derive(Clone, Debug)]
pub struct MenuPageModel<Action> {
    pub background: MenuColor,
    pub nodes: Vec<MenuNode<Action>>,
}

impl<Action> MenuPageModel<Action> {
    pub fn new(background: MenuColor) -> Self {
        Self {
            background,
            nodes: Vec::new(),
        }
    }

    pub fn panel(&mut self, rect: MenuRect, color: MenuColor, action: Option<Action>) {
        self.nodes.push(MenuNode::Panel { rect, color, action });
    }

    pub fn text(
        &mut self,
        x: f32,
        y: f32,
        size: f32,
        text: impl Into<String>,
        align: MenuTextAlign,
        color: MenuColor,
    ) {
        self.nodes.push(MenuNode::Text {
            x,
            y,
            size,
            text: text.into(),
            align,
            color,
        });
    }

    #[allow(clippy::too_many_arguments)]
    pub fn control_with_icon<I>(
        &mut self,
        rect: MenuRect,
        kind: MenuControlKind,
        label: impl Into<String>,
        detail: Option<String>,
        icon: Option<I>,
        selected: bool,
        important: bool,
        action: Option<Action>,
    ) where
        I: Into<String>,
    {
        self.nodes.push(MenuNode::Control {
            rect,
            kind,
            label: label.into(),
            detail,
            icon: icon.map(Into::into),
            selected,
            important,
            action,
        });
    }
}

#[derive(Resource, Clone, Debug)]
pub struct MenuShellConfig {
    pub page_rotate_speed: f32,
    pub open_close_speed: f32,
}

impl Default for MenuShellConfig {
    fn default() -> Self {
        Self {
            page_rotate_speed: 5.2,
            open_close_speed: 8.0,
        }
    }
}
