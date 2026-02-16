/**
 * Inject structured custom_skins data into all game databases.
 * Each skin has { name, aliases, thumbnail_skin_path, rarity }.
 *
 * Usage: node scripts/inject-skins.mjs
 */
import { readFileSync, writeFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const DB_DIR = join(__dirname, '..', 'src-tauri', 'resources', 'databases');

/** Normalize a name to kebab-case for thumbnail paths */
function toSlug(str) {
  return str
    .toLowerCase()
    .replace(/['']/g, '')
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '');
}

/** Build a skin entry with all required fields */
function skin(game, charName, name, aliases, rarity) {
  const thumbPath = `app/assets/thumbnails/${game}/skin/${toSlug(charName)}_${toSlug(name)}.png`;
  return { name, aliases, thumbnail_skin_path: thumbPath, rarity: String(rarity) };
}

// ─── GIMI (Genshin Impact) ──────────────────────────────────────────
const g = (char, name, aliases, rarity) => skin('gimi', char, name, aliases, rarity);

const GIMI_SKINS = {
  Diluc: [g('Diluc', 'Red Dead of Night', ['DilucRed', 'Diluc2', 'DilucNight'], 5)],
  Jean: [
    g('Jean', "Gunnhildr's Heritage", ['JeanCN', 'Jean2'], 5),
    g('Jean', 'Sea Breeze Dandelion', ['JeanSea', 'Jean Sea Breeze', 'Jean3'], 5),
  ],
  Amber: [g('Amber', '100% Outrider', ['AmberCN', 'Amber2'], 4)],
  Mona: [g('Mona', 'Pact of Stars and Moon', ['MonaCN', 'Mona2'], 5)],
  Rosaria: [g('Rosaria', "To the Church's Free Spirit", ['RosariaCN', 'Rosaria2'], 4)],
  Keqing: [g('Keqing', 'Opulent Splendor', ['KeqingOpulent', 'Keqing2'], 5)],
  'Kamisato Ayaka': [g('Ayaka', 'Springbloom Missive', ['AyakaSpringbloom', 'Ayaka2'], 5)],
  Ganyu: [g('Ganyu', 'Twilight Blossom', ['GanyuTwilight', 'Ganyu2'], 5)],
  Shenhe: [g('Shenhe', 'Frostflower Dew', ['ShenheFrostflower', 'Shenhe2'], 5)],
  Klee: [g('Klee', 'Blossoming Starlight', ['KleeBlossoming', 'Klee2'], 5)],
  Nilou: [g('Nilou', 'Breeze of Sabaa', ['NilouBreeze', 'Nilou2'], 5)],
  Neuvillette: [g('Neuvillette', 'Lunar Splendor', ['NeuvilletteLunar', 'Neuvillette2'], 5)],
  Barbara: [g('Barbara', 'Summertime Sparkle', ['BarbaraSummer', 'Barbara2'], 4)],
  Ningguang: [g('Ningguang', "Orchid's Evening Gown", ['NingguangOrchid', 'Ningguang2'], 4)],
  Fischl: [g('Fischl', 'Ein Immernachtstraum', ['FischlFantasy', 'Fischl2'], 4)],
  Lisa: [g('Lisa', 'A Sobriquet Under Shade', ['LisaScholar', 'Lisa2'], 4)],
  Kaeya: [g('Kaeya', 'Sailwind Shadow', ['KaeyaSailwind', 'Kaeya2'], 4)],
  Xingqiu: [g('Xingqiu', 'Bamboo Rain', ['XingqiuBamboo', 'Xingqiu2'], 4)],
  Kirara: [g('Kirara', 'Phantom in Boots', ['KiraraBoots', 'Kirara2'], 4)],
  Bennett: [g('Bennett', 'Sunspray Resort', ['BennettSunspray', 'Bennett2'], 4)],
  Yaoyao: [g('Yaoyao', 'Spring Celebration', ['YaoyaoSpring', 'Yaoyao2'], 4)],
};

// Traveler skins apply to both Aether and Lumine
const GIMI_TRAVELER_SKIN = skin(
  'gimi',
  'Traveler',
  'Origin of the Stars',
  ['TravelerOrigin', 'Traveler2'],
  5,
);

// ─── SRMI (Honkai: Star Rail) ───────────────────────────────────────
const s = (char, name, aliases, rarity) => skin('srmi', char, name, aliases, rarity);

const SRMI_SKINS = {
  'Ruan Mei': [s('Ruan Mei', 'Plumblossom Letter', ['RuanPlum', 'Ruan2', 'RuanLetter'], 5)],
  Herta: [s('Herta', "The Doll's Gala", ['HertaGala', 'Herta2'], 5)],
  Welt: [s('Welt', 'Back to the Past', ['WeltPast', 'Welt2'], 5)],
  Kafka: [s('Kafka', 'Night of the Spider', ['KafkaNight', 'Kafka2'], 5)],
};

// March 7th skins apply to both Preservation and Hunt variants
const SRMI_MARCH_SKIN = s('March 7th', "Be the Show's Star", ['MarchShow', 'March2'], 4);

// Trailblazer skins apply to both Caelus and Stelle
const SRMI_TB_SKIN = s('Trailblazer', 'Vim and Vigor', ['TBVigor', 'TB2', 'Trailblazer2'], 5);

// ─── ZZMI (Zenless Zone Zero) ───────────────────────────────────────
const z = (char, name, aliases, rarity) => skin('zzmi', char, name, aliases, rarity);

const ZZMI_SKINS = {
  'Anby Demara': [z('Anby', 'Cunning Streetwear', ['AnbyStreet', 'Anby2'], 4)],
  'Nicole Demara': [z('Nicole', 'Golden Tycoon', ['NicoleGold', 'Nicole2'], 4)],
  'Ellen Joe': [z('Ellen', 'Midnight Service', ['EllenService', 'Ellen2', 'EllenNight'], 5)],
  'Billy Kid': [z('Billy', 'Starlight Chrome', ['BillyChrome', 'Billy2'], 4)],
  'Zhu Yuan': [z('Zhu Yuan', 'Off-Duty Azure', ['ZhuOffDuty', 'Zhu2'], 5)],
};

// Belle/Wise skin applies to both protagonists
const ZZMI_MC_SKIN = z('Proxy', 'New Eridu Casual', ['MCStreet', 'MC2', 'ProxyCasual'], 5);

// ─── WWMI (Wuthering Waves) ─────────────────────────────────────────
const w = (char, name, aliases, rarity) => skin('wwmi', char, name, aliases, rarity);

const WWMI_SKINS = {
  Jinhsi: [w('Jinhsi', 'Resplendent Moon', ['JinshiMoon', 'Jinshi2', 'JinshiPremium'], 5)],
  Changli: [w('Changli', 'Vermillion Feathers', ['ChangliFeathers', 'Changli2', 'ChangliRed'], 5)],
  Baizhi: [w('Baizhi', 'Ethereal Frost', ['BaizhiFrost', 'Baizhi2'], 4)],
  Sanhua: [w('Sanhua', 'Glacial Grace', ['SanhuaGrace', 'Sanhua2'], 4)],
  Yinlin: [w('Yinlin', 'Nightshade Bloom', ['YinlinNight', 'Yinlin2', 'YinlinBloom'], 5)],
  Chixia: [w('Chixia', 'Blazing Heroine', ['ChixiaHero', 'Chixia2'], 4)],
  Shorekeeper: [w('Shorekeeper', 'Abyssal Echo', ['ShorekeeperEcho', 'Shorekeeper2'], 5)],
};

// Rover skin applies to both Male and Female
const WWMI_ROVER_SKIN = w('Rover', 'Tides of Destiny', ['RoverTides', 'Rover2', 'MCRover2'], 5);

// ─── Injection Logic ────────────────────────────────────────────────

function injectSkins(dbPath, skinMap, specialEntries = []) {
  const db = JSON.parse(readFileSync(dbPath, 'utf-8'));
  let count = 0;

  for (const entry of db) {
    // Direct character match
    if (skinMap[entry.name]) {
      entry.custom_skins = skinMap[entry.name];
      count++;
    }

    // Special multi-character entries (Traveler, March 7th variants, etc.)
    for (const { match, skin } of specialEntries) {
      if (match(entry)) {
        entry.custom_skins = [skin];
        count++;
      }
    }
  }

  writeFileSync(dbPath, JSON.stringify(db, null, 2) + '\n', 'utf-8');
  return count;
}

// ─── Execute ────────────────────────────────────────────────────────

console.log('\n🎮 GIMI — Genshin Impact');
const gimiCount = injectSkins(join(DB_DIR, 'gimi.json'), GIMI_SKINS, [
  { match: (e) => e.name === 'Aether' || e.name === 'Lumine', skin: GIMI_TRAVELER_SKIN },
]);
console.log(`  ✅ ${gimiCount} entries updated`);

console.log('\n🎮 SRMI — Honkai: Star Rail');
const srmiCount = injectSkins(join(DB_DIR, 'srmi.json'), SRMI_SKINS, [
  { match: (e) => e.name.startsWith('March 7th'), skin: SRMI_MARCH_SKIN },
  { match: (e) => e.name === 'Caelus' || e.name === 'Stelle', skin: SRMI_TB_SKIN },
]);
console.log(`  ✅ ${srmiCount} entries updated`);

console.log('\n🎮 ZZMI — Zenless Zone Zero');
const zzmiCount = injectSkins(join(DB_DIR, 'zzmi.json'), ZZMI_SKINS, [
  { match: (e) => e.name === 'Belle' || e.name === 'Wise', skin: ZZMI_MC_SKIN },
]);
console.log(`  ✅ ${zzmiCount} entries updated`);

console.log('\n🎮 WWMI — Wuthering Waves');
const wwmiCount = injectSkins(join(DB_DIR, 'wwmi.json'), WWMI_SKINS, [
  { match: (e) => e.name === 'Rover Male' || e.name === 'Rover Female', skin: WWMI_ROVER_SKIN },
]);
console.log(`  ✅ ${wwmiCount} entries updated`);

const total = gimiCount + srmiCount + zzmiCount + wwmiCount;
console.log(`\n🏁 Done! ${total} characters updated across all games.\n`);
