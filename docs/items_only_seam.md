# Items-only Ambition integration seam

The Ambition mock demo is now the recommended executable sketch for the future
main-game menu integration. It keeps Ambition-style inventory/equipment rules in
a host-owned mock state while the UI crate receives renderer-neutral page models
and emits generic actions.

The visual shell lives in `crates/ambition_mock_demo` and deliberately mirrors
`crates/oot_pause_demo` instead of maintaining a separate 2D approximation.
# Items-only Ambition inventory seam

This document captures the intended integration seam for trying the Lunex / OoT-style inventory without committing Ambition's gameplay code to that renderer.

## Ownership boundary

Ambition owns:

- the canonical item enum/catalog,
- `OwnedItems`, counts, equip state, and save data,
- health/mana/portal-gun/gameplay effects,
- pause/game-mode transitions,
- the canonical menu input frame.

`ambition_inventory_ui` owns:

- renderer-independent page/control data structures,
- shell lifecycle/effect messages,
- focus/hover/action metadata for rendered controls,
- optional renderer backends such as Lunex or a flat Bevy UI debug view.

The UI crate must not depend on Ambition's item types. Ambition should translate its item state into `ItemsOnlyPageSpec<PageId, Action>`, then map `MenuActionActivated<Action>` back into existing item/equip/use systems.

## Items-only first pass

The lowest-risk integration path is:

1. Build an `ItemsOnlyPageSpec` from `OwnedItems` and the current cursor state.
2. Convert it into a `MenuPageModel`.
3. Let the selected renderer display the page model.
4. On activation, emit the host-defined action enum.
5. Let Ambition apply the action through its existing item effects path.

The provided root example demonstrates that flow without opening a Bevy window:

```bash
cargo run --example items_only_seam
```

## Renderer swap criteria

The experiment remains easy to remove if all renderer-specific code consumes the same page model and emits the same host-defined actions. Removing a Lunex backend should not require changing item grants, save/load code, equip rules, or item-use side effects.

## Performance guardrails

For the items-only milestone, the renderer should be idle while the menu is closed. Cursor/focus changes should not require rebuilding every page. Unowned or disabled items should be rendered but should not carry actions into the renderer.

## Mock Ambition demo

The package now also has a host-side mock demo:

```bash
./run_demo.py mock
# or directly
cargo run --example ambition_mock_demo
```

This demo deliberately models equipment as host gameplay state, not UI state.
It has a one-item `held item` slot and a one-item `body` slot. Equipping a new
item in an occupied slot replaces the old item; activating the currently equipped
item unequips it. Consumables decrement their counts. Quest/key items are visible
but not actionable. Unowned items are visible but have no exported action.

The visual shell now also exercises the future four-face Ambition pause menu:
Items is functional, while Map, Quest, and System are placeholder faces. The
selected-item detail panel is fixed-size and uses a scrollbar / paged text
window, so long descriptions stress the UI without resizing item buttons or icon
slots. The cube selector and pause/unpause fold are also drawn inside fixed
bounds: animation changes absolute face-card positions and overlay door sizes,
not the measured size of the menu panel.

That behavior mirrors the desired Ambition integration: the UI crate describes
slots and emits a host-defined action, while Ambition decides whether the action
uses a consumable, toggles an item, replaces an equipped item, rotates to a
different menu face, or does nothing.
