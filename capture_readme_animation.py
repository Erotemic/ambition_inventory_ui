#!/usr/bin/env python3
"""Capture one seamless pause-cube rotation and encode it as an animated WebP."""

from __future__ import annotations

import argparse
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
        default="screenshot.webp",
        help="Animated WebP output path relative to the repository root.",
    )
    parser.add_argument(
        "--frame-count",
        type=int,
        default=60,
        help="Frames captured across one exact 360-degree rotation.",
    )
    parser.add_argument("--fps", type=int, default=20, help="Playback frame rate.")
    parser.add_argument("--width", type=int, default=1180, help="Capture window width.")
    parser.add_argument("--height", type=int, default=760, help="Capture window height.")
    parser.add_argument(
        "--output-width",
        type=int,
        default=900,
        help="Final WebP width; height is derived from the captured aspect ratio.",
    )
    parser.add_argument(
        "--quality",
        type=float,
        default=62.0,
        help="libwebp quality from 0 to 100.",
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


def main() -> int:
    args = parse_args()
    if args.frame_count <= 0:
        raise SystemExit("--frame-count must be positive")
    if args.fps <= 0:
        raise SystemExit("--fps must be positive")
    if args.width <= 0 or args.height <= 0 or args.output_width <= 0:
        raise SystemExit("capture and output dimensions must be positive")
    if not 0.0 <= args.quality <= 100.0:
        raise SystemExit("--quality must be between 0 and 100")
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
            "OOT_CAPTURE_FRAME_COUNT": str(args.frame_count),
            "OOT_CAPTURE_WARMUP_FRAMES": str(args.warmup_frames),
            "OOT_CAPTURE_WINDOW_WIDTH": str(args.width),
            "OOT_CAPTURE_WINDOW_HEIGHT": str(args.height),
        }
        checked_run([sys.executable, "run_demo.py"], cwd=repo_root, env=capture_env)

        frames = sorted(frames_dir.glob("frame_*.png"))
        if len(frames) != args.frame_count:
            raise SystemExit(
                f"capture produced {len(frames)} frames; expected {args.frame_count}"
            )

        with tempfile.NamedTemporaryFile(
            prefix=f".{output_path.stem}-",
            suffix=".webp",
            dir=output_path.parent,
            delete=False,
        ) as temp_output:
            encoded_path = Path(temp_output.name)
        try:
            checked_run(
                [
                    "ffmpeg",
                    "-hide_banner",
                    "-loglevel",
                    "warning",
                    "-y",
                    "-framerate",
                    str(args.fps),
                    "-start_number",
                    "0",
                    "-i",
                    str(frames_dir / "frame_%04d.png"),
                    "-vf",
                    f"scale={args.output_width}:-2:flags=lanczos",
                    "-an",
                    "-loop",
                    "0",
                    "-c:v",
                    "libwebp_anim",
                    "-lossless",
                    "0",
                    "-preset",
                    "drawing",
                    "-quality",
                    str(args.quality),
                    "-pix_fmt",
                    "yuv420p",
                    str(encoded_path),
                ],
                cwd=repo_root,
                env=base_env,
            )
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
    print(f"Wrote {output_path} ({size_mib:.2f} MiB)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
