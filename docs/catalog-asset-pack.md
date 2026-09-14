# Catalog asset pack format

EMMM uses one active, data-only Catalog Pack for the application. A pack can
support more than one game. It supplies canonical identities, aliases, hashes,
metadata, and—only when it declares valid runtime targets—KeyViewer data.

## Install a pack

In **Settings > Catalog assets**, paste one of these public HTTPS URLs:

- `https://github.com/owner/repository`
- `https://github.com/owner/repository/releases/latest`
- `https://github.com/owner/repository/releases/tag/v1.2.3`

EMMM resolves the stable GitHub release and requires exactly one
`catalog-pack.zip` asset. It downloads the asset to a short-lived staging
directory, validates it, shows its manifest information, then activates the
same reviewed archive only after confirmation. Repository source ZIPs, raw-file
URLs, private repositories, and repository cloning are not supported.

You can also choose a local `catalog-pack.zip`. It is staged and reviewed by
the same validator, and the source ZIP remains in its original location. EMMM
does not clone repositories, execute repository files, or install a pack until
the review is confirmed.

To find community packs, EMMM opens this GitHub repository search convention:

```text
topic:emmm-catalog topic:emmm-game-{game} archived:false
```

Publishers may use `emmm-catalog` and game topics such as `emmm-game-gimi`.
Topics make packs discoverable, not verified or endorsed. A broader search uses
`topic:emmm-catalog`.

The Catalog updates section is manual: **Check now** and **Install** only run
when selected by the user. A URL declared in a locally selected manifest is
never used as an update source.

## Pack archive

A catalog ZIP contains only catalog data and bounded image assets:

```text
catalog-pack.zip
  manifest.json
  catalog/gimi.json
  assets/gimi/characters/example.webp
```

`manifest.json` has one schema and no format-version field. Runtime targets
are always supported by the current pack contract:

```json
{
  "id": "community-catalog",
  "version": "1.0.0",
  "author": "Pack author",
  "source": "publisher-declared source",
  "catalogs": {
    "gimi": {
      "path": "catalog/gimi.json",
      "sha256": "lowercase SHA-256 of catalog/gimi.json"
    }
  }
}
```

Each catalog file follows EMMM's `{"entries": [...]}` MasterDB format. Every
entry includes `runtime_targets`, using an empty array when none are available.
Archive validation rejects path traversal, symbolic links, executables,
unsupported files, oversized archives, invalid catalog checksums, and invalid
avatar checksums. Catalog avatars are bounded pack assets; user-provided
thumbnails remain local to the user's library and are never overwritten.

## Identity suggestions

EMMM checks objects locally only after an explicit request: **Check now** on
Home, Mod Inbox preflight, or the selected object action in Mods Manager. A
candidate appears on Home only when it is an unambiguous high-confidence
automatic match and has not already been confirmed, manually classified, or
ignored for that catalog identity. Installing or updating a pack does not
reconcile mod folders or change metadata or images. The Home panel never
changes metadata automatically: **Review suggestions** opens the Object
Classification Wizard, where the default selection remains empty.

Results—including no-match results—are cached by game, object revision, source
fingerprint, catalog identity/version, and matcher revision. A fast game switch
or catalog change invalidates the older generation, so results from one game do
not appear in another. Ignored candidates are stored per object, identity, and
catalog source; reset them from Catalog assets when a user wants them reviewed
again.
