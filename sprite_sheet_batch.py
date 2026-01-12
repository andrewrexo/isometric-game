#!/usr/bin/env python3
"""Convert images to PNG and create sprite sheets from batches."""

from __future__ import annotations

import argparse
import math
from pathlib import Path

from PIL import Image


def iter_images(input_dir: Path, extensions: set[str]) -> list[Path]:
    files = [p for p in input_dir.iterdir() if p.is_file() and p.suffix.lower() in extensions]
    return sorted(files, key=lambda p: p.name)


def key_out_black(image: Image.Image) -> Image.Image:
    rgba = image.convert("RGBA")
    pixels = list(rgba.getdata())
    new_pixels = [
        (0, 0, 0, 0) if (r, g, b) == (0, 0, 0) else (r, g, b, a)
        for (r, g, b, a) in pixels
    ]
    rgba.putdata(new_pixels)
    return rgba


def convert_to_png(
    images: list[Path],
    output_dir: Path,
    key_black: bool,
) -> list[Path]:
    output_dir.mkdir(parents=True, exist_ok=True)
    converted: list[Path] = []
    for image_path in images:
        output_path = output_dir / f"{image_path.stem}.png"
        with Image.open(image_path) as img:
            if key_black:
                converted_img = key_out_black(img)
            else:
                converted_img = img
            converted_img.save(output_path, format="PNG")
        converted.append(output_path)
    return converted


def build_sheet(
    images: list[Path],
    output_path: Path,
    columns: int,
    rows: int,
    fill_color: tuple[int, int, int, int],
) -> None:
    with Image.open(images[0]) as first:
        tile_w, tile_h = first.size

    sheet_w = columns * tile_w
    sheet_h = rows * tile_h
    sheet = Image.new("RGBA", (sheet_w, sheet_h), fill_color)

    for idx, image_path in enumerate(images):
        row = idx % rows
        col = idx // rows
        x = col * tile_w
        y = row * tile_h
        with Image.open(image_path) as img:
            sheet.paste(img, (x, y))

    output_path.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(output_path)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Batch images into sprite sheets, column-major (top-down) layout.",
    )
    parser.add_argument(
        "input_dir",
        nargs="?",
        default="/Users/samson/projects/extract-egf-images/data/output/gfx009",
        help="Directory containing input images",
    )
    parser.add_argument(
        "--output-dir",
        default="/Users/samson/projects/extract-egf-images/data/output/gfx009/atlas",
        help="Directory to write sprite sheets",
    )
    parser.add_argument(
        "--batch-size",
        type=int,
        default=16,
        help="Number of images per spritesheet",
    )
    parser.add_argument(
        "--columns",
        type=int,
        default=16,
        help="Number of columns in the sheet",
    )
    parser.add_argument(
        "--rows",
        type=int,
        default=1,
        help="Number of rows in the sheet",
    )
    parser.add_argument(
        "--ext",
        default="bmp",
        help="Comma-separated extensions to include (default: bmp)",
    )
    parser.add_argument(
        "--convert-dir",
        default="",
        help="Directory to write converted PNGs (default: <input_dir>/converted_png)",
    )
    parser.add_argument(
        "--key-out-black",
        action="store_true",
        help="Replace pure black (#000000) with transparency during conversion",
    )
    args = parser.parse_args()

    input_dir = Path(args.input_dir)
    output_dir = Path(args.output_dir)
    extensions = {f".{ext.strip().lower()}" for ext in args.ext.split(",") if ext.strip()}
    convert_dir = Path(args.convert_dir) if args.convert_dir else input_dir / "converted_png"

    if not input_dir.is_dir():
        raise SystemExit(f"Not a directory: {input_dir}")

    images = iter_images(input_dir, extensions)
    if not images:
        raise SystemExit(f"No images found in {input_dir} for extensions: {sorted(extensions)}")

    converted_images = convert_to_png(images, convert_dir, args.key_out_black)

    if args.columns * args.rows < args.batch_size:
        raise SystemExit("columns * rows must be >= batch-size")

    sheet_count = math.ceil(len(converted_images) / args.batch_size)
    for sheet_idx in range(sheet_count):
        start = sheet_idx * args.batch_size
        end = start + args.batch_size
        batch = converted_images[start:end]
        if not batch:
            continue
        output_path = output_dir / f"spritesheet_{sheet_idx:03d}.png"
        build_sheet(
            batch,
            output_path,
            columns=args.columns,
            rows=args.rows,
            fill_color=(0, 0, 0, 0),
        )
        print(f"wrote {output_path} with {len(batch)} image(s)")


if __name__ == "__main__":
    main()
