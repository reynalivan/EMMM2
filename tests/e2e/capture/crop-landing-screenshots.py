"""Publish native landing screenshots and focused feature crops."""

from pathlib import Path

from PIL import Image


ROOT = Path(__file__).resolve().parents[3] / "landing"
RAW = ROOT / ".screenshots"
PUBLIC = ROOT / "public" / "screenshots"

CROPS = {
    "hero-mod-grid": ("workspace-library", (330, 155, 1580, 600)),
    "hero-collections": ("collections-safe-mode", (220, 185, 1775, 850)),
    "switch-toggle-detail": ("switch-controls", (1040, 330, 1570, 570)),
    "switch-bulk-detail": ("switch-bulk-actions", (625, 1130, 1130, 1240)),
    "quick-play-focus": ("dashboard-play", (220, 185, 405, 355)),
    "game-settings-focus": ("setup-games", (670, 180, 1330, 850)),
    "setup-xxmi-actions": ("setup-xxmi-welcome", (540, 790, 1460, 1120)),
    "home-key-mapping-focus": ("home-key-mapping", (245, 825, 1725, 1190)),
    "hotkey-settings-focus": ("settings-hotkeys", (650, 100, 1585, 815)),
    "settings-catalog-focus": ("settings-catalog", (660, 100, 1590, 585)),
    "metadata-filters-focus": ("search-filters", (0, 80, 330, 550)),
}
FULL_CAPTURES = (
    "collection-switch-preview",
    "collections-safe-mode",
    "dashboard-play",
    "favorites-filters",
    "home-key-mapping",
    "mod-health-details",
    "mod-inbox",
    "nested-subfolders",
    "paste-thumbnail",
    "pin-mod-menu",
    "search-filters",
    "settings-hotkeys",
    "settings-catalog",
    "setup-games",
    "setup-xxmi-welcome",
    "smart-classification",
    "storage-optimizer",
    "switch-bulk-actions",
    "switch-controls",
    "workspace-library",
)


def main() -> None:
    PUBLIC.mkdir(parents=True, exist_ok=True)
    for name in FULL_CAPTURES:
        with Image.open(RAW / f"{name}.png") as image:
            image.save(PUBLIC / f"{name}.webp", format="WEBP", quality=84, method=6)
            if name == "workspace-library":
                image.save(PUBLIC / "aster-costume-grid.webp", format="WEBP", quality=84, method=6)
    for name, (source, box) in CROPS.items():
        source_path = RAW / f"{source}.png"
        with Image.open(source_path) as image:
            if box[2] > image.width or box[3] > image.height:
                raise ValueError(f"Crop {name} exceeds {source_path} ({image.size})")
            image.crop(box).save(PUBLIC / f"{name}.webp", format="WEBP", quality=84, method=6)


if __name__ == "__main__":
    main()
