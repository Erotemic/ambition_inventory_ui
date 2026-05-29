# Ambition Inventory UI: High-Level Design Decisions

## 1. Purpose

`ambition_inventory_ui` is a reusable Bevy inventory/menu UI package intended to support game-feeling inventory systems rather than conventional application-style menus. The package should be useful for Ambition, but it should not be Ambition-specific. The project should demonstrate that it can support a polished, nostalgic, controller-friendly inventory experience while remaining configurable enough for other games.

The current design has three related deliverables:

1. **Core reusable crate**: the generic system, data model, input model, effects, and Lunex-based rendering support.
2. **Regular demo**: an Ambition-style inventory demo that exercises practical game UI features.
3. **OoT demo**: a separate functional recreation demo showing that the core system can compose into an Ocarina-of-Time-like pause menu experience.

The guiding principle is: **polished, coherent interaction beats a broad but rough feature checklist.**

## 2. Core System Design Decisions

### 2.1 The core crate must stay generic

The core crate should expose reusable menu concepts, not hard-code Zelda, Ambition, or any one game’s inventory structure.

The following belong in the core crate:

* menu/page model abstractions,
* page shell / cube geometry configuration,
* open/close effects,
* selection effects,
* input routing patterns,
* data-driven menu node descriptions,
* generic action dispatch,
* Lunex-based rendering helpers,
* reusable animation hooks.

The following belong in demos or downstream games:

* exact item lists,
* exact quest layouts,
* Zelda-style medallion/song arrangements,
* Ambition-specific equipment slots,
* game-specific save semantics,
* exact HUD artwork,
* exact audio behavior.

The OoT demo should prove the core can support an OoT-like experience, but the core should not become an OoT UI library.

### 2.2 Lunex is the primary rendering layer for the 3D menu feel

The project initially experimented with egui-style layout and fake transforms, but that created issues with text and buttons resizing or not transforming as real surfaces. The preferred direction is now:

* use Lunex / 3D UI surfaces for page faces,
* render menu pages as surfaces in a cube-like room,
* transform entire faces rather than redrawing every widget with fake projection,
* avoid letting individual buttons/text live outside the page transform.

This supports the desired “inside the cube looking out” feeling.

### 2.3 Bevy ECS should be used where it helps

The system should be ECS-native where ECS provides clarity and composability:

* resources for current menu state,
* components for page faces and menu roots,
* systems for input handling,
* systems for animation,
* systems for rebuilding rendered menu surfaces when data changes.

However, not every concept needs to become its own entity/component. The data-driven page model can remain a plain Rust data structure that is rendered into Lunex entities. ECS should support the architecture, not make it harder to reason about.

### 2.4 Menu contents should be data-driven

Menu pages should be described with data, not hard-coded imperative drawing everywhere. The page model should represent things like:

* panels,
* text,
* item controls,
* action controls,
* decorations,
* map markers,
* popup actions,
* scrollable regions,
* disabled/inactive states,
* selection state,
* hover state,
* associated actions.

The demos may contain hard-coded data arrays, but the rendering path should treat that content as data.

### 2.5 Effects must be configurable and opt-in

The default reusable system should be conservative and broadly useful. More nostalgic or stylized behavior should be opt-in.

Current effect decisions:

* `SmoothScale` or similar simple behavior should remain the safe default.
* OoT-style page fold/open/close should be opt-in.
* The cube geometry should be configurable.
* Selection rendering should be configurable.
* Touch behavior should eventually be configurable.

The OoT demo should explicitly opt into OoT-like behavior instead of making the entire crate behave that way by default.

### 2.6 “Inside the cube” is the target mental model

The desired OoT-like menu effect is not a cube floating in front of the player. The target is:

* the player is inside the cube,
* the current page is directly in front,
* neighboring pages are visible at the left/right/top/bottom edges,
* rotating the menu pulls another wall of the cube into view,
* page edges should feel connected rather than like detached planes.

Camera tilt should not be used just to reveal edges. If edges need to be visible, that should come from scale, distance, field-of-view, or geometry calibration rather than pitching the active page away from the viewer.

### 2.7 Direction semantics must be centralized

The direction bug with LB/RB happened because “left” and “right” were represented several different ways:

* viewer-left / viewer-right,
* page index +1 / -1,
* positive / negative cube rotation,
* apparent screen motion,
* physical cube-face adjacency.

The invariant should be:

* **RB / right edge / E**: move to the page physically on the viewer’s right.
* **LB / left edge / Q**: move to the page physically on the viewer’s left.

All input paths should call the same named page-turn API, for example:

* `PageTurn::ViewerLeft`
* `PageTurn::ViewerRight`
* `page_on_viewer_left(page)`
* `page_on_viewer_right(page)`

There should be no scattered raw `index() + 1`, `index() - 1`, or sign-flipping in individual input handlers.

### 2.8 Selection, hover, and activation are separate concepts

The UI should distinguish:

* **hover**: mouse/touch pointer is over something,
* **selection/focus**: keyboard/gamepad-selected element,
* **activation**: confirm/click/use action.

OoT-like selection should use white square/corner brackets around the selected item. Hover should use a visually different effect and should not be confused with selection.

Some elements may be actionable without being focusable. For example, in the OoT demo, C-button HUD icons are click/touch assignment targets, but they should not become D-pad/gamepad focus targets.

### 2.9 Input support must be first-class

The system needs to work well with:

* mouse,
* keyboard,
* gamepad,
* touch/mobile.

Input decisions so far:

* D-pad/arrows navigate focusable UI elements.
* Enter/Space/A activate the selected item.
* LB/RB rotate menu pages.
* Mouse hover should visually respond.
* Mouse click activates controls.
* Touch should eventually support select-then-tap or tap-to-activate behavior as a configurable choice.
* Mouse drag should eventually exercise touch-like swipe behavior.
* Scroll panes should feel like modern touch scroll panes.
* Controller stick-as-mouse was tested but disabled because it interfered with the intended interaction model.

### 2.10 Performance and flicker avoidance

Several flicker issues came from overlapping transparent planes or depth layers fighting on angled surfaces. The core design should avoid that by:

* minimizing overlapping full-size transparent panels,
* using stable depth bands,
* rebuilding only when relevant state changes,
* avoiding unnecessary churn in rendered entities,
* keeping map/status panels simple unless they need extra structure.

The demos should prefer fewer, clearer layers over many subtle translucent overlays.

## 3. Regular Demo Design Decisions

The regular demo should demonstrate a polished generic game inventory, not merely a feature dump.



### Structural organization update

The reusable crate remains the workspace-root `ambition_inventory_ui` package. The regular Ambition-style demo now lives in `crates/ambition_demo`, and the OoT functional demo remains in `crates/oot_pause_demo`. Demo-specific code, assets, state machines, item lists, and visual calibration belong in those demo crates; reusable data structures and future stabilized primitives belong in the root library.

### 3.1 The regular demo is Ambition-like, not OoT-like

The regular demo should show how the reusable crate could support Ambition or another game. It can borrow lessons from OoT, Morrowind, Hollow Knight, and Super Metroid, but it should not be a clone.

The regular demo should emphasize:

* readable inventory categories,
* useful equipment assignment,
* consumable use,
* modern input support,
* good hover/selection feedback,
* mobile viability,
* clean data-driven population.

### 3.2 Equipment should demonstrate actual assignment

The demo should not just show item cards. It should demonstrate:

* selecting gear,
* equipping gear into character slots,
* replacing existing gear,
* showing equipped state clearly,
* preventing invalid assignments.

Slots may vary by character, so the system should support data-driven slot definitions.

### 3.3 Consumables should actually be consumable

Consumables in the regular demo should have behavior:

* selecting a consumable should offer a use/consume action,
* using it should decrement quantity,
* unavailable/empty consumables should become disabled or removed,
* feedback should explain what happened.

### 3.4 Popup/context menu interaction is preferred for complex item actions

For item actions like:

* equip,
* compare,
* inspect,
* consume,
* drop,
* assign,

a popup/context menu may be clearer than trying to overload one click.

The popup should be controller, mouse, and touch friendly.

### 3.5 The regular demo should include sprite icons

The demo should show sprite/icon support rather than only text labels. It should exercise:

* item icons,
* equipped indicators,
* disabled icons,
* hover/selection treatment,
* icon scaling,
* readable fallback labels if assets are missing.

### 3.6 Scroll behavior matters

Long panels, especially status/character panels, should use scroll panes. Scroll behavior should feel modern:

* no flickery scrollbar,
* intuitive grab direction,
* mouse wheel support,
* drag/touch scrolling,
* cancel click when drag leaves the original target,
* no accidental activation while scrolling.

## 4. OoT Demo Design Decisions

The OoT demo is a separate workspace crate. It exists to prove that the reusable system can compose into a highly specific nostalgic interface.

### 4.1 The OoT demo should be its own crate

The OoT functional recreation belongs in its own crate, such as:

* `crates/oot_pause_demo`

This avoids conflating:

* generic inventory UI architecture,
* Ambition’s actual UI needs,
* the OoT recreation’s source-specific layout and behavior.

The root crate remains the reusable package.

### 4.2 The OoT demo should be source-inspired, not asset-exact

The demo should use the OoT decomp/source code as behavioral and layout reference, but it should not use the exact original game assets.

The goal is:

* similar feel,
* similar interaction flow,
* similar spatial layout,
* similar animation timing/shape,
* original placeholder/demo art.

### 4.3 The four-page structure should match OoT functionality

The OoT demo should have the four functional pause pages:

1. Items
2. Equipment
3. Map
4. Quest Status

The exact page order, direction behavior, and cube rotation must stay consistent. Direction should be tested against the “viewer-left/viewer-right” invariant.

### 4.4 The item page should use the OoT slot layout

The item page should follow the OoT 6 × 4 inventory slot structure.

The intended adult-complete inventory layout is:

* Row 1: Deku Stick, Deku Nut, Bomb, Bow, Fire Arrow, Din’s Fire
* Row 2: Slingshot, Ocarina, Bombchu, Hookshot/Longshot, Ice Arrow, Farore’s Wind
* Row 3: Boomerang, Lens, Beans, Hammer, Light Arrow, Nayru’s Love
* Row 4: Bottles / trade items

Adult Link mode means child-only items are visible but disabled.

### 4.5 Adult Link mode is the default demo state

The OoT demo should represent Adult Link. That means:

* child-only inventory items remain visible,
* child-only items are grayed out,
* child-only items cannot be assigned to C-buttons,
* child-only equipment cannot be equipped.

Examples:

* Kokiri Sword should be grayed out and inaccessible,
* Deku Shield should be grayed out and inaccessible,
* Slingshot and Boomerang should be unavailable,
* child trade item should be unavailable.

This should be enforced both visually and behaviorally.

### 4.6 C-buttons are assignment targets, not focusable menu items

In OoT, C-button icons are not ordinary selectable menu controls. They are HUD assignment targets.

The demo should follow that:

* C icons should not be D-pad/arrows focus targets.
* Clicking/tapping a C target should immediately assign the currently selected item.
* Keyboard shortcuts can assign directly.
* Gamepad right stick should map to C directions:

  * right stick left → C-left,
  * right stick down → C-down,
  * right stick right → C-right.
* C-up should be decorative or reserved, not a regular item assignment target.

### 4.7 Item-to-C assignment should animate like OoT

When assigning an item to a C-button, the demo should show the item icon moving from the inventory slot to the C-button HUD target.

The animation should use:

* source slot position,
* target C-button position,
* smooth interpolation,
* visible moving icon,
* final assignment when the animation reaches the target or near the target.

This behavior is source-inspired by OoT’s item equip animation path, even if the exact math is approximated.

### 4.8 HUD button positions should match OoT’s functional feel

The OoT demo should include the HUD elements relevant to the pause menu:

* hearts,
* magic meter,
* red Start button,
* C-button cluster,
* green B button,
* blue A button.

These should be positioned according to OoT’s UI feel, keeping in mind that the coordinate system may be inverted or normalized differently in the Lunex page. The source coordinates should be treated as reference data, not blindly copied without coordinate conversion.

The A and B buttons should be slightly right of center, not high at the top of the page. C-buttons should occupy the upper-right HUD cluster.

### 4.9 Save prompt behavior should match OoT’s state flow

Pressing B should not instantly save. It should enter a save prompt state.

The save flow should be:

1. Press B.
2. Current pane flips around a horizontal axis into the save prompt.
3. Prompt asks: “Would you like to save?”
4. Yes/No options are shown.
5. Left/right chooses Yes or No.
6. A/Enter confirms.
7. B/Start cancels or returns.
8. C-buttons and unrelated controls are inactive/dimmed during the prompt.

The visual flip should approximate OoT’s `promptPitch` behavior: the active pane rotates about a horizontal line through the page, transitions into the prompt, and settles.

### 4.10 Active/inactive controls must be visually clear

During normal item selection:

* inventory items are active if usable,
* child-only items are dimmed,
* C-buttons are actionable but not focusable,
* B opens save prompt,
* A activates/confirms selected menu item.

During save prompt:

* normal page controls are inactive,
* C-buttons are dimmed/inactive,
* page rotation should be disabled,
* Yes/No are the active focus choices,
* A confirms,
* B/Start cancels.

### 4.11 Quest Status should use source-like spatial arrangement

Quest Status should be source-inspired:

* medallions in their source-like cluster,
* spiritual stones grouped correctly,
* songs smaller than quest medallions,
* skulltula indicator included,
* heart-piece indicator included as a 2×2 grid near the top-middle of the Quest Status page,
* Stone of Agony / Gerudo Card included.

Spacing should fit the page and avoid oversized gaps.

### 4.12 Equipment page should follow OoT layout

The Equipment page should preserve the source-like structure:

* upgrade column,
* player preview,
* equipment choices,
* three options per applicable slot,
* disabled child-only equipment in Adult Link mode.

Kokiri Sword and Deku Shield should be visible but inaccessible in Adult Link mode.

### 4.13 Map page can remain a placeholder but should preserve relative layout

The map page does not need to be a perfect recreation yet, but it should preserve:

* relative placement of major locations,
* map marker selection,
* minimal flicker,
* readable labels,
* stable depth/layer ordering.

The earlier map relative placement was considered directionally good and should be preserved.

## 5. Visual Design Decisions

### 5.1 Selection uses white corner brackets

OoT-style focus selection should use white square/corner brackets around the selected icon. Fill color alone is insufficient because it is too easy to confuse with hover or active state.

### 5.2 Hover and selection are visually distinct

Hover may use a warm translucent wash or subtle highlight. Selection should use the corner bracket effect. Disabled state should be dimmed and desaturated.

### 5.3 Icons are primary visuals

Item slots should be readable as icon-first controls. Text can be used as fallback or small detail text, but the demo should not regress into text-only boxes.

### 5.4 Disabled items are visible but dimmed

Disabled items should remain in layout to preserve source-like structure. They should be visibly dimmed and non-interactive.

### 5.5 Avoid overlapping translucent full-page panels

Flicker has repeatedly come from overlapping translucent layers on angled surfaces. The demos should avoid unnecessary full-size stacked panels and use stable depth bands.

## 6. Open Issues and Next Work

### 6.1 Coordinate calibration

The OoT source coordinates appear to require inversion or normalization for this Lunex page space. Future work should define a single coordinate conversion helper for source coordinates rather than manually adjusting each layout.

### 6.2 Compile-time safety for direction mapping

The `PageTurn` invariant should be backed by tests or assertions so LB/RB cannot regress again.

### 6.3 Visual regression checkpoints

Because many bugs are visual, the project would benefit from stable screenshots or simple visual regression checkpoints for:

* normal item page,
* rotating page,
* C assignment animation,
* save prompt,
* Adult Link disabled items,
* Quest Status layout.

### 6.4 Better demo assets

The current generated assets are placeholders. The demos should eventually use a coherent original icon set, not exact OoT assets.

### 6.5 Core API extraction

Once the OoT demo stabilizes, reusable pieces should be moved into the core crate:

* page-turn invariant helpers,
* focusable/actionable distinction,
* selection-corner effect,
* item-to-target animation helper,
* prompt flip effect,
* disabled-state rendering,
* HUD target/action zones.

The OoT-specific item lists, layouts, and save text should remain in the OoT demo crate.


## 7. Recent OoT Demo Feedback Captured

The OoT demo has accumulated several concrete interaction rules that should be preserved unless explicitly overridden:

* The gameplay HUD layer, including hearts, magic, Start, A, B, and C indicators, is not part of the rotating pause pane and stays fixed/visible independently of the pause box.
* Disabled child-only entries in Adult Link mode remain selectable/focusable for inspection, but cannot be assigned or equipped.
* White corner brackets represent cursor selection; equipped/highlighted state uses a separate visual.
* L/R edge prompts are horizontal focus sentinels only. Up/down navigation should not jump from an item grid into an edge prompt.
* HUD buttons should use generated icon art rather than runtime text labels. C targets use arrow icons when empty, and assigned item icons overwrite the arrow target rather than blending transparently on top.
* Save confirmation should return cleanly: after YES, keep the saved acknowledgement during the closing flip and restore normal cursor state after the face transition completes.
