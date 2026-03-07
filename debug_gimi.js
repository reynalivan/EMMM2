import fs from 'fs';

const data = JSON.parse(fs.readFileSync('./src-tauri/resources/databases/gimi.json', 'utf8'));

console.log('Entries is array?', Array.isArray(data.entries));
if (data.entries && data.entries.length > 0) {
  const entry = data.entries[0];
  console.log('First entry:', JSON.stringify(entry, null, 2));
  console.log('Does it have object_type?', 'object_type' in entry);
  console.log('object_type value:', entry.object_type);
}
