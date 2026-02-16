/**
 * Normalize metadata in all MasterDB JSON files to align with game schemas.
 *
 * Fixes:
 * 1. GIMI/SRMI/WWMI: rarity "5" → "5-Star", "4" → "4-Star"
 * 2. ZZMI: rarity "5" → "S-Rank", "4" → "A-Rank"
 * 3. SRMI: rename metadata key "weapon" → "path", strip "The " prefix (except "The Hunt")
 * 4. ZZMI: add "specialty" to metadata from first matching tag
 * 5. WWMI/EFMI: object_type "Resonator"/"Operator" → "Character" (schema uses label for display)
 *
 * Usage: node scripts/normalize-metadata.mjs
 */
import { readFileSync, writeFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const DB_DIR = join(__dirname, '..', 'src-tauri', 'resources', 'databases');

// ─── Rarity Mapping ─────────────────────────────────────────────────

const STAR_RARITY = { 5: '5-Star', 4: '4-Star', 3: '3-Star' };
const ZZZ_RARITY = { 5: 'S-Rank', 4: 'A-Rank' };
const EFMI_RARITY = { 6: '6-Star', 5: '5-Star', 4: '4-Star' };

// ─── ZZZ Specialties (from tags) ────────────────────────────────────

const ZZMI_SPECIALTIES = new Set(['Attack', 'Stun', 'Anomaly', 'Support', 'Defense']);

// ─── SRMI Path value normalization ──────────────────────────────────
// Strip "The " prefix except for "The Hunt" (official name)
function normalizeSrmiPath(val) {
  if (!val) return val;
  if (val === 'The Hunt') return 'The Hunt';
  if (val.startsWith('The ')) return val.slice(4);
  return val;
}

// ─── Object Type Normalization ──────────────────────────────────────
// Games where MasterDB has game-specific type names → normalize to "Character"
const OBJECT_TYPE_MAP = {
  Resonator: 'Character',
  Operator: 'Character',
  Agent: 'Character',
};

// ─── Normalize a single game DB ─────────────────────────────────────

function normalizeGame(dbPath, gameId, opts = {}) {
  const db = JSON.parse(readFileSync(dbPath, 'utf-8'));
  let changes = 0;

  for (const entry of db) {
    // 1. Normalize object_type
    if (OBJECT_TYPE_MAP[entry.object_type]) {
      entry.object_type = OBJECT_TYPE_MAP[entry.object_type];
      changes++;
    }

    if (!entry.metadata) continue;

    // 2. Normalize rarity
    const rarityMap = opts.rarityMap || STAR_RARITY;
    if (entry.metadata.rarity && rarityMap[entry.metadata.rarity]) {
      entry.metadata.rarity = rarityMap[entry.metadata.rarity];
      changes++;
    }

    // 3. SRMI: rename "weapon" key → "path" and normalize values
    if (opts.renameWeaponToPath && entry.metadata.weapon !== undefined) {
      const rawVal = entry.metadata.weapon;
      entry.metadata.path = normalizeSrmiPath(rawVal);
      delete entry.metadata.weapon;
      changes++;
    }

    // 4. ZZMI: extract specialty from tags
    if (opts.addSpecialty && entry.tags && !entry.metadata.specialty) {
      const found = entry.tags.find((t) => ZZMI_SPECIALTIES.has(t));
      if (found) {
        entry.metadata.specialty = found;
        changes++;
      }
    }
  }

  writeFileSync(dbPath, JSON.stringify(db, null, 2) + '\n', 'utf-8');
  return { total: db.length, changes };
}

// ─── Execute ────────────────────────────────────────────────────────

console.log('\n🔧 Normalizing MasterDB metadata...\n');

console.log('🎮 GIMI — Genshin Impact');
const gimi = normalizeGame(join(DB_DIR, 'gimi.json'), 'gimi');
console.log(`  ✅ ${gimi.changes} changes across ${gimi.total} entries`);

console.log('🎮 SRMI — Honkai: Star Rail');
const srmi = normalizeGame(join(DB_DIR, 'srmi.json'), 'srmi', {
  renameWeaponToPath: true,
});
console.log(`  ✅ ${srmi.changes} changes across ${srmi.total} entries`);

console.log('🎮 ZZMI — Zenless Zone Zero');
const zzmi = normalizeGame(join(DB_DIR, 'zzmi.json'), 'zzmi', {
  rarityMap: ZZZ_RARITY,
  addSpecialty: true,
});
console.log(`  ✅ ${zzmi.changes} changes across ${zzmi.total} entries`);

console.log('🎮 WWMI — Wuthering Waves');
const wwmi = normalizeGame(join(DB_DIR, 'wwmi.json'), 'wwmi');
console.log(`  ✅ ${wwmi.changes} changes across ${wwmi.total} entries`);

console.log('🎮 EFMI — Endfield');
const efmi = normalizeGame(join(DB_DIR, 'efmi.json'), 'efmi', {
  rarityMap: EFMI_RARITY,
});
console.log(`  ✅ ${efmi.changes} changes across ${efmi.total} entries`);

const total = gimi.changes + srmi.changes + zzmi.changes + wwmi.changes + efmi.changes;
console.log(`\n🏁 Done! ${total} total changes applied.\n`);
