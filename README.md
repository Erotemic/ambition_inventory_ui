# Ambition Inventory UI Prototype

This is now a **Lunex / inside-the-cube worldspace UI** prototype. The previous render-to-texture experiment proved the right problem, but introduced upside-down texture orientation, fuzziness, and snapshot mismatch. This version removes the RTT path and builds the menu faces as real 3D UI panels using `bevy_lunex`.

## Goal

A small polished vertical slice for an inventory UI crate:

- an OoT-inspired rotating page volume controlled by explicit page inputs,
- real 3D UI faces instead of warped 2D screenshots,
- mouse, keyboard, gamepad, and touch-friendly hit targets,
- one concrete test flow: **Gear -> Feet -> Iron Boots -> Equip -> Toggle active**.

## Run

```bash
cargo run
```

## Controls

Keyboard:

- `Q` / `E` or `PageUp` / `PageDown`: rotate pages left/right
- Arrow left/right or `A` / `D`: move spatially between visible UI columns
- Arrow up/down or `W` / `S`: move spatially within the current column
- `Enter` / `Space`: activate focused selection/action
- `T`: toggle Iron Boots if equipped
- `Esc` or `P`: menu toggle, depending on the Status page `Menu toggle` option
- `Backspace`: back out toward the slot list

Mouse / touch:

- Scroll wheel: rotate pages, or scrolls the Status pane while Status is active
- Hover actionable panels for pointer cursor + highlight feedback; hover updates selection only when the logical target changes
- Left click/tap: activate the hovered panel/button
- Right click: back
- Only the active/front page is pickable; neighbor faces are visible but decorative while angled.

Gamepad:

- Left/right trigger: rotate pages using the inside-room/OoT-style direction mapping
- D-pad left/right: move spatially between visible UI columns
- D-pad up/down: move spatially within the current column
- South face button: activate
- East face button: back
- West face button: toggle Iron Boots if equipped
- Left stick: currently disabled for cursor movement while worldspace picking is stabilized

## Architecture note

The important shift is that the menu pages are no longer textures on quads. Each page is a `UiRoot3d` attached to a rotating parent entity. Lunex computes panel layout in local page coordinates, and the whole page inherits the parent transform, so text, panels, highlights, and hit targets rotate together as one 3D UI surface.

This is closer to the desired final architecture for a Bevy inventory UI crate than the egui or RTT prototypes:

- inventory/game rules remain in plain ECS resources,
- layout is retained and entity-based,
- the page switch is a transform animation,
- the UI can later grow real picking/raycast selection without changing the inventory action model.

## Vendored dependency

This overlay vendors the uploaded `bevy_lunex` source under `vendor/bevy_lunex` and points `Cargo.toml` at it with a path dependency. That keeps this prototype pinned to the exact source snapshot being evaluated.

## Flicker mitigation

This revision adds explicit `UiDepth::Set(...)` bands for page backgrounds, large panels, actionable cards, and text. The previous version relied on sibling/default depths, which left several overlapping Lunex planes coplanar with the full-page background. When the menu rotated, those planes could z-fight and flicker. Panels are also placed in the opaque render pass for stability; text remains alpha blended.


## Inside-the-cube orientation

This revision treats the menu as a room around the viewer, not as panels on the outside of a box. The 3D camera is placed near the center of the menu volume and looks outward toward the active wall. The four Lunex page roots are positioned as surrounding walls and the page ring rotates around the viewer when changing pages.

The UI materials are double-sided (`cull_mode: None`) so the inside face of each page wall is visible. Because the viewer is now seeing the back side of each page root, this revision uses inward/negative `UiDepth` bands: backgrounds stay farthest from the viewer, while cards, actions, and text are progressively closer. That prevents opaque panels from hiding the text as dark squares.

Analog-stick cursor movement is disabled in this revision. Gamepad control remains available through triggers, D-pad, and face buttons.


## Navigation / feedback revision

This revision puts arrows and the D-pad back in the role of ordinary UI navigation. Page rotation is on `Q`/`E`, `PageUp`/`PageDown`, mouse wheel, or controller triggers. Actionable Lunex panels now use `UiHover` + `UiColor` state blending and pointer cursor changes, giving a more Bevy-UI-like default hover response. The page ring radius is increased so the active face fits the standard 1180x760 window and neighboring faces only show at the edges.


## Interaction polish revision

This revision changes the active page controls from abstract focus-group cycling to spatial focus movement. On the Gear page, left/right moves between the Slots, Boots, and Actions columns while preserving the row where possible; up/down moves within the current column. Pointer hover now updates the same focus state used by keyboard and gamepad navigation, so mouse/touch, arrows, and D-pad all agree about the currently selected element. Non-actionable panels and text are marked `Pickable::IGNORE` so they do not block clicks/taps on the actual controls.

## Input fallback revision

This revision fixes two interaction gaps from the first Lunex version:

- Arrow keys and D-pad now operate on every active page, not only Gear. Pack, Map, and Status each have selectable rows/cards with visual focus feedback.
- Mouse/touch activation no longer depends only on Lunex/MeshPicking events. The prototype now performs a screen-space hit-test against the active 3D page face by projecting each logical button rectangle through the camera. Hover updates the same selection state used by keyboard/gamepad, and click/tap activates the same `ClickAction`.

The page room parent now also carries `UiRoot3d`/`Visibility` so the Lunex face children do not sit under a plain hierarchy parent, which should eliminate Bevy hierarchy warning B0004 for the menu faces.

## Menu controls / reusable component revision

This revision treats the page tabs as normal controls on every page. Arrow/D-pad up from the top content row moves focus to the tab row; left/right moves between tab buttons; `Enter`/South activates the focused tab and rotates to that page. Bumpers/triggers and `Q`/`E` still remain as the fast page-switch path.

The Status page now includes simple reusable option controls:

- checkbox-like toggle: Input hints enabled/disabled,
- radio-style toggle: Layout density cozy/compact,
- combo-style cycling option: Detail level minimal/normal/verbose.

The code keeps these behind the same `ClickAction` / focus / hit-target model used by gear slots and inventory cards, which is the first step toward extracting a reusable Lunex inventory-menu component. Pointer hover no longer mutates inventory focus every frame; Lunex hover state provides visual feedback, while click/tap selects and activates. This avoids despawning/rebuilding all page faces on ordinary mouse movement and should feel less laggy.

## UI feel / churn-aware revision

This revision keeps the current Lunex inside-cube presentation and focuses on interaction feel instead of changing the visual model.

- Mouse hover remains handled by Lunex `UiHover` / `UiColor`, so icons and panels highlight without changing inventory state every frame.
- Mouse click now goes through one inventory-layer hit-test path instead of both Lunex click observers and the custom fallback. This avoids double-activating toggle/option controls.
- Touch has a configurable policy on the Status page:
  - `Select + tap`: first tap selects/highlights a control, second tap activates it.
  - `Instant tap`: first tap activates immediately.
- Status options now include Input hints, Layout density, Detail level, and Touch mode.
- Rebuild churn is reduced: page changes rebuild all faces because pickability/tab state changes, but focus/content changes on the same page rebuild only the active page face.

The next higher-value optimization would be replacing active-page rebuilds with persistent control entities plus a movable focus/highlight layer, but this revision keeps the code simple while cutting the most obvious four-page respawn churn.

## Menu shell / open-close revision

This revision treats the rotating Lunex room as a reusable menu shell rather than only an always-on demo scene.

- `MenuShell` owns the lifecycle: closed, opening, open, and closing are represented by a continuous `openness` value plus a target state.
- The shell animation scales and eases the menu room in/out when the registered toggle input is pressed.
- The demo default toggle is `Escape / Start`; the Status page includes a `Menu toggle` row that cycles the demo binding between `Escape / Start` and `P / Start`. In a real game, the host would wire its own pause/menu action into `MenuShell::toggle()`.
- While the shell is opening or closing, inventory navigation/input is locked. This keeps focus, click, and touch behavior deterministic.
- `Backspace` is now the keyboard back/cancel action inside the menu. Escape is reserved for the menu toggle when the default binding is active.

## Status scroll pane revision

The Status page now behaves like a small scroll pane instead of an overflowing list.

- Only five rows are visible at a time.
- D-pad/arrow navigation clamps through the full status row list and automatically scrolls the selected row into view.
- Mouse wheel scrolls the Status pane when Status is the active page. On other pages, wheel still rotates the cube pages.
- Pointer hit-testing only targets visible rows, so clicks/taps match what the user sees.


## OoT-style open / close revision

This revision adds a second menu-shell animation mode based on the uploaded OoT Kaleido Scope source. In `z_kaleido_scope.c`, the pause menu open/close state drives the four page pitch values from `160.0` down to `0.0` on open and back up to `160.0` on close. The draw matrices divide those values by `100.0`, so the pages fold by roughly `1.6` radians. The source applies the pitch signs by page position: `-Z` uses `RotateX(-pitch)`, `+Z` uses `RotateX(+pitch)`, `+X` uses `RotateZ(-pitch)`, and `-X` uses `RotateZ(+pitch)`. The prototype maps that to the Lunex inside-cube wall positions so the pages build into the room on open and fall away on close, rather than spinning like flat clock hands.

The prototype now has two shell styles:

- `OoT page fold`, the new default, which folds each Lunex page wall into place using that source-inspired pitch model.
- `Smooth scale`, the previous shell animation, kept as an option.

The Status page `Open/close` row cycles between those styles. This keeps the prototype moving toward a reusable Lunex menu-shell component: host games can wire their own input into `MenuShell::toggle()`, while the shell owns lifecycle, animation style, and interaction locking.

This revision also restores mouse hover highlighting through the inventory-layer hit-test path, but only changes focus when the hovered logical control changes. The Status scroll bar track/thumb now use explicit separated depth bands to reduce z-fighting flicker while the page is angled.


## Tight cube / OoT geometry revision

This revision fixes two mismatches with the OoT feel:

- The page faces now use a tight cube relationship: `PAGE_W == 2 * PAGE_RADIUS`. In the OoT source the page background is `3 * 80 = 240` units wide, then scaled by `0.78`, giving about `187.2` units of page width. `R_PAUSE_DEPTH_OFFSET / 100.0` is about `93.55`, so the page width is effectively `2 * depth`. That means adjacent pause pages meet at their vertical edges instead of floating apart.
- The camera is no longer exactly at the cube center. It is slightly backed away from the active page, closer to OoT's `eye` vs page-depth relationship. This keeps the active page readable while still exposing the shared page edges and a little of the neighboring pages.

Thin page-edge rails were added to make the shared cube edges readable during rotation. Controller shoulder direction was also adjusted for the inside-room mental model: the right shoulder pulls the right-hand wall into view and the left shoulder pulls the left-hand wall into view, rather than moving the visible square in the opposite-feeling direction.

## Reversed selectors / lower-edge OoT fold revision

This revision keeps the physical cube order unchanged, but reverses the visible tab-strip order to `Status / Map / Pack / Gear`. In the inside-cube view this should make the highlighted tab feel like it moves with the wall being pulled into view, instead of fighting the spatial rotation.

The OoT-style open/close animation now includes the lower-edge hinge detail from `include/pause.h` and `z_kaleido_scope.c`. OoT shifts `pagesYOrigin1` and `R_PAUSE_PAGES_Y_ORIGIN_2` during opening/closing so each page rotates around its lower edge rather than flipping around its center. The Lunex prototype now keeps each page's bottom-center hinge fixed while applying the source-style fold signs:

- front/back pages fold around X,
- left/right pages fold around Z,
- all four faces fold together from about `1.6` radians to `0` on open, and back on close.

This keeps the existing tight cube geometry where adjacent faces meet at visible edges. The remaining OoT detail still not implemented as a true material effect is per-page alpha fade; the code comments preserve the source behavior, but the prototype currently avoids per-frame material-alpha churn while the interaction model is still changing.

## Data-driven API / reusable crate direction

This revision starts separating the reusable menu component from the demo game
state.

The public API surface lives in `src/lib.rs` and is intentionally small:

- `MenuPageModel<PageId, Action>` describes one page/face.
- `MenuNode<Action>` describes panels, text, and actionable controls.
- `MenuRect` uses normalized page coordinates so the same data can render to a
  Lunex 3D page, a flat debug view, or a future mobile layout.
- `MenuControlKind` gives controls semantic roles such as tab, slot, item,
  action, option, map marker, or scrollbar.
- `MenuShellEffects` is a host hook queue. The UI module pushes lifecycle cues
  such as `Opening`, `Opened`, `Closing`, and `Closed`; a game can drain those
  to play sfx, pause gameplay, muffle music, or update a game-mode resource.

The demo still owns gameplay state (`InventoryDemo`), but it now builds each
page through `build_page_model(...)` and the Lunex renderer consumes that model.
Pointer hit testing also reads the same model, which keeps drawing and input in
sync. This is the shape to preserve when extracting a true library crate:

game data -> `MenuPageModel` -> Lunex renderer / hit testing -> `Action` back to game

## Ambition integration notes

The uploaded Ambition docs emphasize that menus should consume semantic menu
input rather than raw device input, that touch should use conservative
select-then-confirm behavior for rows, and that pause/menu state should suppress
gameplay movement. This prototype aligns with those constraints:

- keyboard, mouse, touch, and gamepad all map into the same selected control and
  action path;
- touch mode is configurable between `Select + tap` and `Instant tap`;
- the shell locks interaction while opening/closing;
- lifecycle effects are exposed without hard-coding audio or music policy;
- dev-style toggles are visible on the Status/settings page rather than hidden
  only behind hotkeys.

The next extraction pass should move the Lunex renderer into a plugin with a
host-provided page-builder callback or resource, plus systems that emit host
`Action` values when controls are activated.

## Package polish / ECS boundary revision

This revision keeps the data-driven page model, but moves the state that benefits
from ECS identity onto rendered entities:

- `AmbitionMenuRoot` marks the shell/root entity.
- `AmbitionMenuPage<PageId>` marks each page face and records whether it is the active page.
- `AmbitionMenuControl<Action>` stores the semantic control kind, action, and focus key on rendered controls.
- `MenuVisualState` stores frequently changing hover/focus/selected/pressed/disabled state.
- `MenuScrollPane` marks the Status page viewport with visible/total row metadata.

The deliberate boundary is: **menu content remains data-driven**, while
**rendered controls and visual interaction state are ECS components**. I do not
think every declarative menu row should become hand-authored ECS state; that
would make inventory screens harder to construct from Ambition's equipment,
item, map, and settings data. The builder/model layer should remain the normal
API, with ECS components available for renderer/input systems and advanced host
integration.

`MenuShellConfig` now exposes shell defaults. The reusable crate default is
`SmoothScale`; the OoT-style fold is intentionally opt-in through
`MenuOpenCloseStyle::OotPageFold`. The demo opts into the OoT fold because this
prototype is specifically validating that nostalgic shell, but games using the
crate should choose it deliberately.

The Status scrollbar was changed from overlapping track/thumb planes to a
segmented scroll indicator. The old two-plane version could still shimmer while
angled in 3D. The segmented indicator avoids self-overlap and should be more
stable while retaining clear scroll position feedback.
