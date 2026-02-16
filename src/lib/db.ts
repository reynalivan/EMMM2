import Database from '@tauri-apps/plugin-sql';

let dbInstance: Database | null = null;

/** Max retries for SQLITE_BUSY / locked errors. */
const MAX_RETRIES = 3;
/** Base delay (ms) for exponential backoff: 50 → 100 → 200. */
const BASE_DELAY_MS = 50;

/**
 * Get or create the singleton DB connection.
 * If the previous connection is stale/broken, it auto-reconnects.
 */
export async function getDb(): Promise<Database> {
  if (!dbInstance) {
    dbInstance = await Database.load('sqlite:app.db');
  }
  return dbInstance;
}

/** Force-reset the DB singleton so the next call reconnects. */
export function resetDb(): void {
  dbInstance = null;
}

/** Check if an error is a retryable SQLITE_BUSY / locked condition. */
function isBusyError(err: unknown): boolean {
  const msg = String(err).toLowerCase();
  return msg.includes('database is locked') || msg.includes('sqlite_busy');
}

/** Sleep helper for retry backoff. */
function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export async function execute(query: string, values?: unknown[]) {
  let lastErr: unknown;
  for (let attempt = 0; attempt <= MAX_RETRIES; attempt++) {
    try {
      const db = await getDb();
      return await db.execute(query, values);
    } catch (err) {
      lastErr = err;
      if (isBusyError(err) && attempt < MAX_RETRIES) {
        await sleep(BASE_DELAY_MS * 2 ** attempt);
        continue;
      }
      resetDb();
      throw err;
    }
  }
  /* istanbul ignore next -- safety: loop always throws on final attempt */
  throw lastErr;
}

export async function select<T>(query: string, values?: unknown[]) {
  let lastErr: unknown;
  for (let attempt = 0; attempt <= MAX_RETRIES; attempt++) {
    try {
      const db = await getDb();
      return await db.select<T>(query, values);
    } catch (err) {
      lastErr = err;
      if (isBusyError(err) && attempt < MAX_RETRIES) {
        await sleep(BASE_DELAY_MS * 2 ** attempt);
        continue;
      }
      resetDb();
      throw err;
    }
  }
  /* istanbul ignore next -- safety: loop always throws on final attempt */
  throw lastErr;
}
