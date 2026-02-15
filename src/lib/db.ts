import Database from '@tauri-apps/plugin-sql';

let dbInstance: Database | null = null;

export async function getDb(): Promise<Database> {
  if (!dbInstance) {
    dbInstance = await Database.load('sqlite:app.db');
  }
  return dbInstance;
}

export async function execute(query: string, values?: unknown[]) {
  const db = await getDb();
  return db.execute(query, values);
}

export async function select<T>(query: string, values?: unknown[]) {
  const db = await getDb();
  return db.select<T>(query, values);
}
