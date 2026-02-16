/**
 * Migration Script: OLD-DB → New Flat Array Master DB Format
 *
 * Reads OLD-DB char + other files, transforms to new DbEntry flat array format,
 * and writes output to src-tauri/resources/databases/{game}.json
 *
 * Usage: node scripts/migrate-old-db.mjs
 */
import { readFileSync, writeFileSync, existsSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, '..');
const OLD_DB_DIR = join(ROOT, 'src-tauri', 'resources', 'OLD-DB');
const NEW_DB_DIR = join(ROOT, 'src-tauri', 'resources', 'databases');

const GAMES = [
  { id: 'gimi', charFile: 'db_gimi_char.json', otherFile: 'db_gimi_other.json' },
  { id: 'srmi', charFile: 'db_srmi_char.json', otherFile: 'db_srmi_other.json' },
  { id: 'wwmi', charFile: 'db_wwmi_char.json', otherFile: 'db_wwmi_other.json' },
  { id: 'zzmi', charFile: 'db_zzmi_char.json', otherFile: 'db_zzmi_other.json' },
];

/**
 * Transform a single OLD-DB entry to new DbEntry format.
 * @param {object} old - The old DB entry
 * @returns {object} New DbEntry format
 */
function transformEntry(old) {
  const entry = {
    name: old.name,
    tags: (old.tags || []).filter((t) => t && t.trim() !== ''),
    object_type: old.object_type || 'Other',
    custom_skins: [],
    thumbnail_path: old.thumbnail_path || '',
  };

  // Build metadata from all extra fields
  const metadata = {};
  const SKIP_KEYS = new Set(['name', 'tags', 'object_type', 'thumbnail_path', 'custom_skins']);

  for (const [key, value] of Object.entries(old)) {
    if (!SKIP_KEYS.has(key) && value !== undefined && value !== null) {
      metadata[key] = String(value);
    }
  }

  // Only include metadata if it has content
  if (Object.keys(metadata).length > 0) {
    entry.metadata = metadata;
  }

  return entry;
}

function readOldDb(filePath) {
  if (!existsSync(filePath)) {
    console.warn(`  ⚠️  File not found: ${filePath}`);
    return [];
  }
  const raw = readFileSync(filePath, 'utf-8');
  const parsed = JSON.parse(raw);
  return parsed.objects || [];
}

let totalEntries = 0;

for (const game of GAMES) {
  console.log(`\n🎮 Processing ${game.id.toUpperCase()}...`);

  const charPath = join(OLD_DB_DIR, game.charFile);
  const otherPath = join(OLD_DB_DIR, game.otherFile);

  const charEntries = readOldDb(charPath);
  const otherEntries = readOldDb(otherPath);

  console.log(`  📋 Characters: ${charEntries.length}`);
  console.log(`  📋 Other:      ${otherEntries.length}`);

  const allOldEntries = [...charEntries, ...otherEntries];
  const newEntries = allOldEntries.map(transformEntry);

  const outputPath = join(NEW_DB_DIR, `${game.id}.json`);
  writeFileSync(outputPath, JSON.stringify(newEntries, null, 2) + '\n', 'utf-8');

  console.log(`  ✅ Written ${newEntries.length} entries → ${game.id}.json`);
  totalEntries += newEntries.length;
}

console.log(`\n🏁 Migration complete! Total entries: ${totalEntries}`);
console.log('📌 EFMI was not migrated (no OLD-DB source). Keeping existing template.\n');
