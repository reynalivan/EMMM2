import { describe, it, expect, vi, beforeEach } from 'vitest';
import { getObjects } from './objectService';
import * as db from '../lib/db';

// Mock the DB module
vi.mock('../lib/db', () => ({
  select: vi.fn(),
  execute: vi.fn(),
  getDb: vi.fn(),
}));

describe('objectService.getObjects (SQL Generation)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('generates correct SQL for meta_filters (US-3.4)', async () => {
    // Arrange
    const filter = {
      game_id: 'genshin',
      safe_mode: true,
      meta_filters: {
        Element: ['Pyro', 'Hydro'],
        Rarity: ['5-Star'],
      },
    };

    // Act
    await getObjects(filter);

    // Assert
    const selectMock = db.select as unknown as ReturnType<typeof vi.fn>;
    const [query, params] = selectMock.mock.calls[0];

    // Verify SQL contains JSON_EXTRACT clauses
    expect(query).toContain("JSON_EXTRACT(o.metadata, '$.Element') IN");
    expect(query).toContain("JSON_EXTRACT(o.metadata, '$.Rarity') IN");

    // Verify params
    // Note: The specific implementation might use parameters differently,
    // but we expect at least the game_id and filter values to be passed somehow
    expect(params).toContain('genshin');
    // We expect the array values to be flattened or handled in the query construction
    // For now, let's just check the query structure
  });

  it('generates correct ORDER BY for sort_by (US-3.4)', async () => {
    // Arrange
    const filter = {
      game_id: 'genshin',
      safe_mode: true,
      sort_by: 'date' as const,
    };

    // Act
    await getObjects(filter);

    // Assert
    const selectMock = db.select as unknown as ReturnType<typeof vi.fn>;
    const [query] = selectMock.mock.calls[0];

    // Expect ORDER BY created_at DESC (assuming date means newest)
    expect(query).toContain('ORDER BY o.created_at DESC');
  });

  it('generates correct HAVING clause for status_filter (US-3.4)', async () => {
    // Arrange
    const filter = {
      game_id: 'genshin',
      safe_mode: true,
      status_filter: 'enabled' as const,
    };

    // Act
    await getObjects(filter);

    // Assert
    const selectMock = db.select as unknown as ReturnType<typeof vi.fn>;
    const [query] = selectMock.mock.calls[0];

    // Expect HAVING enabled_count > 0
    expect(query).toContain('HAVING enabled_count > 0');
  });
});
