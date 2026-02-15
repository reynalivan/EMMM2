// Character Names for Dummy Data
export const GENERATED_CHARS = [
  'Albedo',
  'Alhaitham',
  'Aloy',
  'Amber',
  'Arataki Itto',
  'Ayaka',
  'Ayato',
  'Baizhu',
  'Barbara',
  'Beidou',
  'Bennett',
  'Candace',
  'Charlotte',
  'Chiori',
  'Chongyun',
  'Clorinde',
  'Collei',
  'Cyno',
  'Dehya',
  'Diluc',
  'Diona',
  'Dori',
  'Eula',
  'Faruzan',
  'Fischl',
  'Freminet',
  'Furina',
  'Gaming',
  'Ganyu',
  'Gorou',
  'Hu Tao',
  'Jean',
  'Kazuha',
  'Kaeya',
  'Kaveh',
  'Keqing',
  'Kirara',
  'Klee',
  'Kokomi',
  'Kuki Shinobu',
  'Layla',
  'Lisa',
  'Lynette',
  'Lyney',
  'Mika',
  'Mona',
  'Nahida',
  'Navia',
  'Neuvillette',
  'Nilou',
  'Ningguang',
  'Noelle',
  'Qiqi',
  'Raiden Shogun',
  'Razor',
  'Rosaria',
  'Sara',
  'Sayu',
  'Shenhe',
  'Heizou',
  'Sucrose',
  'Tartaglia',
  'Thoma',
  'Tighnari',
  'Venti',
  'Wanderer',
  'Wriothesley',
  'Xiangling',
  'Xianyun',
  'Xiao',
  'Xingqiu',
  'Xinyan',
  'Yae Miko',
  'Yanfei',
  'Yaoyao',
  'Yelan',
  'Yoimiya',
  'Yun Jin',
  'Zhongli',
];

export interface DummyObject {
  id: string;
  name: string;
  count: number;
  enabled: boolean;
}

export const DUMMY_OBJECTS: DummyObject[] = GENERATED_CHARS.map((name, i) => ({
  id: `obj-${i}`,
  name,
  count: Math.floor(Math.random() * 15) + 1,
  enabled: Math.random() > 0.3,
}));

export const getGradient = (name: string) => {
  const hash = name.split('').reduce((acc, char) => char.charCodeAt(0) + acc, 0);
  const hue = hash % 360;
  return `linear-gradient(135deg, hsl(${hue}, 60%, 25%), hsl(${hue + 40}, 60%, 15%))`;
};

// Folder Grid Dummy Items
export const generateDummyItems = (count: number) => {
  return Array.from({ length: count }).map((_, i) => ({
    id: `item-${i}`,
    name: `Mod_Folder_Variant_${i + 1}`,
    type: Math.random() > 0.8 ? 'file' : 'folder',
    enabled: Math.random() > 0.5,
    imageUrl: Math.random() > 0.6 ? `https://picsum.photos/seed/${i}/300/200` : null,
  }));
};
