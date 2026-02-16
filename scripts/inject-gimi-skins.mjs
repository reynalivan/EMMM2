/**
 * Inject GIMI outfit/skin data into the database.
 * Usage: node scripts/inject-gimi-skins.mjs
 */
import { readFileSync, writeFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const DB_PATH = join(__dirname, '..', 'src-tauri', 'resources', 'databases', 'gimi.json');

// Skin data from the official outfit list
// Each key = character name (must match DB entry name exactly)
// Each value = array of { name, aliases } for each outfit
const GIMI_SKINS = {
  Diluc: [{ name: 'Red Dead of Night', aliases: ['DilucRed', 'Diluc2', 'DilucNight'] }],
  Jean: [
    { name: "Gunnhildr's Heritage", aliases: ['JeanCN', 'Jean2'] },
    { name: 'Sea Breeze Dandelion', aliases: ['JeanSea', 'Jean Sea Breeze', 'Jean3'] },
  ],
  Amber: [{ name: '100% Outrider', aliases: ['AmberCN', 'Amber2'] }],
  Mona: [{ name: 'Pact of Stars and Moon', aliases: ['MonaCN', 'Mona2'] }],
  Rosaria: [{ name: "To the Church's Free Spirit", aliases: ['RosariaCN', 'Rosaria2'] }],
  Keqing: [{ name: 'Opulent Splendor', aliases: ['KeqingOpulent', 'Keqing2'] }],
  'Kamisato Ayaka': [{ name: 'Springbloom Missive', aliases: ['AyakaSpringbloom', 'Ayaka2'] }],
  Ganyu: [{ name: 'Twilight Blossom', aliases: ['GanyuTwilight', 'Ganyu2'] }],
  Shenhe: [{ name: 'Frostflower Dew', aliases: ['ShenheFrostflower', 'Shenhe2'] }],
  Klee: [{ name: 'Blossoming Starlight', aliases: ['KleeBlossoming', 'Klee2'] }],
  Nilou: [{ name: 'Breeze of Sabaa', aliases: ['NilouBreeze', 'Nilou2'] }],
  Neuvillette: [{ name: 'Lunar Splendor', aliases: ['NeuvilletteLunar', 'Neuvillette2'] }],
  Barbara: [{ name: 'Summertime Sparkle', aliases: ['BarbaraSummer', 'Barbara2'] }],
  Ningguang: [{ name: "Orchid's Evening Gown", aliases: ['NingguangOrchid', 'Ningguang2'] }],
  Fischl: [{ name: 'Ein Immernachtstraum', aliases: ['FischlFantasy', 'Fischl2'] }],
  Lisa: [{ name: 'A Sobriquet Under Shade', aliases: ['LisaScholar', 'Lisa2'] }],
  Kaeya: [{ name: 'Sailwind Shadow', aliases: ['KaeyaSailwind', 'Kaeya2'] }],
  Xingqiu: [{ name: 'Bamboo Rain', aliases: ['XingqiuBamboo', 'Xingqiu2'] }],
  Kirara: [{ name: 'Phantom in Boots', aliases: ['KiraraBoots', 'Kirara2'] }],
  Bennett: [{ name: 'Sunspray Resort', aliases: ['BennettSunspray', 'Bennett2'] }],
  Yaoyao: [{ name: 'Spring Celebration', aliases: ['YaoyaoSpring', 'Yaoyao2'] }],
};

// Traveler skins need special handling — match both Aether and Lumine
const TRAVELER_SKIN = {
  name: 'Origin of the Stars',
  aliases: ['TravelerOrigin', 'Traveler2'],
};

const db = JSON.parse(readFileSync(DB_PATH, 'utf-8'));

let matched = 0;
let travelerMatched = 0;

for (const entry of db) {
  // Check for character-specific skins
  if (GIMI_SKINS[entry.name]) {
    const skins = GIMI_SKINS[entry.name];
    // custom_skins = flat list of skin names
    entry.custom_skins = skins.map((s) => s.name);
    // Add skin aliases to tags for better matching
    for (const skin of skins) {
      entry.tags.push(...skin.aliases);
    }
    matched++;
    console.log(`  ✅ ${entry.name}: ${skins.map((s) => s.name).join(', ')}`);
  }

  // Traveler applies to Aether and Lumine
  if (entry.name === 'Aether' || entry.name === 'Lumine') {
    entry.custom_skins = [TRAVELER_SKIN.name];
    entry.tags.push(...TRAVELER_SKIN.aliases);
    travelerMatched++;
    console.log(`  ✅ ${entry.name} (Traveler): ${TRAVELER_SKIN.name}`);
  }
}

writeFileSync(DB_PATH, JSON.stringify(db, null, 2) + '\n', 'utf-8');

console.log(
  `\n🏁 Injected skins for ${matched} characters + ${travelerMatched} traveler variants.`,
);
