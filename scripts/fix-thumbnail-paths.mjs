/**
 * Fix thumbnail_path values in all MasterDB JSON files.
 *
 * Changes:
 * 1. Character entries: "app/assets/thumbnails/{game}/char/{file}.png"
 *    → "databases/thumbnails/{game}/char/{file}.png" (relative to resource_dir)
 * 2. Skin entries: "app/assets/thumbnails/{game}/skin/{file}.png"
 *    → null (no skin thumbnail files exist)
 * 3. Non-character categories with fake paths (weapon.png, ui.png, etc.)
 *    → null (files don't exist)
 *
 * Also verifies each path points to a real file in resources/databases/thumbnails/.
 *
 * Usage: node scripts/fix-thumbnail-paths.mjs
 */
import { readFileSync, writeFileSync, existsSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const DB_DIR = join(__dirname, '..', 'src-tauri', 'resources', 'databases');
const RESOURCE_DIR = join(__dirname, '..', 'src-tauri', 'resources');

const GAMES = ['gimi', 'srmi', 'zzmi', 'wwmi', 'efmi'];

// Old prefix → new prefix mapping
const OLD_PREFIX = 'app/assets/thumbnails/';
const NEW_PREFIX = 'databases/thumbnails/';

let totalFixed = 0;
let totalNulled = 0;
let totalSkinNulled = 0;

for (const game of GAMES) {
  const filePath = join(DB_DIR, `${game}.json`);
  if (!existsSync(filePath)) {
    console.log(`⚠ ${game}.json not found, skipping`);
    continue;
  }

  const entries = JSON.parse(readFileSync(filePath, 'utf8'));
  let fixed = 0;
  let nulled = 0;
  let skinNulled = 0;

  for (const entry of entries) {
    // Fix base thumbnail_path
    if (entry.thumbnail_path && typeof entry.thumbnail_path === 'string') {
      if (entry.thumbnail_path.startsWith(OLD_PREFIX)) {
        // Convert old prefix to new prefix
        const relativePath = entry.thumbnail_path.replace(OLD_PREFIX, NEW_PREFIX);
        const absolutePath = join(RESOURCE_DIR, relativePath);

        if (existsSync(absolutePath)) {
          entry.thumbnail_path = relativePath;
          fixed++;
        } else {
          // File doesn't exist (non-char categories like weapon.png, ui.png)
          entry.thumbnail_path = null;
          nulled++;
        }
      }
    }

    // Fix custom_skins thumbnail_skin_path
    if (Array.isArray(entry.custom_skins)) {
      for (const skin of entry.custom_skins) {
        if (skin.thumbnail_skin_path && typeof skin.thumbnail_skin_path === 'string') {
          if (skin.thumbnail_skin_path.startsWith(OLD_PREFIX)) {
            const relativePath = skin.thumbnail_skin_path.replace(OLD_PREFIX, NEW_PREFIX);
            const absolutePath = join(RESOURCE_DIR, relativePath);

            if (existsSync(absolutePath)) {
              skin.thumbnail_skin_path = relativePath;
              fixed++;
            } else {
              // Skin thumbnail files don't exist
              skin.thumbnail_skin_path = null;
              skinNulled++;
            }
          }
        }
      }
    }
  }

  // Write back
  writeFileSync(filePath, JSON.stringify(entries, null, 2) + '\n', 'utf8');

  console.log(
    `${game.toUpperCase()}: ${fixed} fixed, ${nulled} nulled (no file), ${skinNulled} skin thumbnails nulled`,
  );
  totalFixed += fixed;
  totalNulled += nulled;
  totalSkinNulled += skinNulled;
}

console.log(
  `\n✅ Total: ${totalFixed} fixed, ${totalNulled} nulled, ${totalSkinNulled} skin nulled`,
);
