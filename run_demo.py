#!/usr/bin/env python3
"""Run standalone ambition_inventory_ui demos.

This mirrors the main Ambition repository's run_game.sh ergonomics, but keeps
all commands scoped to the UI submodule.

Default:
  ./run_demo.py

launches the new visual Ambition mock inventory demo. That demo is interactive:
it borrows the exact Lunex inside-the-cube shell structure from
crates/oot_pause_demo while replacing only the page data/action layer with a
mock Ambition inventory host. The Items face is functional; Map, Quest, and
System are placeholders.
"""

from __future__ import annotations

import argparse
import os
import shlex
import subprocess
import sys
from pathlib import Path


MODE_ALIASES = {
    # Default / new host-style visual mock demo.
    "mock": "mock",
    "ambition-mock": "mock",
    "mock-demo": "mock",
    "ambition": "mock",

    # Non-window smoke path for CI / quick API validation.
    "mock-smoke": "mock-smoke",
    "smoke": "mock-smoke",
    "scripted": "mock-smoke",

    # Renderer-neutral items-only model seam example.
    "seam": "seam",
    "items-seam": "seam",
    "items-only": "seam",

    # Existing demos kept intact and reachable by name.
    "demo1": "demo1",
    "demo": "demo1",
    "original": "demo1",
    "ambition-demo": "demo1",
    "oot": "oot",
    "oot-pause": "oot",
    "pause": "oot",

    # Maintenance commands.
    "check": "check",
    "test": "test",
    "fmt": "fmt",
    "all": "all",
}


HELP_TEXT = """Interactive demo modes:
  ./run_demo.py             new Lunex/OoT-shell Ambition mock inventory demo (default)
  ./run_demo.py mock        same as default
  ./run_demo.py demo1       the original ambition_demo package
  ./run_demo.py demo        alias for demo1
  ./run_demo.py oot         the existing OoT pause demo package

Non-window / maintenance modes:
  ./run_demo.py mock-smoke  scripted validation of the mock host rules
  ./run_demo.py seam        renderer-neutral items-only page-model example
  ./run_demo.py check       cargo check --workspace --all-targets
  ./run_demo.py all         fmt-check + tests + non-window smoke examples
"""


def print_cmd(cmd: list[str], cwd: Path) -> None:
    print(f"cd {shlex.quote(str(cwd))}")
    print("+ " + " ".join(shlex.quote(part) for part in cmd))


def run(cmd: list[str], cwd: Path, dry_run: bool) -> int:
    print_cmd(cmd, cwd)
    if dry_run:
        return 0
    env = {**os.environ, "RUST_BACKTRACE": os.environ.get("RUST_BACKTRACE", "full")}
    return subprocess.call(cmd, cwd=str(cwd), env=env)


def cargo_run_args(args: argparse.Namespace, package_or_example: list[str]) -> list[str]:
    cmd = ["cargo", "run", *package_or_example]
    if args.no_default_features:
        cmd.append("--no-default-features")
    if args.features:
        cmd.extend(["--features", ",".join(args.features)])
    if args.release:
        cmd.append("--release")
    return cmd


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run standalone ambition_inventory_ui demos.",
        epilog=(
            HELP_TEXT
            + "\nExamples with extra cargo/demo args:\n"
            + "  ./run_demo.py mock --release\n"
            + "  ./run_demo.py mock -- --smoke\n"
            + "  ./run_demo.py demo1 release\n"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "tokens",
        nargs="*",
        help="mode aliases, options, or arguments after -- for the selected demo",
    )
    return interpret_tokens(parser, argv)


def interpret_tokens(parser: argparse.ArgumentParser, argv: list[str]) -> argparse.Namespace:
    mode = "mock"
    release = False
    no_default_features = False
    features: list[str] = []
    dry_run = False
    demo_args: list[str] = []

    tokens = list(argv)
    passthrough = False
    idx = 0
    while idx < len(tokens):
        token = tokens[idx]
        if passthrough:
            demo_args.append(token)
        elif token == "--":
            passthrough = True
        elif token in {"-r", "--release", "release"}:
            release = True
        elif token in {"--debug", "debug", "dev"}:
            release = False
        elif token == "--no-default-features":
            no_default_features = True
        elif token == "--features":
            idx += 1
            if idx >= len(tokens):
                parser.error("--features requires a comma-separated feature list")
            features.extend(part for part in tokens[idx].split(",") if part)
        elif token.startswith("--features="):
            features.extend(part for part in token.split("=", 1)[1].split(",") if part)
        elif token == "--dry-run":
            dry_run = True
        elif token in {"-h", "--help"}:
            parser.print_help()
            raise SystemExit(0)
        elif token.startswith("--"):
            parser.error(f"unknown option {token!r}; put demo args after --")
        else:
            mode = MODE_ALIASES.get(token, token)
            if mode not in MODE_ALIASES.values():
                choices = ", ".join(sorted(MODE_ALIASES))
                parser.error(f"unknown mode {token!r}; choices/aliases: {choices}")
        idx += 1

    return argparse.Namespace(
        mode=mode,
        release=release,
        no_default_features=no_default_features,
        features=features,
        dry_run=dry_run,
        demo_args=demo_args,
    )


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    repo_root = Path(__file__).resolve().parent

    if args.mode == "fmt":
        return run(["cargo", "fmt", "--all"], repo_root, args.dry_run)

    if args.mode == "test":
        return run(["cargo", "test", "-p", "ambition_inventory_ui", "--lib"], repo_root, args.dry_run)

    if args.mode == "check":
        return run(["cargo", "check", "--workspace", "--all-targets"], repo_root, args.dry_run)

    if args.mode == "all":
        steps = [
            ["cargo", "fmt", "--all", "--check"],
            ["cargo", "test", "-p", "ambition_inventory_ui", "--lib"],
            ["cargo", "run", "--example", "items_only_seam"],
            ["cargo", "run", "-p", "ambition_mock_demo", "--", "--smoke"],
        ]
        for cmd in steps:
            code = run(cmd, repo_root, args.dry_run)
            if code:
                return code
        return 0

    if args.mode == "mock":
        cmd = cargo_run_args(args, ["-p", "ambition_mock_demo"])
    elif args.mode == "mock-smoke":
        cmd = cargo_run_args(args, ["-p", "ambition_mock_demo"])
        cmd.extend(["--", "--smoke"])
    elif args.mode == "seam":
        cmd = cargo_run_args(args, ["--example", "items_only_seam"])
    elif args.mode == "demo1":
        cmd = cargo_run_args(args, ["-p", "ambition_demo"])
    elif args.mode == "oot":
        cmd = cargo_run_args(args, ["-p", "oot_pause_demo"])
    else:
        raise AssertionError(f"unhandled mode: {args.mode}")

    if args.demo_args:
        if "--" not in cmd:
            cmd.append("--")
        cmd.extend(args.demo_args)
    return run(cmd, repo_root, args.dry_run)


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
