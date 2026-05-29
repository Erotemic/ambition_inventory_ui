# OoT Functional Pause Demo Design Document

## 1. Purpose

The OoT Functional Pause Demo is a separate demo crate that proves the reusable `ambition_inventory_ui` system can reproduce a game-feeling, Ocarina-of-Time-inspired pause menu using original placeholder assets and reusable UI primitives.

The demo is not intended to become the core API. It is a stress test and reference implementation for:

* inside-the-cube page navigation,
* source-inspired page layouts,
* controller-first menu behavior,
* item-to-C-button assignment,
* Adult Link item restrictions,
* OoT-like selection feedback,
* OoT-like save prompt flow,
* HUD-like button/meter overlays,
* page open/close and prompt-flip animations.

The demo should feel functionally similar to OoT while using original generated/demo artwork.

## 2. Crate Boundary

The OoT demo must live in its own workspace crate.

Recommended crate path:

```text
crates/oot_pause_demo
```

The reusable package remains the root crate:

```text
ambition_inventory_ui
```

The OoT demo may hard-code OoT-like item data, page layouts, and behaviors. The root crate should only receive reusable abstractions after those abstractions have stabilized.

## 3. Core Design Principles

### 3.1 Source-inspired, not asset-exact

The demo should use the OoT source/decomp code as the behavioral and layout reference, especially for:

* four-page pause structure,
* item page slot ordering,
* equipment page structure,
* quest status icon arrangement,
* item-to-C-button assignment behavior,
* save prompt state flow,
* page/prompt animation concepts.

The demo must not use original OoT art assets. It should use generated or original placeholder icons.

### 3.2 Functional accuracy over pixel accuracy

The goal is not a perfect visual clone. The goal is that a player familiar with OoT recognizes the functional behavior:

* page rotation feels like the OoT pause menu,
* item grid behaves like the OoT item menu,
* C-button assignment works like OoT,
* save prompt works like OoT,
* disabled Adult Link items behave like OoT,
* selection/activation states are clear and familiar.

### 3.3 The demo is an integration proof

The demo should prove that the core UI system can support:

* non-application-style game menus,
* page-shell effects,
* 3D transformed UI surfaces,
* modal prompt states,
* focusable vs actionable distinctions,
* data-driven page contents,
* item assignment animations,
* keyboard, mouse, gamepad, and eventually touch.

## 4. High-Level User Experience

The demo opens into an OoT-like pause menu composed of four cube faces. The viewer is inside the menu cube looking outward at the active face.

The player can rotate between four pages:

1. Items
2. Equipment
3. Map
4. Quest Status

The active face is mostly flat-on to the viewer. Neighboring faces should be visible at the edges, giving the impression of an enclosing cube. The current page should not be pitched backward just to reveal edges.

The menu has a HUD overlay with:

* health hearts,
* magic meter,
* red Start button,
* C-button cluster,
* green B button,
* blue A button.

The Items page supports selecting an item and assigning it to C-left, C-down, or C-right. Assignment should animate by moving the item icon from its item grid slot to the target C-button.

Pressing B opens a save prompt. The current pane flips into a prompt that asks whether the player wants to save.

## 5. Page Shell and Cube Navigation

### 5.1 Four cube faces

The demo uses four page faces arranged as an inside-facing cube room.

Required pages:

```text
Items
Equipment
Map
Quest Status
```

Each page is rendered as a Lunex face. The face contents should transform together as a surface.

### 5.2 Inside-the-cube mental model

The player is inside the cube. The current page is in front of the viewer. Neighboring pages are on the left and right cube walls. The top edge of the cube may be subtly visible if geometry permits.

The menu should not feel like detached planes floating in front of the camera. Face edges should feel connected.

### 5.3 Page direction invariant

Direction must be centralized and never duplicated with raw page-index math.

Required invariant:

```text
RB / right edge / E:
    rotate to the page physically on the viewer's right.

LB / left edge / Q:
    rotate to the page physically on the viewer's left.
```

The implementation should expose named helpers similar to:

```rust
enum PageTurn {
    ViewerLeft,
    ViewerRight,
}

fn page_on_viewer_left(page: OotPage) -> OotPage;
fn page_on_viewer_right(page: OotPage) -> OotPage;
fn turn_page(direction: PageTurn);
```

All page-turn inputs must call the same helper:

* LB,
* RB,
* keyboard Q/E,
* left/right edge prompts,
* mouse wheel if enabled,
* swipe gestures if later implemented.

There should be no raw `index() + 1`, `index() - 1`, `rotate_left`, `rotate_right`, or sign flipping scattered across individual handlers.

### 5.4 Page edge prompts

The demo should not have a top four-page tab strip.

Navigation should be implied by left/right edge prompts and shoulder buttons. Edge prompts may be small icon-only controls near the side edges of the active face.

Edge prompts are actionable, but they should not dominate the page. They should feel like OoT page-turn affordances, not modern tabs.

## 6. Open and Close Animation

### 6.1 Menu opening

The OoT-style open animation should be opt-in.

The desired feel:

* cube faces appear/build into place,
* pages fold or assemble around the player,
* the effect resembles OoT’s pause menu opening rather than a generic fade.

### 6.2 Menu closing

Closing should feel source-inspired:

* the cube/page faces collapse or fold away,
* the current pane should not just fade out,
* side/top/bottom faces should participate if visible,
* the page movement should feel connected to a cube shell.

### 6.3 Implementation note

The core crate should expose the effect hook/configuration. The OoT demo chooses the OoT-style effect explicitly.

## 7. Input Model

### 7.1 Keyboard

Required keyboard mapping:

```text
Q / PageUp:
    viewer-left page turn

E / PageDown:
    viewer-right page turn

Arrow keys / WASD:
    move focus among focusable menu elements

Enter / Space:
    activate selected item/control

Z:
    assign selected item to C-left

X:
    assign selected item to C-down

C:
    assign selected item to C-right

B:
    open save prompt, or cancel/return during save prompt

Escape:
    pause/unpause or cancel prompt depending on state
```

### 7.2 Gamepad

Required gamepad mapping:

```text
LB:
    viewer-left page turn

RB:
    viewer-right page turn

D-pad:
    focus movement among focusable menu items

A / South:
    activate selected item or confirm prompt

B / East:
    open save prompt or cancel/return during prompt

Right stick left:
    assign selected item to C-left

Right stick down:
    assign selected item to C-down

Right stick right:
    assign selected item to C-right
```

The right-stick C-button mapping should be edge-triggered, not continuously repeated every frame. Use a latch or threshold crossing so a held stick direction only assigns once until released.

### 7.3 Mouse

Mouse behavior:

* hover should show hover feedback,
* click on a normal item selects/activates it as appropriate,
* click on a C-button target assigns the currently selected item,
* click on B opens the save prompt,
* click on Yes/No selects or activates the save prompt choice.

### 7.4 Touch

Touch is not the current priority, but the demo should be compatible with future touch behavior.

Expected future behavior:

* tap item to select,
* tap C-button target to assign,
* drag/swipe left/right to page-turn,
* drag scroll areas when present,
* cancel click activation if touch drags off the original target.

## 8. Focus, Hover, Action, and Disabled States

### 8.1 Distinct concepts

The demo must separate:

```text
hover:
    pointer is over something

selection/focus:
    keyboard/gamepad target

action:
    click/confirm/tap executes behavior

disabled:
    visible but unavailable
```

### 8.2 Selection rendering

Selected/focused items should use OoT-like white square/corner brackets.

Selection should not rely primarily on fill color. Fill color may warm slightly, but the corner brackets are the primary selected-state indicator.

### 8.3 Hover rendering

Hover should be visually distinct from selection. It may use:

* subtle glow,
* warm wash,
* slight brightness change.

Hover must not look like the white selection brackets.

### 8.4 Disabled rendering

Disabled items remain visible in their correct slots but are:

* dimmed,
* desaturated or grayed,
* non-focusable,
* non-actionable,
* clearly marked if useful with small “child” or disabled detail text.

### 8.5 Actionable but non-focusable controls

C-button HUD icons are actionable targets but not focusable menu controls.

That means:

* D-pad/arrows should not move focus onto them,
* white selection brackets should not appear around C HUD buttons,
* click/tap can activate them,
* keyboard/gamepad right-stick can activate them.

The same distinction may apply to some HUD decorations.

## 9. HUD Overlay

The OoT demo should include the gameplay HUD elements relevant to the pause interface.

### 9.1 Required HUD elements

The HUD overlay should include:

* health hearts,
* magic meter,
* red Start button,
* C-button cluster,
* green B button,
* blue A button.

### 9.2 Coordinate caution

The OoT source coordinates may be inverted or use a different origin compared to Lunex page space.

Implementation should not blindly copy raw OoT source coordinates. Instead, create a single coordinate conversion helper for source-derived HUD points.

The conversion helper should account for:

* source origin,
* x/y inversion,
* page aspect ratio,
* normalized Lunex page coordinates,
* whether the coordinate represents a center point or top-left point.

### 9.3 Health hearts

Health hearts should appear in the upper-left HUD area.

They are decorative in the demo.

Requirements:

* visible on all normal pause pages,
* do not flicker,
* do not overlap the magic meter,
* use one stable depth band.

### 9.4 Magic meter

The magic meter appears below the hearts.

Requirements:

* stable, non-flickering rendering,
* avoid overlapping transparent full-size panels,
* use one meter background and one meter fill if possible,
* no z-fighting with the page background.

If flicker appears, simplify the meter to a single solid green bar before adding layered styling.

### 9.5 Start button

The red Start button should appear near the top HUD region, matching OoT’s functional feel.

It is mostly decorative during the pause menu because the player is already paused. It may eventually close/unpause or serve as a prompt-cancel input, but it should not become a normal focus target.

### 9.6 C-button cluster

The C-button cluster should be positioned in the upper-right HUD area.

Required C-button behavior:

* C-left target,
* C-down target,
* C-right target,
* optional C-up/Navi decorative slot,
* C targets show assigned item icons,
* C targets are not focusable,
* C targets are clickable/tappable assignment targets,
* right stick maps to C-left/C-down/C-right.

### 9.7 A and B buttons

The green B and blue A button indicators should be positioned slightly right of center in the lower HUD/action area, matching the OoT functional feel.

They should not be high near the top.

Behavior:

```text
B:
    opens the save prompt in normal menu state
    cancels/returns during save prompt

A:
    activates/decides selected item or prompt choice
```

During save prompt:

* A should indicate Decide/Confirm,
* B should indicate Cancel/Back,
* inactive buttons should dim.

## 10. Items Page

### 10.1 Layout

The Items page uses a 6 × 4 item grid based on OoT’s inventory slot order.

Required layout:

```text
Row 1:
    Deku Stick
    Deku Nut
    Bomb
    Fairy Bow
    Fire Arrow
    Din's Fire

Row 2:
    Fairy Slingshot
    Ocarina of Time
    Bombchu
    Hookshot / Longshot
    Ice Arrow
    Farore's Wind

Row 3:
    Boomerang
    Lens of Truth
    Magic Bean
    Megaton Hammer
    Light Arrow
    Nayru's Love

Row 4:
    Bottle 1
    Bottle 2
    Bottle 3
    Bottle 4
    Adult Trade Item
    Child Trade Item / Mask
```

### 10.2 Adult Link mode

The demo default state is Adult Link.

Adult Link restrictions:

* child-only items remain visible,
* child-only items are grayed out,
* child-only items cannot be selected or assigned.

Child-only examples:

* Deku Stick,
* Fairy Slingshot,
* Boomerang,
* Magic Bean,
* child trade item / mask.

Deku Nut may remain available if the demo chooses to treat it as usable in this simplified model, but the final behavior should be checked against the desired source-accurate adult inventory rules.

### 10.3 Item selection

Selecting an item should:

* move focus to that item,
* show white selection brackets,
* update status text,
* allow C-button assignment if adult-usable.

Disabled child-only items should not receive focus.

### 10.4 C-button assignment

The Items page supports assigning selected items to:

* C-left,
* C-down,
* C-right.

Assignment can be triggered by:

* keyboard Z/X/C,
* gamepad right stick left/down/right,
* mouse/touch click on a C-button HUD target.

C-button targets themselves are not focusable.

### 10.5 Assignment animation

When assigning an item to a C-button:

1. determine the item’s grid slot center,
2. determine the target C-button HUD center,
3. spawn/show a moving copy of the item icon,
4. interpolate from slot to HUD target,
5. commit the assignment near the end of the animation,
6. remove the moving copy when complete.

Animation should visually resemble OoT’s item-equip movement.

Implementation notes:

* use smooth interpolation,
* the moving icon should render above normal page contents,
* the moving icon should not disturb the source slot icon,
* assignment should be ignored for disabled items,
* if another assignment starts mid-animation, either interrupt cleanly or queue/replace the animation.

## 11. Equipment Page

### 11.1 Source-like layout

The Equipment page should be arranged similarly to OoT:

* upgrades column on the left,
* character preview left-center,
* equipment choices on the right,
* rows for sword, shield, tunic, boots,
* three choices per relevant row.

### 11.2 Adult Link restrictions

Adult Link cannot equip certain child-only equipment.

At minimum:

```text
Kokiri Sword:
    visible but disabled

Deku Shield:
    visible but disabled
```

Disabled equipment should:

* be grayed out,
* not be focusable,
* not equip on click/confirm,
* not show as available unless already equipped in an explicit child-mode demo.

### 11.3 Equipped state

Currently equipped items should show a clear equipped indicator, such as:

* small “E” label,
* check mark,
* border,
* brighter icon.

Equipped state and selected state should be visually distinct.

## 12. Map Page

### 12.1 Placeholder status

The Map page can remain a placeholder but should keep the source-like relative placement of major locations.

### 12.2 Requirements

The Map page should include:

* Hyrule Field center region,
* location markers,
* readable labels,
* stable relative placement,
* marker selection,
* no flickering map panels.

Avoid overlapping large transparent panels. Use a simplified opaque or mostly opaque map plate if needed.

## 13. Quest Status Page

### 13.1 Source-like arrangement

Quest Status should follow source-like spatial relationships:

* medallions in the correct cluster,
* spiritual stones grouped correctly,
* songs in compact rows,
* song reminder buttons smaller than medallions,
* skulltula token indicator,
* Stone of Agony,
* Gerudo Card,
* heart-piece indicator in a compact 2×2 grid near the top-middle of the Quest Status page.

### 13.2 Fit requirements

The page must fit in the active face without crowding offscreen.

Song buttons should be smaller than primary quest icons. Quest icons should not be so spaced out that the page fails to fit.

## 14. Save Prompt Flow

### 14.1 Trigger

Pressing B in the normal menu opens the save prompt.

The demo should not instantly save on B.

### 14.2 Animation

The active pane should flip around a horizontal axis into the save menu state.

The axis is conceptually the horizontal line from the center of the left edge to the center of the right edge.

Source inspiration:

* OoT uses prompt/save menu state changes,
* OoT uses prompt pitch/rotation values,
* the prompt appears as a flipped/rotated state rather than a simple popup.

Demo implementation may approximate this by:

* rotating the active face around local X,
* swapping content when the face is edge-on,
* settling into the prompt face,
* reversing the transition when closing.

### 14.3 Prompt contents

Prompt text:

```text
Would you like to save?
```

Options:

```text
YES
NO
```

### 14.4 Prompt controls

During prompt:

```text
Left / Right:
    choose Yes or No

A / Enter / Space:
    confirm current choice

B / Start / Escape:
    cancel / return

C-buttons:
    inactive

Page rotation:
    inactive

Normal page items:
    inactive
```

### 14.5 Prompt visual state

During prompt:

* C-buttons are dimmed/inactive,
* normal page controls are unavailable,
* Yes/No are the only focusable choices,
* A/B labels change to prompt meanings,
* save panel appears on the active face.

### 14.6 Saved state

If Yes is confirmed:

* show “Saved.” or equivalent confirmation,
* then allow A/B/Start to return to the pause menu.

If No is confirmed or B/Start cancels:

* return to the normal pause menu without saving.

## 15. Coordinate Conversion

### 15.1 Problem

OoT source coordinates may not match Lunex page coordinates directly.

Potential differences:

* origin may be center instead of top-left,
* y axis may be inverted,
* units may be screen pixels or display-list units,
* source coordinates may refer to quad centers,
* Lunex coordinates are normalized page percentages,
* page aspect ratio differs from screen coordinate assumptions.

### 15.2 Required helper

The demo should define one helper for source-derived coordinates.

Example conceptual API:

```rust
fn oot_center_to_page_rect(src_x: f32, src_y: f32, src_w: f32, src_h: f32) -> MenuRect;
```

The helper should document:

* source coordinate bounds,
* whether `src_x/src_y` are centers,
* whether y is inverted,
* how scaling is applied,
* how aspect ratio is handled.

No source-derived layout should manually repeat coordinate inversion logic.

### 15.3 Calibration targets

Use these visible targets to calibrate conversion:

* item grid slot centers,
* quest medallion positions,
* spiritual stone positions,
* C-button HUD cluster,
* A/B button placement,
* save prompt panel location.

## 16. Rendering and Flicker Rules

### 16.1 Avoid z-fighting

Flicker usually means overlapping transparent or coplanar planes.

Avoid:

* stacked full-screen transparent panels,
* multiple panels with same depth and size,
* transparent magic meter layers on the same plane,
* decorative panels overlapping controls unless depth is deliberate.

### 16.2 Stable depth bands

Use stable depth bands for:

* page background,
* panels,
* controls,
* icons,
* selection brackets,
* moving assignment icon,
* modal prompt overlay.

### 16.3 Magic meter rule

If the magic meter flickers, simplify it.

Preferred simple implementation:

* one dark meter background,
* one solid green fill,
* no translucent overlap,
* no nested full-size panels.

## 17. Data Model Requirements

### 17.1 Page enum

Required enum:

```rust
enum OotPage {
    Items,
    Equipment,
    Map,
    Quest,
}
```

### 17.2 C-button enum

Required enum:

```rust
enum CButton {
    Left,
    Down,
    Right,
}
```

C-up may be decorative/reserved.

### 17.3 Page turn enum

Required enum:

```rust
enum PageTurn {
    ViewerLeft,
    ViewerRight,
}
```

### 17.4 Save state

Required save prompt state model:

```rust
enum SavePromptState {
    Idle,
    Appearing,
    WaitChoice,
    Saved,
    Closing,
}

enum SaveChoice {
    Yes,
    No,
}

struct SavePrompt {
    state: SavePromptState,
    choice: SaveChoice,
    progress: f32,
}
```

### 17.5 Assignment animation state

Required assignment animation model:

```rust
struct AssignAnim {
    item_idx: usize,
    button: CButton,
    from: Vec2,
    to: Vec2,
    progress: f32,
    committed: bool,
}
```

### 17.6 Item data

Each item should include:

```rust
struct OotItem {
    name: &'static str,
    short: &'static str,
    icon: &'static str,
    detail: Option<&'static str>,
    important: bool,
    adult_usable: bool,
}
```

### 17.7 Equipment data

Each equipment choice should include:

```rust
struct EquipChoice {
    name: &'static str,
    short: &'static str,
    icon: &'static str,
    adult_usable: bool,
}
```

## 18. Action Model Requirements

Required action enum concepts:

```rust
enum OotAction {
    EdgeLeft,
    EdgeRight,
    AssignC(CButton),
    Save,
    SaveChoice(SaveChoice),
    Item(usize),
    EquipChoice { slot: usize, choice: usize },
    MapMarker(usize),
    QuestIcon(usize),
    Song(usize),
}
```

Action filtering:

* `AssignC` is actionable but not focusable.
* `EdgeLeft` and `EdgeRight` are actionable but should usually not be normal focus targets.
* `Save` is actionable but may not need to be focusable except where desired.
* `SaveChoice` actions are focusable only during save prompt.
* disabled items/equipment should have no action attached.

## 19. Acceptance Criteria

### 19.1 Basic rendering

* Demo opens to a visible Items page.
* Neighboring cube faces are visible at edges.
* No blank-screen regression.
* No major flicker on magic meter or map page.

### 19.2 Page rotation

* LB moves to the viewer-left page.
* RB moves to the viewer-right page.
* Q and E match LB/RB semantics.
* Left/right edge prompts match LB/RB semantics.
* No duplicated raw direction logic.

### 19.3 Item page

* Item grid matches OoT slot layout.
* Adult-disabled items are grayed out.
* Disabled items cannot receive focus.
* Disabled items cannot be assigned.
* Selected item uses white corner brackets.

### 19.4 C-button assignment

* C targets are not focusable.
* Clicking/tapping a C target assigns selected item.
* Right stick left/down/right assigns selected item.
* Assignment plays moving-icon animation.
* Assigned icon appears in target C HUD slot.

### 19.5 Equipment page

* Kokiri Sword is disabled for Adult Link.
* Deku Shield is disabled for Adult Link.
* Disabled equipment cannot be equipped.
* Equipped state is visually distinct from selected state.

### 19.6 Save prompt

* B opens save prompt.
* Active pane visibly flips into prompt state.
* Prompt says “Would you like to save?”
* Yes/No choices are shown.
* Left/right changes choice.
* A/Enter confirms.
* B/Start cancels.
* C buttons and normal controls are inactive/dimmed.
* Returning from prompt restores normal page.

### 19.7 HUD

* Hearts upper-left.
* Magic meter below hearts with no flicker.
* Start button near top HUD region.
* C cluster upper-right.
* A/B slightly right of center, not high near top.
* Button colors:

  * Start red,
  * B green,
  * A blue,
  * C yellow.

## 20. Non-Goals

The OoT demo does not need to:

* use original OoT art,
* exactly match every pixel,
* implement every inventory trade sequence,
* implement real save files,
* implement all map dungeon behavior,
* become the core crate API.

## 21. Rebuild Checklist

To reproduce the OoT demo from scratch:

1. Create `crates/oot_pause_demo`.
2. Depend on `ambition_inventory_ui`, Bevy, and Lunex.
3. Define `OotPage`, `PageTurn`, `CButton`, `OotAction`.
4. Build the four inside-cube Lunex page faces.
5. Centralize page-turn direction using `PageTurn`.
6. Build data-driven page models for Items, Equipment, Map, Quest.
7. Implement the Items 6 × 4 source-like grid.
8. Add Adult Link item restrictions.
9. Add Equipment page with Adult Link restrictions.
10. Add Map placeholder with stable marker placement.
11. Add Quest Status source-like icon layout.
12. Add HUD overlay: hearts, magic, Start, C, B, A.
13. Implement C-button assignment actions.
14. Implement item-to-C assignment animation.
15. Implement save prompt state machine.
16. Implement save prompt flip animation.
17. Add selection corner brackets.
18. Add disabled-state rendering.
19. Add keyboard input.
20. Add mouse input.
21. Add gamepad input including right-stick C assignment.
22. Add generated placeholder assets.
23. Verify no flicker.
24. Verify LB/RB direction invariant.
25. Verify Adult Link restrictions.
26. Verify save prompt flow.


## 22. Recent Iteration Decisions

### 22.1 Persistent HUD is gameplay HUD, not pause-face content

The heart, magic, Start, A, B, and C-button HUD is independent gameplay HUD. It must remain visible and fixed when the pause box opens, closes, spins, or enters the save prompt. The pause/cube faces may rotate or disappear; the HUD layer should not translate, scale, fold, rotate, or clip against those faces.

### 22.2 Disabled Adult/Child entries are selectable but not usable

Adult Link mode remains the default, but child-only inventory and equipment entries should still be selectable/focusable. This preserves the source-like slot layout and lets the player inspect disabled entries. Disabled entries are visibly dimmed and may show a small disabled detail label, but activation, C-button assignment, and equipment changes must be blocked with clear status feedback.

### 22.3 Selection brackets and equip/highlight are distinct

The white corner-bracket cursor indicates the current keyboard/gamepad selection, even on disabled entries. Equipped/important/highlighted state must use a separate visual treatment so the player can tell the difference between “currently selected” and “currently equipped.”

### 22.4 Edge prompts are horizontal sentinels only

The L/R page-turn prompts are focusable by horizontal navigation and clickable, but vertical movement should never choose them as the best target from an item/equipment grid. Moving left from L or right from R turns the page and lands on the opposite prompt of the neighboring page. Moving up/down from grid items should prefer the item above/below, not diagonal edge prompts.

### 22.5 HUD button art uses icons, not runtime labels

Start, A, and B should be rendered as generated HUD button icons without extra runtime text labels layered on top. The C-button targets should use yellow arrow art for empty/default targets; once an item is assigned, the item icon replaces/overwrites that arrow affordance and should not ghost transparently over the arrow.

### 22.6 Save prompt close behavior

After confirming save, the prompt should show a stable saved acknowledgement until dismissed. During the closing flip, the prompt should not snap back to the Yes/No state before the normal face returns. Once the closing flip crosses back through the edge-on midpoint and the normal pane is visible, normal selection and controls should be restored immediately; the UI should not remain visually disabled until the last sub-pixel tail of the rotation finishes.
