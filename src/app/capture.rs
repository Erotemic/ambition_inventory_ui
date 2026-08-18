// README capture is a deterministic input-driven showcase, not a separate demo
// implementation. Each logical capture frame injects at most one keyboard pulse
// into Bevy's normal ButtonInput<KeyCode> resource, so the same pause, navigation,
// page-turn, activation, and C-button systems used interactively handle the movie.

const README_DEMO_KEYFRAMES_MS: &[(u32, KeyCode)] = &[
    // Show both sides of the pause transition.
    (500, KeyCode::KeyP),
    (1350, KeyCode::KeyP),

    // Items: move with the arrow keys and equip Longshot, Ice Arrow, Light Arrow.
    (2250, KeyCode::ArrowDown),
    (2500, KeyCode::KeyZ),
    (2900, KeyCode::ArrowRight),
    (3150, KeyCode::KeyX),
    (4000, KeyCode::ArrowDown),
    (4250, KeyCode::KeyC),

    // Walk the cursor onto the right page arrow, then through it to rotate.
    (5200, KeyCode::ArrowRight),
    (5400, KeyCode::ArrowRight),
    (5600, KeyCode::ArrowRight),

    // Equipment: interact with several rows while the page is active.
    (6450, KeyCode::ArrowRight),
    (6650, KeyCode::Enter),
    (6900, KeyCode::ArrowDown),
    (7100, KeyCode::Enter),
    (7350, KeyCode::ArrowDown),
    (7550, KeyCode::Enter),
    (7900, KeyCode::ArrowRight),
    (8100, KeyCode::ArrowRight),

    // Quest: activate a medallion, move to another, then down to a stone.
    (9000, KeyCode::Enter),
    (9250, KeyCode::ArrowLeft),
    (9450, KeyCode::Enter),
    (9700, KeyCode::ArrowDown),
    (9900, KeyCode::Enter),
    // Move across the lower-right cluster onto the page arrow and rotate to Map.
    (10250, KeyCode::ArrowRight),
    (10450, KeyCode::ArrowRight),
    (10650, KeyCode::ArrowRight),
    (10850, KeyCode::ArrowRight),

    // Map: visit several markers using ordinary spatial navigation.
    (11600, KeyCode::Enter),
    (11850, KeyCode::ArrowLeft),
    (12050, KeyCode::Enter),
    (12300, KeyCode::ArrowDown),
    (12500, KeyCode::Enter),

    // Return to Items and restore the default Bow assignment for a cleaner loop.
    (13000, KeyCode::PageDown),
    (13700, KeyCode::KeyC),
];

fn readme_animation_delta_secs(time: &Time, capture: Option<&ReadmeCapture>) -> f32 {
    match capture {
        Some(capture) if capture.should_step_simulation() => capture.frame_delta_secs(),
        Some(_) => 0.0,
        None => time.delta_secs(),
    }
}

fn readme_demo_key_for_frame(frame: u32, fps: u32) -> Option<KeyCode> {
    README_DEMO_KEYFRAMES_MS.iter().find_map(|(millis, key)| {
        let event_frame = ((*millis as u64 * fps as u64 + 500) / 1000) as u32;
        (event_frame == frame).then_some(*key)
    })
}

fn drive_readme_demo_input(
    mut keys: ResMut<ButtonInput<KeyCode>>,
    readme_capture: Option<Res<ReadmeCapture>>,
    mut previous_key: Local<Option<KeyCode>>,
) {
    let Some(readme_capture) = readme_capture else {
        return;
    };

    if let Some(key) = previous_key.take() {
        keys.release(key);
    }
    if !readme_capture.should_step_simulation() {
        return;
    }
    if let Some(key) = readme_demo_key_for_frame(readme_capture.next_frame, readme_capture.fps) {
        keys.press(key);
        *previous_key = Some(key);
    }
}
