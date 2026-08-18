#!/usr/bin/env python3
"""Capture the scripted OoT pause-menu showcase and encode an animated WebP."""

from __future__ import annotations

import argparse
import math
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output",
        default="animation.webp",
        help="Animated WebP output path relative to the repository root.",
    )
    parser.add_argument(
        "--duration",
        type=float,
        default=14.5,
        help="Seconds of the scripted showcase to capture.",
    )
    parser.add_argument(
        "--fps",
        type=int,
        default=12,
        help="Simulation/capture frame rate. 12 fps keeps README media compact.",
    )
    parser.add_argument("--width", type=int, default=1180, help="Capture window width.")
    parser.add_argument("--height", type=int, default=760, help="Capture window height.")
    parser.add_argument(
        "--output-width",
        type=int,
        default=720,
        help="Starting WebP width; the encoder may reduce it to meet --max-mib.",
    )
    parser.add_argument(
        "--quality",
        type=float,
        default=50.0,
        help="Starting libwebp quality from 0 to 100.",
    )
    parser.add_argument(
        "--max-mib",
        type=float,
        default=5.0,
        help="Soft size target. Encoding is retried smaller if necessary; 0 disables.",
    )
    parser.add_argument(
        "--warmup-frames",
        type=int,
        default=30,
        help="Rendered frames to wait before capture so textures/fonts are resident.",
    )
    parser.add_argument(
        "--keep-frames",
        action="store_true",
        help="Copy captured PNGs to target/readme-animation-frames for inspection.",
    )
    return parser.parse_args()


def checked_run(
    command: list[str],
    *,
    cwd: Path,
    env: dict[str, str] | None = None,
) -> None:
    subprocess.run(command, cwd=cwd, env=env, check=True)


def require_tool(name: str) -> None:
    if shutil.which(name) is None:
        raise SystemExit(f"{name} is required but was not found on PATH")


def encode_webp(
    *,
    repo_root: Path,
    base_env: dict[str, str],
    frames_dir: Path,
    encoded_path: Path,
    capture_fps: int,
    output_fps: int,
    output_width: int,
    quality: float,
) -> None:
    checked_run(
        [
            "ffmpeg",
            "-hide_banner",
            "-loglevel",
            "warning",
            "-y",
            "-framerate",
            str(capture_fps),
            "-start_number",
            "0",
            "-i",
            str(frames_dir / "frame_%04d.png"),
            "-vf",
            f"fps={output_fps},scale={output_width}:-2:flags=lanczos",
            "-an",
            "-loop",
            "0",
            "-c:v",
            "libwebp_anim",
            "-lossless",
            "0",
            "-preset",
            "drawing",
            "-compression_level",
            "6",
            "-quality",
            f"{quality:.1f}",
            "-pix_fmt",
            "yuv420p",
            str(encoded_path),
        ],
        cwd=repo_root,
        env=base_env,
    )


def encoding_attempts(width: int, fps: int, quality: float) -> list[tuple[int, int, float]]:
    candidates = [
        (width, fps, quality),
        (width, fps, max(36.0, quality - 8.0)),
        (max(480, min(width, int(width * 0.9))), min(fps, 10), max(34.0, quality - 12.0)),
        (max(480, min(width, int(width * 0.82))), min(fps, 10), max(30.0, quality - 18.0)),
    ]
    unique: list[tuple[int, int, float]] = []
    for candidate in candidates:
        if candidate not in unique:
            unique.append(candidate)
    return unique


def main() -> int:
    args = parse_args()
    if args.duration <= 0.0:
        raise SystemExit("--duration must be positive")
    if args.fps <= 0:
        raise SystemExit("--fps must be positive")
    if args.width <= 0 or args.height <= 0 or args.output_width <= 0:
        raise SystemExit("capture and output dimensions must be positive")
    if not 0.0 <= args.quality <= 100.0:
        raise SystemExit("--quality must be between 0 and 100")
    if args.max_mib < 0.0:
        raise SystemExit("--max-mib cannot be negative")
    if args.warmup_frames < 0:
        raise SystemExit("--warmup-frames cannot be negative")

    require_tool("cargo")
    require_tool("ffmpeg")

    repo_root = Path(__file__).resolve().parent
    output_path = Path(args.output)
    if not output_path.is_absolute():
        output_path = repo_root / output_path
    output_path = output_path.resolve()
    output_path.parent.mkdir(parents=True, exist_ok=True)

    frame_count = math.ceil(args.duration * args.fps)
    base_env = {
        **os.environ,
        "RUST_BACKTRACE": os.environ.get("RUST_BACKTRACE", "full"),
    }

    with tempfile.TemporaryDirectory(prefix="oot-readme-capture-") as temp_dir:
        frames_dir = Path(temp_dir) / "frames"
        frames_dir.mkdir(parents=True)

        capture_env = {
            **base_env,
            "OOT_CAPTURE_FRAMES_DIR": str(frames_dir),
            "OOT_CAPTURE_FRAME_COUNT": str(frame_count),
            "OOT_CAPTURE_FPS": str(args.fps),
            "OOT_CAPTURE_WARMUP_FRAMES": str(args.warmup_frames),
            "OOT_CAPTURE_WINDOW_WIDTH": str(args.width),
            "OOT_CAPTURE_WINDOW_HEIGHT": str(args.height),
        }
        checked_run([sys.executable, "run_demo.py"], cwd=repo_root, env=capture_env)

        frames = sorted(frames_dir.glob("frame_*.png"))
        if len(frames) != frame_count:
            raise SystemExit(
                f"capture produced {len(frames)} frames; expected {frame_count}"
            )

        with tempfile.NamedTemporaryFile(
            prefix=f".{output_path.stem}-",
            suffix=".webp",
            dir=output_path.parent,
            delete=False,
        ) as temp_output:
            encoded_path = Path(temp_output.name)

        max_bytes = int(args.max_mib * 1024 * 1024) if args.max_mib > 0.0 else None
        chosen: tuple[int, int, float] | None = None
        try:
            for output_width, output_fps, quality in encoding_attempts(
                args.output_width, args.fps, args.quality
            ):
                encode_webp(
                    repo_root=repo_root,
                    base_env=base_env,
                    frames_dir=frames_dir,
                    encoded_path=encoded_path,
                    capture_fps=args.fps,
                    output_fps=output_fps,
                    output_width=output_width,
                    quality=quality,
                )
                chosen = (output_width, output_fps, quality)
                if max_bytes is None or encoded_path.stat().st_size <= max_bytes:
                    break
            encoded_path.replace(output_path)
        finally:
            encoded_path.unlink(missing_ok=True)

        if args.keep_frames:
            kept_frames_dir = repo_root / "target" / "readme-animation-frames"
            if kept_frames_dir.exists():
                shutil.rmtree(kept_frames_dir)
            kept_frames_dir.parent.mkdir(parents=True, exist_ok=True)
            shutil.copytree(frames_dir, kept_frames_dir)
            print(f"Kept captured frames in {kept_frames_dir}")

    size_mib = output_path.stat().st_size / (1024 * 1024)
    width, output_fps, quality = chosen or (args.output_width, args.fps, args.quality)
    print(
        f"Wrote {output_path} ({size_mib:.2f} MiB, "
        f"{width}px, {output_fps} fps, quality {quality:.0f})"
    )
    if args.max_mib > 0.0 and size_mib > args.max_mib:
        print(
            f"Warning: animation remains above the {args.max_mib:.1f} MiB soft target; "
            "lower --output-width, --fps, or --quality for a smaller file."
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
