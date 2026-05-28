//! Reusable data model for the Ambition-style Lunex cube menu prototype.
//!
//! The renderer in `main.rs` is still a demo, but it now consumes this data
//! model instead of hard-coding each visual panel directly in the draw path.
//! Host games can build these specs from their own inventory, map, quest, or
//! settings resources and then translate `MenuAction` requests back into game
//! events.

use bevy::prelude::{Component, Plugin, Resource};

/// A normalized page-space rectangle.
///
/// Coordinates are percentages in the page's local 2D layout space. `(0, 0)` is
/// the top-left corner and `(100, 100)` is the bottom-right corner.
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

/// Renderer-independent color token.
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

/// Text alignment independent of the concrete renderer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuTextAlign {
    Left,
    Center,
    Right,
}

/// Broad semantic class for controls.
///
/// A renderer may style these differently, and a navigation policy may use this
/// to decide whether a control participates in focus, hover, or scroll. Renderers
/// may also use this to choose default icon size/placement for controls that set
/// an icon asset path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuControlKind {
    Tab,
    Slot,
    Item,
    Action,
    PopupAction,
    OptionToggle,
    OptionChoice,
    MapMarker,
    Scrollbar,
    PopupPanel,
    Decoration,
}

/// A single page node. `Action` is intentionally generic so games can use their
/// own enum instead of stringly typed callbacks.
///
/// `Control::icon` is an optional asset path, relative to Bevy's asset root.
/// This keeps content data-driven while allowing renderers to show sprite icons
/// beside buttons, tabs, item cards, or option rows.
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

/// Full data description for one visible page/face of the cube menu.
#[derive(Clone, Debug)]
pub struct MenuPageModel<PageId, Action> {
    pub id: PageId,
    pub title: String,
    pub background: MenuColor,
    pub nodes: Vec<MenuNode<Action>>,
}

impl<PageId, Action> MenuPageModel<PageId, Action> {
    pub fn new(id: PageId, title: impl Into<String>, background: MenuColor) -> Self {
        Self {
            id,
            title: title.into(),
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

    pub fn control(
        &mut self,
        rect: MenuRect,
        kind: MenuControlKind,
        label: impl Into<String>,
        detail: Option<String>,
        selected: bool,
        important: bool,
        action: Option<Action>,
    ) {
        self.control_with_icon(rect, kind, label, detail, None::<String>, selected, important, action);
    }

    pub fn control_with_icon(
        &mut self,
        rect: MenuRect,
        kind: MenuControlKind,
        label: impl Into<String>,
        detail: Option<String>,
        icon: Option<impl Into<String>>,
        selected: bool,
        important: bool,
        action: Option<Action>,
    ) {
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

/// Host-facing lifecycle/effect hook. The demo queues these events; an Ambition
/// integration can drain the queue to play sfx, pause gameplay, or muffle music.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuShellEffect {
    Opening,
    Opened,
    Closing,
    Closed,
    PageChanged,
    Navigate,
    Activate,
    Cancel,
}

/// Queue of shell effects generated by the menu module.
///
/// This intentionally avoids hard-coding audio or music behavior into the UI.
/// A game can map `Opening`/`Closing` to sounds and map `Opened`/`Closed` to
/// music ducking or gameplay mode changes.
#[derive(Resource, Default, Clone, Debug)]
pub struct MenuShellEffects {
    pub pending: Vec<MenuShellEffect>,
}

impl MenuShellEffects {
    pub fn push(&mut self, effect: MenuShellEffect) {
        self.pending.push(effect);
    }

    pub fn drain(&mut self) -> impl Iterator<Item = MenuShellEffect> + '_ {
        self.pending.drain(..)
    }
}

/// Coarse lifecycle phase derived from a shell's openness and target state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuShellPhase {
    Closed,
    Opening,
    Open,
    Closing,
}

/// Configurable touch policy for game-friendly menus.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TouchActivationPolicy {
    ActivateOnFirstTap,
    SelectThenTap,
}

/// Pointer/touch gesture affordances supported by the menu shell.
///
/// These are policy flags rather than hard-coded behavior so host games can
/// keep desktop, controller, and mobile interaction conventions aligned.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MenuGesturePolicy {
    pub swipe_pages: bool,
    pub drag_off_cancels: bool,
    pub drag_scroll_panes: bool,
}

impl Default for MenuGesturePolicy {
    fn default() -> Self {
        Self {
            swipe_pages: true,
            drag_off_cancels: true,
            drag_scroll_panes: true,
        }
    }
}


/// Optional plugin marker for host games that want a single import point.
///
/// The prototype renderer still lives in `main.rs`, but these public types are
/// intentionally crate-shaped. A future extraction can move the Lunex renderer,
/// input systems, and shell animation systems behind this plugin without
/// changing the data model below.
pub struct AmbitionInventoryUiPlugin;

impl Plugin for AmbitionInventoryUiPlugin {
    fn build(&self, _app: &mut bevy::prelude::App) {
        // Intentionally empty in this prototype overlay. The reusable API is
        // already available through ECS components/resources and the data model;
        // renderer systems will move here once the demo hardening settles.
    }
}

/// High-level shell animation style.
///
/// Keep the nostalgic OoT-inspired page fold opt-in at the reusable API level.
/// Most games should start with `SmoothScale` and deliberately choose
/// `OotPageFold` when they want the strong N64 pause-menu identity.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MenuOpenCloseStyle {
    #[default]
    SmoothScale,
    OotPageFold,
}


/// Reusable selection rendering hint.
///
/// The default package renderer may interpret this as a fill, outline, or
/// corner-bracket effect. The OoT demo uses `CornerBrackets` to separate
/// keyboard/gamepad selection from transient hover color.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MenuSelectionEffect {
    Fill,
    Outline,
    CornerBrackets { corner_len_pct: f32, thickness_pct: f32 },
}

impl Default for MenuSelectionEffect {
    fn default() -> Self {
        Self::CornerBrackets { corner_len_pct: 24.0, thickness_pct: 4.0 }
    }
}

/// Cube/page geometry shared by renderers that want an OoT-like four-page room.
///
/// `page_width = 2 * page_radius` is the important source-derived relationship:
/// OoT's page background width is effectively twice the page depth, which makes
/// adjacent faces meet at visible cube edges instead of floating apart.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MenuCubeGeometry {
    pub page_radius: f32,
    pub page_width: f32,
    pub page_height: f32,
    pub camera_distance: f32,
    pub camera_y: f32,
    pub look_y: f32,
}

impl MenuCubeGeometry {
    pub const fn oot_like(page_radius: f32) -> Self {
        let page_width = page_radius * 2.0;
        Self {
            page_radius,
            page_width,
            page_height: page_width * (160.0 / 240.0),
            camera_distance: page_radius * 0.80,
            camera_y: 0.0,
            look_y: 0.0,
        }
    }
}

impl Default for MenuCubeGeometry {
    fn default() -> Self {
        Self::oot_like(2.85)
    }
}

/// Configuration resource for a reusable menu shell.
///
/// This intentionally avoids audio/music decisions. Instead, use
/// `MenuShellEffects` and let the host game map lifecycle events to SFX,
/// pause-state changes, or music ducking/muffling.
#[derive(Resource, Clone, Debug)]
pub struct MenuShellConfig {
    pub open_close_style: MenuOpenCloseStyle,
    pub touch_policy: TouchActivationPolicy,
    pub gestures: MenuGesturePolicy,
    pub page_rotate_speed: f32,
    pub open_close_speed: f32,
    pub selection_effect: MenuSelectionEffect,
    pub cube_geometry: MenuCubeGeometry,
}

impl Default for MenuShellConfig {
    fn default() -> Self {
        Self {
            open_close_style: MenuOpenCloseStyle::SmoothScale,
            touch_policy: TouchActivationPolicy::SelectThenTap,
            gestures: MenuGesturePolicy::default(),
            page_rotate_speed: 5.2,
            open_close_speed: 8.0,
            selection_effect: MenuSelectionEffect::default(),
            cube_geometry: MenuCubeGeometry::default(),
        }
    }
}

/// Marker for the root entity that owns a menu shell / menu room.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AmbitionMenuRoot;

/// ECS component attached to a rendered menu page/face.
#[derive(Component, Clone, Debug, Eq, PartialEq)]
pub struct AmbitionMenuPage<PageId> {
    pub id: PageId,
    pub active: bool,
}

/// Stable navigation identity for focusable controls.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub struct MenuFocusKey {
    pub row: i32,
    pub col: i32,
    pub order: i32,
}

/// ECS component attached to rendered controls.
///
/// The data-driven builder is still the ergonomic API, but controls that make
/// it into the world should carry their semantic action/kind as components so
/// hover, focus, accessibility, and alternative input can be implemented by ECS
/// systems instead of by renderer-private bookkeeping.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct AmbitionMenuControl<Action> {
    pub kind: MenuControlKind,
    pub action: Option<Action>,
    pub focus: MenuFocusKey,
}

/// Runtime visual state for a control.
///
/// This is the part that belongs in ECS. It changes frequently from hover,
/// focus, touch, and gamepad navigation, while the declarative page data can
/// remain stable and data-driven.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MenuVisualState {
    pub hovered: bool,
    pub focused: bool,
    pub selected: bool,
    pub pressed: bool,
    pub disabled: bool,
}

/// ECS metadata for a scrollable viewport.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MenuScrollPane {
    pub first_visible: usize,
    pub visible_rows: usize,
    pub total_rows: usize,
}
