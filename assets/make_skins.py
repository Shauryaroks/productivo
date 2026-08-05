#!/usr/bin/env python3
"""Generates the pet skins as original pixel art (CC0, drawn in-code here).
Requires imagemagick. Run from assets/: python3 make_skins.py
Frames are written as PPM with magenta = transparent, assembled via magick.
"""
import os, subprocess, tempfile

PAL = {
    ".": None,             # transparent
    "K": (28, 28, 30),     # outline
    "G": (158, 158, 166),  # gray fur
    "W": (242, 242, 247),  # white
    "E": (20, 20, 22),     # eye
    "N": (255, 122, 148),  # nose
    "P": (255, 158, 181),  # cheek
    "O": (244, 155, 51),   # orange fur
    "C": (255, 232, 199),  # cream
    # rainbow bands
    "1": (255, 59, 48), "2": (255, 149, 0), "3": (255, 204, 0),
    "4": (52, 199, 89), "5": (10, 132, 255), "6": (175, 82, 222),
}
MAGENTA = (255, 0, 255)

CAT = """\
..............K.....K.
..............KK...KK.
....KKKKKKKK..KKKKKKK.
...KGGGGGGGGKKGGGGGGGK
..KGGGGGGGGGGKGWEGWEGK
..KGGGGGGGGGGKGGGNGGGK
..KGGGGGGGGGGKPGWWWGPK
...KGGGGGGGGKKGGGGGGGK
....KKKKKKKK..KKKKKKK.
.....KGK..KGK..KGK.KGK
.....KKK..KKK..KKK.KKK""".splitlines()

TOAST_BASE = """\
...K.....K........
...KK...KK........
...KOKKKOK........
..KOOOOOOOK.......
..KOWEOWEOK.......
..KOOONOOOK.......
..KPOCCCOPK.......
..KOOOOOOOK.......
.KOOOOOOOOOK......
KOOOOOOOOOOOK..KK.
KOOOOOOOOOOOK.KOK.
KOOOOOOOOOOOKKKOK.
.KOKKOKKKOKKOKKK..
..KK..KK..KK......""".splitlines()


def blank(w, h):
    return [[None] * w for _ in range(h)]


def blit(canvas, art, ox, oy):
    for y, row in enumerate(art):
        for x, ch in enumerate(row):
            c = PAL[ch]
            if c is not None:
                canvas[oy + y][ox + x] = c


def write_ppm(path, canvas):
    h, w = len(canvas), len(canvas[0])
    with open(path, "w") as f:
        f.write(f"P3\n{w} {h}\n255\n")
        for row in canvas:
            f.write(" ".join(
                " ".join(map(str, px if px else MAGENTA)) for px in row) + "\n")


def comet_frames():
    """Gray cat flying right with a waving 6-band rainbow trail."""
    W, H, frames = 38, 15, []
    for fr in range(6):
        cv = blank(W, H)
        bob = 1 if fr % 4 >= 2 else 0
        for x in range(0, 19):  # trail runs under the cat, which draws over it
            yo = 1 if (x // 4 + fr // 2) % 2 else 0
            for band, ch in enumerate("123456"):
                cv[1 + band * 2 + yo + bob][x] = PAL[ch]
                cv[2 + band * 2 + yo + bob][x] = PAL[ch]
        cat = [r for r in CAT]
        if fr % 2:  # legs tuck: drop the last (paw) row on odd frames
            cat = cat[:-1]
        blit(cv, cat, 15, 2 + bob)
        frames.append(cv)
    return frames


def toast_frames():
    """Orange loaf cat: tail wag + a blink."""
    frames = []
    for fr in range(4):
        art = [r for r in TOAST_BASE]
        if fr == 3:  # blink
            art[4] = art[4].replace("E", "O")
        if fr % 2:  # tail wag: shift tail pixels one column left
            art = [r.replace("KK.", ".KK").replace("KOK.", ".KOK")
                   if x >= 9 else r for x, r in enumerate(art)]
        cv = blank(20, 15)
        blit(cv, art, 1, 1 if fr % 2 else 0)  # gentle bob with the wag
        frames.append(cv)
    return frames


def build(name, frames):
    with tempfile.TemporaryDirectory() as td:
        paths = []
        for i, cv in enumerate(frames):
            p = os.path.join(td, f"{i:02}.ppm")
            write_ppm(p, cv)
            paths.append(p)
        subprocess.run(
            ["magick", "-delay", "10", "-loop", "0", *paths,
             "-transparent", "magenta", f"{name}.gif"], check=True)
    print(f"{name}.gif: {len(frames)} frames")


if __name__ == "__main__":
    os.chdir(os.path.dirname(os.path.abspath(__file__)))
    build("comet", comet_frames())
    build("toast", toast_frames())
