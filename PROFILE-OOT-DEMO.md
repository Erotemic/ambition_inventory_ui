# OoT demo profiling toggles

The demo reads these environment flags at startup. Any value except `0` or `false` enables the flag.

```bash
OOT_PROFILE_NO_HUD=1 cargo run -p oot_pause_demo --release
OOT_PROFILE_NO_SIDE_FACES=1 cargo run -p oot_pause_demo --release
OOT_PROFILE_NO_PICKING=1 cargo run -p oot_pause_demo --release
OOT_PROFILE_NO_OIT=1 cargo run -p oot_pause_demo --release
OOT_PROFILE_NO_FXAA=1 cargo run -p oot_pause_demo --release
OOT_PROFILE_NO_REBUILDS=1 cargo run -p oot_pause_demo --release
OOT_PROFILE_NO_LUNEX=1 cargo run -p oot_pause_demo --release
OOT_PROFILE_LOG_REBUILDS=1 cargo run -p oot_pause_demo --release
```

Suggested bisection order:

1. Baseline release FPS and flamegraph.
2. `OOT_PROFILE_NO_OIT=1 OOT_PROFILE_NO_FXAA=1` to check post-processing / transparency cost.
3. `OOT_PROFILE_NO_HUD=1` to check the second camera and HUD overlay cost.
4. `OOT_PROFILE_NO_SIDE_FACES=1` to check whether four Lunex faces are the issue.
5. `OOT_PROFILE_NO_PICKING=1` to check Bevy picking overhead.
6. `OOT_PROFILE_NO_REBUILDS=1` while interacting to check despawn/rebuild churn.
7. `OOT_PROFILE_NO_LUNEX=1` as the coarse bevy_lunex isolation test.

For cargo flamegraph on Linux, `kernel.perf_event_paranoid=4` is the restrictive direction. Use root or temporarily lower it, for example:

```bash
sudo sysctl kernel.perf_event_paranoid=-1
CARGO_PROFILE_RELEASE_DEBUG=true cargo flamegraph -p oot_pause_demo
```

Restore your original setting afterward if needed.
