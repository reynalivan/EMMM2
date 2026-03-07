import sqlite3 from 'sqlite3';

const db = new sqlite3.Database('./src-tauri/resources/app.db', sqlite3.OPEN_READONLY, (err) => {
  if (err) {
    console.error(err.message);
  }
  console.log('Connected to the app.db database.');
});

db.serialize(() => {
  db.each(
    `SELECT id, name, object_type, thumbnail_path, metadata FROM objects ORDER BY created_at DESC LIMIT 5`,
    (err, row) => {
      if (err) {
        console.error(err.message);
      }
      console.log(row.id + '\t' + row.name + '\t' + row.object_type + '\t' + row.thumbnail_path);
    },
  );
});

db.close();
