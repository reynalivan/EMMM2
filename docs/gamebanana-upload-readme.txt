+==========================================================================+
| READ THIS FIRST                                                          |
| THIS ZIP CONTAINS DOWNLOAD LINKS AND INFORMATION ONLY.                   |
| IT DOES NOT CONTAIN THE EMMM APPLICATION OR WINDOWS INSTALLER.           |
+==========================================================================+

EMMM — 3DMigoto Mod Manager for Windows
=======================================

EMMM is an open-source desktop mod manager for GIMI, SRMI, ZZMI, WWMI, EFMI, and other supported 3DMigoto setups. It works with the mod folders you already have, including libraries spread across multiple locations. You do not need to reorganize everything into a manager-specific folder layout.

+-- OFFICIAL LINKS --------------------------------------------------------+
🏠  HOMEPAGE / PROJECT & SCREENSHOTS: https://github.com/reynalivan/EMMM2
⬇️   WINDOWS DOWNLOAD PAGE: https://github.com/reynalivan/EMMM2/releases
☕  SUPPORT DEVELOPMENT: https://ko-fi.com/reynalivan
+--------------------------------------------------------------------------+

This GameBanana ZIP contains this README only. It has no installer, game files, or mods. Check the Releases page for a published Windows build. If no release is listed yet, the repository README has instructions for building EMMM from source.

What EMMM does
--------------

• Smart Classification: Bring folders or .zip, .7z, and .rar downloads into Mod Inbox. EMMM finds mod roots inside nested archives and suggests destinations using available INI hashes, installed catalog records, aliases, and folder names. Review uncertain matches before import.

• Flexible libraries: Browse your existing folder structure across multiple locations. Open nested folders, use breadcrumbs, and keep working with folders created outside EMMM.

• Mod controls: Enable or disable individual mods, use “Enable Only This” to compare alternatives in a group, or apply bulk actions. Pin and star favorites for quick access.

• Search and filters: Find mods by name, category, and available metadata. Switch between grid and list views.

• Images and metadata: EMMM detects images in mod folders. Choose a preview, paste a thumbnail from the clipboard, and edit metadata in the app.

• Keybinds and in-game display: Inspect INI files and search detected keybinds from active mods. For supported runtime matches, the optional KeyViewer overlay shows bindings and status in game; F7 toggles it.

• Collections and Safe Mode: Save setups, preview changes before applying one, and switch collections while the game is in the foreground. Shift+F5 and Ctrl+F5 cycle collections; F5 toggles Safe Mode for mods you marked Safe.

• Live folder tracking: Changes to relevant folders, INI files, metadata, and images made in File Explorer are reconciled with EMMM's library view.

• Storage Optimizer: Review duplicate file groups and choose what to keep, ignore, move to trash, or hardlink.

• Optional catalog data: A separately installed catalog pack can supply known identities, hashes, metadata, images, and reviewed KeyViewer targets. Matching suggestions still need your review.

Install EMMM
------------

1. Open the Windows releases link above and choose a published EMMM release.
2. Download the Windows installer listed in that release and run it. This README-only ZIP is not the installer.
3. Open EMMM and use setup to detect a supported XXMI installation or add your game and mod-folder paths manually.
4. Point EMMM at your existing mod library. Add another location if your mods are spread across folders.
5. If you want catalog-assisted matching, open Settings > Catalog Assets, choose a separate catalog-pack.zip, review it, and install it. Do not place the catalog ZIP in your Mods folder.
6. Bring new downloads into Mod Inbox and review EMMM's classification suggestions before importing.

Your files
----------

The normal Remove Mod action sends the mod folder to the OS Recycle Bin instead of immediately deleting it permanently. Keep a separate backup of your mod library before large imports or reorganizations.

Independence
------------

EMMM and its optional catalog are independent third-party projects. They are not affiliated with, endorsed by, or sponsored by any game, game developer, publisher, or platform. EMMM does not include game content or mods.
