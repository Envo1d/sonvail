"""Export DPI variants and a multi-size Windows ICO from the approved source."""

from pathlib import Path

from PIL import Image


icon_dir = Path(__file__).resolve().parents[1] / "assets" / "icons"
with Image.open(icon_dir / "sonvail-source.png") as source:
    image = source.convert("RGBA")
    for size in (32, 64, 128, 256):
        # Filter premultiplied color and alpha together to avoid dark transparent edges.
        resized = image.convert("RGBa").resize((size, size), Image.Resampling.LANCZOS)
        resized.convert("RGBA").save(icon_dir / f"sonvail-{size}.png")
    image.save(
        icon_dir / "sonvail.ico",
        sizes=[(size, size) for size in (16, 20, 24, 32, 40, 48, 64, 96, 128, 256)],
    )
