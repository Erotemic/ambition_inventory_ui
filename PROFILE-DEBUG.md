# Debug-mode profiling notes

This project now uses a Bevy-style dev profile:

```toml
[profile.dev]
opt-level = 1

[profile.dev.package."*"]
opt-level = 3
```

That keeps local workspace crates debuggable while avoiding fully-unoptimized
Bevy/wgpu/Lunex dependency code in normal `cargo run` development builds.

## Run the demo in dev mode

```bash
cargo run -p oot_pause_demo
```

## Profile the dev build with cargo-flamegraph

```bash
sudo sysctl kernel.perf_event_paranoid=-1
CARGO_PROFILE_DEV_DEBUG=true cargo flamegraph --dev --no-inline --freq 49 -p oot_pause_demo
```

## Capture only steady-state after startup

```bash
cargo build -p oot_pause_demo

target/debug/oot_pause_demo &
PID=$!
sleep 5
perf record -F 49 -g -p "$PID" -- sleep 10
kill "$PID"
perf script > out-debug.perf
inferno-collapse-perf out-debug.perf > stacks-debug.folded
inferno-flamegraph stacks-debug.folded > flamegraph-debug-idle.svg
```

## Useful bisection runs

```bash
OOT_PROFILE_NO_HUD=1 cargo run -p oot_pause_demo
OOT_PROFILE_NO_SIDE_FACES=1 cargo run -p oot_pause_demo
OOT_PROFILE_NO_REBUILDS=1 cargo run -p oot_pause_demo
OOT_PROFILE_NO_LUNEX=1 cargo run -p oot_pause_demo
```

If the dev profile jumps from ~30 FPS to something close to the broader game,
then the issue was mostly unoptimized dependencies. If it stays near 30 FPS, use
`flamegraph-debug-idle.svg` to compare against the release idle profile.
