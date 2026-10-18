# Tauri Updater Release Runbook

This project ships application updates through the Tauri v2 updater and GitHub Releases. The updater signature is mandatory for every update. Windows Authenticode is a separate publisher-trust control; without a trusted code-signing certificate, installers must be disclosed as unsigned and may trigger Windows/SmartScreen warnings. A self-signed certificate does not establish public publisher trust.

## Immutable key rules

- `TAURI_SIGNING_PRIVATE_KEY` is the Tauri updater private key. It must exist only in an encrypted backup and the GitHub Actions secret of this repository.
- The matching public key belongs in `src-tauri/tauri.conf.json` under `plugins.updater.pubkey`.
- Never commit either private-key file, print the private key in a log, or rotate the key after publishing an updater-enabled release. Existing installations can only accept future artifacts signed by the same private key.
- Authenticode signing is separate from Tauri updater signing. When a trusted certificate is available, configure Windows code signing during the bundle step; never alter an updater artifact after its Tauri `.sig` has been generated. Without a certificate, preserve updater signing and disclose the unsigned Windows publisher in the release notes; do not fabricate publisher trust.

## Required tracked configuration

- `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json` use the same SemVer version.
- `src-tauri/tauri.conf.json` sets `bundle.createUpdaterArtifacts` to `true`.
- `plugins.updater.pubkey` contains the updater public key, not a file path.
- `plugins.updater.endpoints` contains production HTTPS URLs only.
- `.github/workflows/release.yml` passes `TAURI_SIGNING_PRIVATE_KEY` and optional `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` to `tauri-apps/tauri-action@v1` and retains `uploadUpdaterJson: true`.

## GitHub setup

Create repository secrets at **Settings → Secrets and variables → Actions → Secrets**:

- `TAURI_SIGNING_PRIVATE_KEY`: complete content of the private `.key` file.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: the private-key password, only when one was set while generating the key.

The GitHub release updater endpoint is `https://github.com/reynalivan/EMMM2/releases/latest/download/latest.json`. A draft release is not the latest published release: inspect its assets, then publish it before expecting installed apps to find it.

## Prepare a release locally

Use the project script to update all three tracked version sources and validate updater prerequisites:

```powershell
corepack pnpm release:prepare -- --version 0.1.0
```

The script does not create a tag or push. After the release commit is complete and the working tree is clean, create an annotated local tag without pushing it:

```powershell
corepack pnpm release:prepare -- --version 0.1.0 --create-tag
```

The script refuses to create a tag from a dirty working tree. Push is deliberately a separate user action:

```powershell
git push origin v0.1.0
```

## Release verification

The dependency gate is mandatory before bundling: run
`node scripts/audit-rust-dependencies.mjs <dedicated-output-directory>` with
`cargo-audit 0.22.2`. The release workflow runs this same gate and retains its full,
unfiltered report. See `security/README.md` for exact, owned, expiring exceptions;
do not bypass an expired or newly failing review. Windows updates use raw signed
MSI/NSIS assets; the updater's optional legacy ZIP feature is disabled. Do not
publish ZIP-wrapped Windows updater assets without a new security/compatibility review.

1. Push the release tag only after its commit is reviewed.
2. Confirm the GitHub Actions workflow succeeds without exposing secrets in logs.
3. Open the draft GitHub Release and verify `latest.json`, both requested Windows installers and their generated signatures are attached. Verify updater signatures against the configured public key, compare SHA-256 checksums and confirm manifest URLs point to this release.
4. Publish the release so the `releases/latest` endpoint resolves it.
5. On a clean Windows VM, install the prior EMMM version, then update to the new release through the app.
6. Verify the installed version, the Authenticode publisher when configured, and a failed update when a copied artifact is intentionally modified in a non-production test. Record unavailable VM/upgrade checks instead of claiming they passed.

References: [Tauri updater signing](https://v2.tauri.app/plugin/updater/#signing-updates) and [Windows code signing](https://v2.tauri.app/distribute/sign/windows/).

## First release

The first updater-enabled release has no previous EMMM installation to update. Distribute it manually. Verify the updater by publishing the following release with a higher version, such as `0.1.1`, and upgrading an installation of `0.1.0`.
