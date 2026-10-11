# Main push and Windows installers

The user requested pushing to main and producing Tauri MSI/EXE installers, then
confirmed inclusion of all completed application, test and documentation changes.

1. Wait for the settings/shell owner to finish source, contract, warning and native
   checks and explicitly release Cargo/dist. Preserve its unfinished edits.
2. Freeze the completed source, confirm main is not behind origin/main and review
   packaging/publication risks. Stage only application, tests and documentation;
   exclude local audit data, screenshots, databases and secrets.
3. Run proportional final checks on the frozen combined tree, retaining exact
   failures and ignored-test distinctions. Verify typed registry and actual
   notice HTML emission. Do not bypass failed checks.
4. Build the normal production configuration with Tauri `--bundles msi,nsis`.
   Keep synchronized version 0.1.0 and the existing updater public/private key;
   do not create a tag, rotate a key or publish a GitHub Release. Check installer
   signatures, filenames, hashes and build/source provenance. Report missing
   Windows publisher signing separately from Tauri updater signing.
5. Commit the approved completed changes with Conventional Commits and push main
   without force. Verify the remote commit matches and deliver local installer
   links. Do not run installers or replace installed EMMM during packaging.

Installer execution, direct game rendering, click-through hardware, multi-monitor
DPI visuals and exclusive-fullscreen visibility are not part of this build proof.
