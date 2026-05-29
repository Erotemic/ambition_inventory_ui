# OoT Pause Demo Idle Flamegraph Notes

Input profile: `flamegraph-idle.svg` / `stacks.folded` captured after startup.

The idle profile does not primarily implicate `bevy_lunex` layout. Named Lunex work is very small in this capture, while the largest recognizable steady-state cost is Bevy's 3D/PBR render path and view specialization over many mesh/material UI entities.

Largest recognizable leaves in the uploaded folded stacks:

- `bevy_pbr::render::mesh::check_views_need_specialization`: ~6.7%
- `fixedbitset::FixedBitSet::is_disjoint`: ~6.8%, likely schedule/render filtering overhead around systems/views
- `__memmove_avx_unaligned_erms`: ~5.5%
- NVIDIA driver / GPU compiler / GL/Vulkan symbols: visible several percent
- `bevy_lunex::system_pipe_sprite_size_from_dimension`: ~0.06%
- `oot_pause_demo::tag_hud_entity_recursive`: ~0.08%

Interpretation:

1. The problem is likely steady-state rendering of too many 3D StandardMaterial UI meshes through PBR cameras, not Lunex layout itself.
2. The separate HUD camera is still useful for correctness, but every extra camera/view increases render specialization and draw preparation work.
3. The opposite cube face cannot be visible from the inside-camera view, so keeping it alive costs render work for no visual benefit.
4. The FPS overlay should not rewrite text every frame, because text changes can trigger layout/glyph/mesh work.

This overlay makes conservative first-step changes:

- OIT and FXAA are disabled by default and opt-in via `OOT_ENABLE_OIT=1` / `OOT_ENABLE_FXAA=1`.
- Only the active, viewer-left, and viewer-right pause faces are spawned.
- The FPS overlay samples every frame but rewrites the Text entity only four times per second.
- The per-frame HUD render-layer retagging system is removed from the schedule because spawned HUD children are already tagged with the HUD render layer.

Next things to profile if this is still slow:

- Replace the 3D HUD overlay with a true 2D overlay / sprite UI so the persistent HUD does not need a second PBR camera.
- Use a simpler unlit custom material instead of `StandardMaterial` for the Lunex planes.
- Update face entities in place instead of despawning/rebuilding controls when only selection/highlight changes.
