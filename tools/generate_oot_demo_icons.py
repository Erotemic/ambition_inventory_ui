#!/usr/bin/env python3
"""Generate stylized placeholder icons for the OoT pause-menu demo.

The icons are original, procedural approximations meant for the demo UI. They
match the filenames used by `crates/oot_pause_demo/src/main.rs` and follow the
same high-level item families seen in the OoT decomp references, especially
`include/item.h` and `src/code/z_inventory.c`, without copying game assets.

Usage:
    python3 tools/generate_oot_demo_icons.py
    python3 tools/generate_oot_demo_icons.py --size 96 --out assets/icons/oot

Requires Pillow (`python3 -m pip install pillow`) for antialiased PNG output.
"""
from __future__ import annotations

import argparse
import math
from pathlib import Path
from typing import Callable, Dict, Iterable, Tuple

try:
    from PIL import Image, ImageDraw, ImageFilter, ImageFont
except ImportError as ex:  # pragma: no cover
    raise SystemExit("This generator requires Pillow. Install with: python3 -m pip install pillow") from ex

Color = Tuple[int, int, int, int]
BG = (0, 0, 0, 0)
INK = (246, 238, 194, 255)
DARK = (30, 23, 36, 255)
GOLD = (246, 194, 70, 255)
YELLOW = (255, 232, 92, 255)
RED = (229, 72, 54, 255)
BLUE = (74, 154, 242, 255)
GREEN = (76, 190, 92, 255)
PURPLE = (161, 94, 220, 255)
CYAN = (96, 230, 232, 255)
ORANGE = (238, 128, 45, 255)
GRAY = (154, 164, 180, 255)
SILVER = (215, 222, 230, 255)
BROWN = (126, 77, 40, 255)
PINK = (255, 153, 194, 255)
BLACK = (12, 12, 16, 255)
WHITE = (255, 255, 245, 255)


_FONT_CACHE: dict[tuple[int, bool], ImageFont.ImageFont] = {}


def font_px(px: int, *, bold: bool = True) -> ImageFont.ImageFont:
    key = (px, bold)
    if key not in _FONT_CACHE:
        names = [
            "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf" if bold else "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/local/share/fonts/DejaVuSans-Bold.ttf" if bold else "/usr/local/share/fonts/DejaVuSans.ttf",
        ]
        for name in names:
            try:
                _FONT_CACHE[key] = ImageFont.truetype(name, px)
                break
            except OSError:
                continue
        else:
            _FONT_CACHE[key] = ImageFont.load_default()
    return _FONT_CACHE[key]


def centered_text(draw: ImageDraw.ImageDraw, xy_pos: tuple[float, float], text: str, px: int, fill: Color, *, stroke: Color | None = None, bold: bool = True) -> None:
    kwargs = {"font": font_px(px, bold=bold), "anchor": "mm", "fill": fill}
    if stroke is not None:
        kwargs.update({"stroke_width": max(1, px // 14), "stroke_fill": stroke})
    draw.text(xy_pos, text, **kwargs)


def rgba(hex_rgb: str, a: int = 255) -> Color:
    hex_rgb = hex_rgb.lstrip("#")
    return tuple(int(hex_rgb[i:i+2], 16) for i in (0, 2, 4)) + (a,)  # type: ignore[return-value]


def scale_points(points: Iterable[Tuple[float, float]], s: int) -> list[Tuple[float, float]]:
    return [(x * s / 64.0, y * s / 64.0) for x, y in points]


def icon_canvas(size: int) -> tuple[Image.Image, ImageDraw.ImageDraw]:
    hi = size * 4
    img = Image.new("RGBA", (hi, hi), BG)
    return img, ImageDraw.Draw(img)


def finish(img: Image.Image, size: int) -> Image.Image:
    img = img.filter(ImageFilter.GaussianBlur(radius=0.0))
    return img.resize((size, size), Image.Resampling.LANCZOS)


def stroke_width(size: int, units: float) -> int:
    return max(1, round(units * size * 4 / 64.0))


def pts(points: Iterable[Tuple[float, float]], size: int) -> list[Tuple[float, float]]:
    return scale_points(points, size * 4)


def xy(box: Tuple[float, float, float, float], size: int) -> Tuple[float, float, float, float]:
    k = size * 4 / 64.0
    return tuple(v * k for v in box)  # type: ignore[return-value]


def glow(draw: ImageDraw.ImageDraw, box: Tuple[float, float, float, float], fill: Color, size: int) -> None:
    draw.ellipse(xy(box, size), fill=(fill[0], fill[1], fill[2], 42))


def badge(draw: ImageDraw.ImageDraw, size: int, fill: Color = rgba("#172351", 150)) -> None:
    draw.rounded_rectangle(xy((6, 6, 58, 58), size), radius=stroke_width(size, 12), fill=fill, outline=rgba("#7a8ee8", 130), width=stroke_width(size, 1.2))


def polygon(draw: ImageDraw.ImageDraw, size: int, points, fill, outline=DARK, width=1.6):
    draw.polygon(pts(points, size), fill=fill)
    draw.line(pts(list(points) + [points[0]], size), fill=outline, width=stroke_width(size, width), joint="curve")


def line(draw: ImageDraw.ImageDraw, size: int, points, fill=INK, width=3, joint="curve"):
    draw.line(pts(points, size), fill=fill, width=stroke_width(size, width), joint=joint)


def ellipse(draw: ImageDraw.ImageDraw, size: int, box, fill, outline=DARK, width=1.5):
    draw.ellipse(xy(box, size), fill=fill, outline=outline, width=stroke_width(size, width))


def rect(draw: ImageDraw.ImageDraw, size: int, box, fill, outline=DARK, width=1.5, radius=0):
    draw.rounded_rectangle(xy(box, size), radius=stroke_width(size, radius), fill=fill, outline=outline, width=stroke_width(size, width))


def draw_arrow_icon(size: int, direction: str) -> Image.Image:
    img, draw = icon_canvas(size)
    badge(draw, size, rgba("#0b1730", 130))
    if direction == "left":
        polygon(draw, size, [(16, 32), (34, 14), (34, 25), (50, 25), (50, 39), (34, 39), (34, 50)], GOLD)
    else:
        polygon(draw, size, [(48, 32), (30, 14), (30, 25), (14, 25), (14, 39), (30, 39), (30, 50)], GOLD)
    return finish(img, size)


def draw_stick(size: int) -> Image.Image:
    img, draw = icon_canvas(size); badge(draw, size)
    line(draw, size, [(19, 51), (28, 35), (33, 20), (43, 10)], BROWN, 7)
    line(draw, size, [(22, 48), (31, 32), (36, 18), (46, 8)], rgba("#d79b5c"), 3)
    line(draw, size, [(34, 20), (47, 21)], GREEN, 3)
    return finish(img, size)


def draw_nut(size: int) -> Image.Image:
    img, draw = icon_canvas(size); badge(draw, size)
    ellipse(draw, size, (16, 20, 48, 52), rgba("#9b6a35"), width=2)
    polygon(draw, size, [(24, 20), (31, 11), (40, 20)], GREEN)
    ellipse(draw, size, (25, 28, 39, 42), rgba("#5f391e"), outline=rgba("#2c1b11"), width=1)
    return finish(img, size)


def draw_bomb(size: int) -> Image.Image:
    img, draw = icon_canvas(size); badge(draw, size)
    ellipse(draw, size, (15, 22, 47, 54), rgba("#23293f"), outline=SILVER, width=2)
    rect(draw, size, (30, 14, 40, 25), rgba("#5b4738"), width=1, radius=2)
    line(draw, size, [(36, 15), (43, 9), (51, 12)], ORANGE, 3)
    polygon(draw, size, [(49, 9), (55, 7), (53, 14)], YELLOW, outline=ORANGE)
    return finish(img, size)


def draw_bow(size: int) -> Image.Image:
    img, draw = icon_canvas(size); badge(draw, size)
    line(draw, size, [(43, 10), (27, 18), (22, 32), (27, 46), (43, 54)], rgba("#a96b37"), 5)
    line(draw, size, [(43, 10), (33, 32), (43, 54)], INK, 1.5)
    line(draw, size, [(16, 32), (50, 32)], SILVER, 2)
    polygon(draw, size, [(51, 32), (43, 28), (43, 36)], RED, outline=DARK)
    return finish(img, size)


def draw_arrow(size: int, color: Color) -> Image.Image:
    img, draw = icon_canvas(size); badge(draw, size)
    glow(draw, (10, 10, 54, 54), color, size)
    line(draw, size, [(15, 48), (48, 15)], SILVER, 3)
    polygon(draw, size, [(48, 15), (46, 30), (35, 19)], color, outline=WHITE, width=1.4)
    line(draw, size, [(17, 48), (10, 54), (22, 51)], color, 2)
    return finish(img, size)


def draw_spell(size: int, color: Color, letter: str) -> Image.Image:
    img, draw = icon_canvas(size); badge(draw, size)
    glow(draw, (7, 7, 57, 57), color, size)
    ellipse(draw, size, (14, 14, 50, 50), color, outline=WHITE, width=1.8)
    draw.arc(xy((18, 18, 46, 46), size), 20, 330, fill=DARK, width=stroke_width(size, 3))
    draw.text((size*4*0.39, size*4*0.27), letter, fill=WHITE, anchor="mm")
    return finish(img, size)


def draw_slingshot(size: int) -> Image.Image:
    img, draw = icon_canvas(size); badge(draw, size)
    line(draw, size, [(32, 52), (32, 30), (22, 14)], BROWN, 6)
    line(draw, size, [(32, 30), (43, 14)], BROWN, 6)
    line(draw, size, [(23, 16), (32, 32), (42, 16)], rgba("#e0d6aa"), 2)
    ellipse(draw, size, (27, 27, 37, 37), rgba("#81562e"), width=1)
    return finish(img, size)


def draw_ocarina(size: int) -> Image.Image:
    img, draw = icon_canvas(size); badge(draw, size)
    ellipse(draw, size, (14, 24, 47, 48), BLUE, outline=DARK, width=2)
    polygon(draw, size, [(42, 28), (56, 23), (48, 38)], BLUE, outline=DARK)
    for x0, y0 in [(26, 31), (34, 32), (30, 39), (40, 39)]:
        ellipse(draw, size, (x0-2, y0-2, x0+2, y0+2), DARK, outline=DARK, width=.5)
    return finish(img, size)


def draw_bombchu(size: int) -> Image.Image:
    img, draw = icon_canvas(size); badge(draw, size)
    polygon(draw, size, [(13, 39), (25, 24), (47, 23), (55, 36), (41, 45), (23, 48)], rgba("#4962b6"))
    polygon(draw, size, [(20, 24), (14, 15), (29, 22)], PINK, outline=DARK)
    ellipse(draw, size, (40, 28, 46, 34), RED, outline=DARK, width=1)
    line(draw, size, [(19, 44), (12, 52)], YELLOW, 3)
    return finish(img, size)


def draw_hookshot(size: int) -> Image.Image:
    img, draw = icon_canvas(size); badge(draw, size)
    rect(draw, size, (19, 36, 45, 49), rgba("#5c5d68"), outline=SILVER, width=1.5, radius=3)
    line(draw, size, [(32, 37), (32, 13)], SILVER, 4)
    polygon(draw, size, [(32, 10), (23, 24), (32, 20), (41, 24)], RED, outline=DARK)
    return finish(img, size)


def draw_boomerang(size: int) -> Image.Image:
    img, draw = icon_canvas(size); badge(draw, size)
    polygon(draw, size, [(13, 43), (31, 15), (51, 21), (40, 30), (32, 26), (23, 47)], rgba("#6fb7e9"), outline=WHITE)
    return finish(img, size)


def draw_lens(size: int) -> Image.Image:
    img, draw = icon_canvas(size); badge(draw, size)
    ellipse(draw, size, (15, 15, 49, 49), rgba("#5ab1e8", 170), outline=GOLD, width=3)
    ellipse(draw, size, (24, 24, 40, 40), PURPLE, outline=WHITE, width=1.4)
    line(draw, size, [(41, 42), (53, 54)], GOLD, 4)
    return finish(img, size)


def draw_beans(size: int) -> Image.Image:
    img, draw = icon_canvas(size); badge(draw, size)
    for box, col in [((17, 35, 34, 50), GREEN), ((30, 18, 48, 34), rgba("#89d455")), ((21, 18, 36, 35), rgba("#5baa38"))]:
        ellipse(draw, size, box, col, outline=DARK, width=1.2)
    return finish(img, size)


def draw_hammer(size: int) -> Image.Image:
    img, draw = icon_canvas(size); badge(draw, size)
    line(draw, size, [(24, 51), (41, 27)], BROWN, 6)
    rect(draw, size, (25, 13, 53, 28), rgba("#6c7383"), outline=SILVER, width=2, radius=3)
    return finish(img, size)


def draw_bottle(size: int, contents: Color | None = None) -> Image.Image:
    img, draw = icon_canvas(size); badge(draw, size)
    rect(draw, size, (26, 10, 38, 23), rgba("#a9dbff", 120), outline=SILVER, width=1.3, radius=2)
    polygon(draw, size, [(21, 23), (43, 23), (49, 52), (15, 52)], rgba("#bfe8ff", 95), outline=SILVER, width=1.5)
    if contents:
        polygon(draw, size, [(18, 41), (46, 41), (43, 51), (21, 51)], contents, outline=contents)
    return finish(img, size)


def draw_poe(size: int) -> Image.Image:
    img, draw = icon_canvas(size); badge(draw, size)
    ellipse(draw, size, (17, 13, 47, 48), rgba("#8247c5", 210), outline=WHITE, width=1.5)
    polygon(draw, size, [(18, 40), (25, 55), (32, 42), (39, 55), (47, 40)], rgba("#8247c5", 210), outline=WHITE)
    ellipse(draw, size, (24, 26, 28, 31), YELLOW, outline=YELLOW, width=1)
    ellipse(draw, size, (36, 26, 40, 31), YELLOW, outline=YELLOW, width=1)
    return finish(img, size)


def draw_scroll(size: int) -> Image.Image:
    img, draw = icon_canvas(size); badge(draw, size)
    rect(draw, size, (16, 18, 48, 46), rgba("#f1d48a"), outline=BROWN, width=2, radius=4)
    line(draw, size, [(22, 26), (42, 26)], BROWN, 1.3)
    line(draw, size, [(22, 34), (39, 34)], BROWN, 1.3)
    polygon(draw, size, [(42, 18), (48, 24), (42, 24)], rgba("#caa965"), outline=BROWN, width=1)
    return finish(img, size)


def draw_mask(size: int) -> Image.Image:
    img, draw = icon_canvas(size); badge(draw, size)
    polygon(draw, size, [(16, 20), (32, 12), (48, 20), (45, 47), (32, 55), (19, 47)], rgba("#d79452"))
    ellipse(draw, size, (23, 28, 29, 35), BLACK, outline=BLACK, width=1)
    ellipse(draw, size, (35, 28, 41, 35), BLACK, outline=BLACK, width=1)
    line(draw, size, [(26, 44), (38, 44)], DARK, 2)
    return finish(img, size)


def draw_sword(size: int, blade: Color, hilt: Color) -> Image.Image:
    img, draw = icon_canvas(size); badge(draw, size)
    polygon(draw, size, [(33, 8), (38, 35), (32, 51), (26, 35), (31, 8)], blade, outline=WHITE)
    line(draw, size, [(21, 38), (43, 38)], hilt, 5)
    ellipse(draw, size, (27, 48, 37, 58), hilt, outline=DARK, width=1)
    return finish(img, size)


def draw_shield(size: int, fill: Color, mark: Color = RED) -> Image.Image:
    img, draw = icon_canvas(size); badge(draw, size)
    polygon(draw, size, [(18, 12), (46, 12), (50, 33), (32, 56), (14, 33)], fill, outline=SILVER, width=2)
    polygon(draw, size, [(32, 20), (39, 35), (32, 31), (25, 35)], mark, outline=mark, width=1)
    return finish(img, size)


def draw_tunic(size: int, fill: Color) -> Image.Image:
    img, draw = icon_canvas(size); badge(draw, size)
    polygon(draw, size, [(20, 15), (28, 12), (32, 20), (36, 12), (44, 15), (52, 29), (43, 35), (41, 53), (23, 53), (21, 35), (12, 29)], fill)
    return finish(img, size)


def draw_boots(size: int, fill: Color, wing: bool = False) -> Image.Image:
    img, draw = icon_canvas(size); badge(draw, size)
    polygon(draw, size, [(18, 17), (29, 17), (30, 42), (42, 43), (45, 52), (18, 52)], fill, outline=DARK)
    polygon(draw, size, [(36, 21), (47, 18), (47, 43), (54, 44), (56, 52), (36, 52)], fill, outline=DARK)
    if wing:
        line(draw, size, [(43, 22), (56, 12), (49, 27), (58, 25)], WHITE, 2)
    return finish(img, size)


def draw_marker(size: int) -> Image.Image:
    img, draw = icon_canvas(size)
    glow(draw, (12, 8, 52, 56), RED, size)
    polygon(draw, size, [(32, 56), (20, 30), (44, 30)], RED, outline=WHITE)
    ellipse(draw, size, (20, 10, 44, 34), RED, outline=WHITE, width=2)
    ellipse(draw, size, (28, 18, 36, 26), WHITE, outline=WHITE, width=1)
    return finish(img, size)


def draw_player(size: int, tunic: Color = GREEN, cap: Color | None = None) -> Image.Image:
    if cap is None:
        cap = tunic
    img, draw = icon_canvas(size); badge(draw, size, rgba("#112112", 130))
    # Simple original Link-like preview silhouette. The tunic/cap color is the
    # important state cue, and equipment badges are composed by the demo UI.
    ellipse(draw, size, (25, 10, 39, 24), rgba("#f0c58a"), outline=DARK, width=1)
    polygon(draw, size, [(18, 22), (32, 13), (46, 22), (40, 54), (24, 54)], tunic, outline=DARK)
    polygon(draw, size, [(25, 11), (34, 4), (40, 15)], cap, outline=DARK)
    line(draw, size, [(23, 52), (18, 60)], rgba("#5f3a24"), 4)
    line(draw, size, [(41, 52), (46, 60)], rgba("#5f3a24"), 4)
    return finish(img, size)


def draw_medallion(size: int, color: Color, symbol: str) -> Image.Image:
    img, draw = icon_canvas(size); badge(draw, size)
    glow(draw, (8, 8, 56, 56), color, size)
    ellipse(draw, size, (13, 13, 51, 51), color, outline=GOLD, width=2.4)
    ellipse(draw, size, (23, 23, 41, 41), rgba("#ffffff", 55), outline=WHITE, width=1.2)
    draw.text((size*4*0.5, size*4*0.49), symbol, fill=WHITE, anchor="mm")
    return finish(img, size)


def draw_stone(size: int, color: Color, shape: str) -> Image.Image:
    img, draw = icon_canvas(size); badge(draw, size)
    glow(draw, (8, 8, 56, 56), color, size)
    if shape == "triangle":
        polygon(draw, size, [(32, 11), (51, 48), (13, 48)], color, outline=WHITE, width=2)
    elif shape == "ruby":
        polygon(draw, size, [(32, 9), (49, 24), (44, 50), (20, 50), (15, 24)], color, outline=WHITE, width=2)
    else:
        polygon(draw, size, [(32, 10), (51, 31), (32, 54), (13, 31)], color, outline=WHITE, width=2)
    return finish(img, size)


def draw_skull(size: int) -> Image.Image:
    img, draw = icon_canvas(size); badge(draw, size)
    ellipse(draw, size, (17, 12, 47, 43), WHITE, outline=DARK, width=2)
    rect(draw, size, (24, 39, 40, 53), WHITE, outline=DARK, width=1, radius=2)
    ellipse(draw, size, (23, 25, 29, 32), BLACK, outline=BLACK, width=1)
    ellipse(draw, size, (35, 25, 41, 32), BLACK, outline=BLACK, width=1)
    line(draw, size, [(27, 47), (37, 47)], DARK, 1.4)
    return finish(img, size)


def draw_card(size: int) -> Image.Image:
    img, draw = icon_canvas(size); badge(draw, size)
    rect(draw, size, (13, 18, 51, 46), rgba("#a67bdc"), outline=GOLD, width=2, radius=4)
    polygon(draw, size, [(23, 33), (32, 23), (41, 33), (36, 39), (28, 39)], rgba("#5a276f"), outline=WHITE, width=1)
    return finish(img, size)


def draw_heart(size: int) -> Image.Image:
    img, draw = icon_canvas(size); badge(draw, size)
    polygon(draw, size, [(32, 53), (15, 34), (15, 22), (24, 15), (32, 23), (40, 15), (49, 22), (49, 34)], RED, outline=WHITE, width=2)
    return finish(img, size)


def draw_song(size: int, color: Color, mark: str) -> Image.Image:
    img, draw = icon_canvas(size); badge(draw, size)
    glow(draw, (9, 9, 55, 55), color, size)
    ellipse(draw, size, (15, 15, 49, 49), rgba("#1e2548"), outline=color, width=3)
    line(draw, size, [(26, 42), (26, 21), (42, 17), (42, 36)], color, 3)
    ellipse(draw, size, (18, 38, 28, 48), color, outline=WHITE, width=.8)
    ellipse(draw, size, (34, 32, 44, 42), color, outline=WHITE, width=.8)
    draw.text((size*4*0.5, size*4*0.52), mark, fill=WHITE, anchor="mm")
    return finish(img, size)


def draw_song_button(size: int, label: str, color: Color) -> Image.Image:
    img, draw = icon_canvas(size)
    ellipse(draw, size, (10, 10, 54, 54), color, outline=WHITE, width=3)
    draw.text((size*4*0.5, size*4*0.47), label, fill=WHITE, anchor="mm")
    return finish(img, size)



def draw_hud_button(size: int, label: str, fill: Color, *, wide: bool = False) -> Image.Image:
    img, draw = icon_canvas(size)
    # Solid OoT-inspired HUD buttons. These are icon glyphs, not extra runtime
    # text labels; the demo can draw them without adding separate text nodes.
    if wide:
        box = (5, 15, 59, 49)
        inner = (9, 19, 55, 45)
        shadow = (8, 34, 58, 52)
        radius = 10
    else:
        box = (6, 6, 58, 58)
        inner = (11, 11, 53, 53)
        shadow = (10, 38, 58, 60)
        radius = 24
    draw.rounded_rectangle(xy(shadow, size), radius=stroke_width(size, radius), fill=(0, 0, 0, 115))
    draw.rounded_rectangle(xy(box, size), radius=stroke_width(size, radius), fill=fill, outline=rgba("#fff8ce"), width=stroke_width(size, 2.0))
    r, g, b, a = fill
    highlight = (min(255, r + 42), min(255, g + 42), min(255, b + 42), a)
    draw.rounded_rectangle(xy(inner, size), radius=stroke_width(size, radius * 0.72), fill=highlight)
    draw.rounded_rectangle(xy((inner[0] + 1.5, inner[1] + 2.0, inner[2] - 1.5, inner[3] - 2.0), size), radius=stroke_width(size, radius * 0.62), fill=fill)
    if label == "START":
        # Menu/start glyph: three short bars rather than a literal word.
        for yy, ww in [(25, 30), (32, 38), (39, 24)]:
            rect(draw, size, (32 - ww / 2, yy - 1.4, 32 + ww / 2, yy + 1.4), WHITE, outline=BLACK, width=0.3, radius=1.4)
    elif label == "A":
        # Confirm glyph.
        line(draw, size, [(20, 34), (29, 43), (46, 22)], WHITE, 5)
        line(draw, size, [(20, 34), (29, 43), (46, 22)], rgba("#1b2440"), 1.4)
    elif label == "B":
        # Back/cancel glyph.
        line(draw, size, [(43, 22), (25, 22), (19, 31), (25, 40), (43, 40)], WHITE, 5)
        polygon(draw, size, [(19, 31), (29, 21), (29, 41)], WHITE, outline=BLACK, width=0.7)
    return finish(img, size)


def draw_c_hud_button(size: int, direction: str) -> Image.Image:
    img, draw = icon_canvas(size)
    draw.ellipse(xy((7, 7, 57, 57), size), fill=YELLOW, outline=rgba("#fff8ce"), width=stroke_width(size, 2.0))
    draw.ellipse(xy((12, 12, 52, 52), size), fill=rgba("#f2c327"), outline=rgba("#885e09"), width=stroke_width(size, 1.0))
    if direction == "left":
        points = [(21, 32), (36, 18), (36, 27), (47, 27), (47, 37), (36, 37), (36, 46)]
    elif direction == "right":
        points = [(43, 32), (28, 18), (28, 27), (17, 27), (17, 37), (28, 37), (28, 46)]
    else:
        points = [(32, 45), (18, 30), (27, 30), (27, 19), (37, 19), (37, 30), (46, 30)]
    polygon(draw, size, points, WHITE, outline=BLACK, width=1.0)
    return finish(img, size)

def build_icons(size: int) -> Dict[str, Image.Image]:
    icons: Dict[str, Image.Image] = {
        "edge_left.png": draw_arrow_icon(size, "left"),
        "edge_right.png": draw_arrow_icon(size, "right"),
        "deku_stick.png": draw_stick(size),
        "deku_nut.png": draw_nut(size),
        "bomb.png": draw_bomb(size),
        "bow.png": draw_bow(size),
        "fire_arrow.png": draw_arrow(size, RED),
        "ice_arrow.png": draw_arrow(size, CYAN),
        "light_arrow.png": draw_arrow(size, YELLOW),
        "dins_fire.png": draw_spell(size, RED, "D"),
        "farores_wind.png": draw_spell(size, GREEN, "F"),
        "nayrus_love.png": draw_spell(size, BLUE, "N"),
        "slingshot.png": draw_slingshot(size),
        "ocarina.png": draw_ocarina(size),
        "bombchu.png": draw_bombchu(size),
        "longshot.png": draw_hookshot(size),
        "boomerang.png": draw_boomerang(size),
        "lens.png": draw_lens(size),
        "beans.png": draw_beans(size),
        "hammer.png": draw_hammer(size),
        "bottle.png": draw_bottle(size, CYAN),
        "milk.png": draw_bottle(size, WHITE),
        "poe.png": draw_poe(size),
        "claim_check.png": draw_scroll(size),
        "mask.png": draw_mask(size),
        "kokiri_sword.png": draw_sword(size, SILVER, GREEN),
        "master_sword.png": draw_sword(size, rgba("#c8e8ff"), BLUE),
        "biggoron_sword.png": draw_sword(size, rgba("#e7e0cb"), RED),
        "deku_shield.png": draw_shield(size, rgba("#8c5727"), GREEN),
        "hylian_shield.png": draw_shield(size, rgba("#245cb8"), RED),
        "mirror_shield.png": draw_shield(size, rgba("#b93442"), CYAN),
        "kokiri_tunic.png": draw_tunic(size, GREEN),
        "goron_tunic.png": draw_tunic(size, RED),
        "zora_tunic.png": draw_tunic(size, BLUE),
        "kokiri_boots.png": draw_boots(size, rgba("#8b582e")),
        "iron_boots.png": draw_boots(size, GRAY),
        "hover_boots.png": draw_boots(size, rgba("#c9a35d"), wing=True),
        "map_marker.png": draw_marker(size),
        "player.png": draw_player(size),
        "player_kokiri_tunic.png": draw_player(size, GREEN),
        "player_goron_tunic.png": draw_player(size, RED),
        "player_zora_tunic.png": draw_player(size, BLUE),
        "med_forest.png": draw_medallion(size, GREEN, "F"),
        "med_fire.png": draw_medallion(size, RED, "F"),
        "med_water.png": draw_medallion(size, BLUE, "W"),
        "med_spirit.png": draw_medallion(size, ORANGE, "S"),
        "med_shadow.png": draw_medallion(size, PURPLE, "S"),
        "med_light.png": draw_medallion(size, YELLOW, "L"),
        "stone_emerald.png": draw_stone(size, GREEN, "triangle"),
        "stone_ruby.png": draw_stone(size, RED, "ruby"),
        "stone_sapphire.png": draw_stone(size, BLUE, "diamond"),
        "stone_agony.png": draw_stone(size, PURPLE, "diamond"),
        "skull_token.png": draw_skull(size),
        "gerudo_card.png": draw_card(size),
        "heart_piece.png": draw_heart(size),
        "song_button_a.png": draw_song_button(size, "A", BLUE),
        "song_button_c.png": draw_song_button(size, "C", YELLOW),
        "hud_start.png": draw_hud_button(size, "START", RED, wide=True),
        "hud_button_a.png": draw_hud_button(size, "A", BLUE),
        "hud_button_b.png": draw_hud_button(size, "B", GREEN),
        "hud_button_c_left.png": draw_c_hud_button(size, "left"),
        "hud_button_c_down.png": draw_c_hud_button(size, "down"),
        "hud_button_c_right.png": draw_c_hud_button(size, "right"),
    }
    song_specs = {
        "song_minuet.png": (GREEN, "M"),
        "song_bolero.png": (RED, "B"),
        "song_serenade.png": (BLUE, "S"),
        "song_requiem.png": (ORANGE, "R"),
        "song_nocturne.png": (PURPLE, "N"),
        "song_prelude.png": (YELLOW, "P"),
        "song_lullaby.png": (rgba("#f5b5ff"), "Z"),
        "song_epona.png": (rgba("#d68b4e"), "E"),
        "song_saria.png": (GREEN, "S"),
        "song_sun.png": (YELLOW, "☀"),
        "song_time.png": (CYAN, "T"),
        "song_storms.png": (rgba("#798cff"), "⚡"),
    }
    for name, (color, mark) in song_specs.items():
        icons[name] = draw_song(size, color, mark)
    return icons


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, default=Path("assets/icons/oot"), help="output directory")
    parser.add_argument("--size", type=int, default=96, help="icon size in pixels")
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    icons = build_icons(args.size)
    for filename, image in sorted(icons.items()):
        image.save(args.out / filename)
    print(f"Generated {len(icons)} icons in {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
