#!/usr/bin/env python3
"""
Sprite atlas packer for WASM fast loading.

Combines individual sprite PNGs into atlas textures with JSON metadata.
Categories packed: objects, walls, inventory.
Equipment/weapons/NPCs are kept as individual files (animation spritesheets).

Usage: python3 tools/pack_atlases.py
"""

import json
import os
from pathlib import Path

from PIL import Image

ASSETS_DIR = Path(__file__).parent.parent / "client" / "assets"
SPRITES_DIR = ASSETS_DIR / "sprites"
MANIFEST_PATH = ASSETS_DIR / "sprite_manifest.json"
MAX_ATLAS_SIZE = 4096


def collect_sprites(directory: Path):
    """Collect all PNGs in a directory, returning {key: path} sorted by height desc."""
    sprites = {}
    for png in sorted(directory.glob("*.png")):
        key = png.stem
        sprites[key] = png
    return sprites


def pack_atlas(sprites: dict[str, Path]):
    """
    Row-pack sprites into atlas(es) up to MAX_ATLAS_SIZE x MAX_ATLAS_SIZE.
    Returns list of (Image, {key: {x, y, w, h}}) tuples.
    """
    # Load all images and sort by height descending for better packing
    images = {}
    for key, path in sprites.items():
        img = Image.open(path).convert("RGBA")
        images[key] = img

    sorted_keys = sorted(images.keys(), key=lambda k: (-images[k].height, -images[k].width))

    atlases = []
    remaining = list(sorted_keys)

    while remaining:
        rects = {}
        cur_x = 0
        cur_y = 0
        row_height = 0
        atlas_width = 0
        atlas_height = 0
        placed = []

        for key in remaining:
            img = images[key]
            w, h = img.size

            if w > MAX_ATLAS_SIZE or h > MAX_ATLAS_SIZE:
                print(f"  WARNING: sprite {key} is {w}x{h}, exceeds max atlas size, skipping")
                placed.append(key)
                continue

            # Try to place in current row
            if cur_x + w <= MAX_ATLAS_SIZE:
                if cur_y + h > MAX_ATLAS_SIZE:
                    # Doesn't fit vertically, skip to next atlas
                    continue
                rects[key] = {"x": cur_x, "y": cur_y, "w": w, "h": h}
                row_height = max(row_height, h)
                cur_x += w
                atlas_width = max(atlas_width, cur_x)
                atlas_height = max(atlas_height, cur_y + row_height)
                placed.append(key)
            else:
                # Start new row
                cur_x = 0
                cur_y += row_height
                row_height = 0

                if cur_y + h > MAX_ATLAS_SIZE:
                    # Atlas full
                    continue

                rects[key] = {"x": cur_x, "y": cur_y, "w": w, "h": h}
                row_height = max(row_height, h)
                cur_x += w
                atlas_width = max(atlas_width, cur_x)
                atlas_height = max(atlas_height, cur_y + row_height)
                placed.append(key)

        if not placed:
            print(f"  ERROR: Could not place any sprites, {len(remaining)} remaining")
            break

        # Create the atlas image
        atlas_img = Image.new("RGBA", (atlas_width, atlas_height), (0, 0, 0, 0))
        for key, rect in rects.items():
            atlas_img.paste(images[key], (rect["x"], rect["y"]))

        atlases.append((atlas_img, rects))
        remaining = [k for k in remaining if k not in placed]

    return atlases


def pack_category(category: str):
    """Pack a single category and return atlas info for manifest."""
    sprite_dir = SPRITES_DIR / category
    if not sprite_dir.exists():
        print(f"  Skipping {category}: directory not found")
        return None

    sprites = collect_sprites(sprite_dir)
    if not sprites:
        print(f"  Skipping {category}: no sprites found")
        return None

    print(f"  Packing {len(sprites)} {category} sprites...")
    atlases = pack_atlas(sprites)

    if len(atlases) == 1:
        atlas_img, rects = atlases[0]
        atlas_filename = f"{category}_atlas.png"
        atlas_path = SPRITES_DIR / atlas_filename
        atlas_img.save(atlas_path, optimize=True)
        print(f"  -> {atlas_filename}: {atlas_img.width}x{atlas_img.height}, {len(rects)} sprites")
        return {
            "file": f"sprites/{atlas_filename}",
            "sprites": rects,
        }
    else:
        # Multiple atlases needed
        all_sprites = {}
        files = []
        for i, (atlas_img, rects) in enumerate(atlases):
            atlas_filename = f"{category}_atlas_{i}.png"
            atlas_path = SPRITES_DIR / atlas_filename
            atlas_img.save(atlas_path, optimize=True)
            print(f"  -> {atlas_filename}: {atlas_img.width}x{atlas_img.height}, {len(rects)} sprites")
            # Tag each sprite with its atlas index
            for key, rect in rects.items():
                rect["atlas"] = i
                all_sprites[key] = rect
            files.append(f"sprites/{atlas_filename}")
        return {
            "files": files,
            "sprites": all_sprites,
        }


def main():
    print("Sprite Atlas Packer")
    print("=" * 40)

    # Load existing manifest
    with open(MANIFEST_PATH) as f:
        manifest = json.load(f)

    categories_to_pack = ["objects", "walls", "inventory"]

    for category in categories_to_pack:
        atlas_info = pack_category(category)
        if atlas_info:
            manifest[f"{category}_atlas"] = atlas_info

    # Write updated manifest
    with open(MANIFEST_PATH, "w") as f:
        json.dump(manifest, f, indent=2)
        f.write("\n")

    print()
    print("Done! Updated sprite_manifest.json with atlas info.")


if __name__ == "__main__":
    main()
