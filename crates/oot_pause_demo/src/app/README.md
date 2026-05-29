# OoT pause demo source layout

This is the first structural split of the original single-file proof of concept.
The files are currently `include!`d into `app.rs` so the split is behavior-preserving
and does not require broad visibility changes while the demo is still moving fast.

- `state.rs`: demo resources, enums, action routing, and state mutation helpers.
- `data.rs`: hard-coded OoT-inspired items, equipment, map markers, quest icons, and songs.
- `models.rs`: data-driven `MenuPageModel` construction for pages and HUD.
- `render.rs`: Lunex entity construction and visual styling helpers.
- `systems.rs`: app setup, cameras, HUD/page spawning, FPS overlay, and face transforms.
- `input.rs`: keyboard/mouse/gamepad handling, hit testing, animation, and geometry helpers.

A later cleanup can turn these includes into normal Rust submodules once the demo API
settles enough to make visibility boundaries useful instead of noisy.
