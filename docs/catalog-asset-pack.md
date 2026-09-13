# Catalog asset pack format

EMMM does not bundle or download catalog packs. Extract one user-provided pack into the folder opened from **Settings > General > Catalog assets**.

```text
asset-pack/
  manifest.json
  catalog/gimi.json
  images/gimi/example.webp
```

`manifest.json` uses this format:

```json
{
  "format_version": 1,
  "id": "community-catalog",
  "version": "1.0.0",
  "author": "Pack author",
  "source": "publisher-declared source",
  "license": "publisher-declared license",
  "catalogs": {
    "gimi": {
      "path": "catalog/gimi.json",
      "sha256": "lowercase SHA-256 of catalog/gimi.json"
    }
  }
}
```

Each catalog file follows EMMM's `{"entries": [...]}` MasterDB format. Thumbnail paths must be relative to the pack root, use PNG, JPEG, or WebP, and stay within the pack directory. Packs cannot contain executable code, scripts, or remote URLs.
