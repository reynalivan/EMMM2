import { describe, expect, it, vi } from 'vitest';

vi.mock('../lib/i18n', () => ({
  default: {
    t: (key: string, options?: Record<string, unknown>) =>
      options ? `${key}|${JSON.stringify(options)}` : key,
  },
}));

vi.mock('../lib/appError', () => ({
  formatAppError: (error: unknown) => `formatted:${String(error)}`,
}));

import {
  BULK_TOAST_PREVIEW_LIMIT,
  formatBulkFailureMessage,
  formatBulkSuccessMessage,
} from './bulkToastMessages';

describe('formatBulkSuccessMessage', () => {
  it('stays silent for an empty batch', () => {
    expect(formatBulkSuccessMessage([], 'enabled')).toBe('');
  });

  it('names every folder while the batch fits the preview limit', () => {
    const names = Array.from({ length: BULK_TOAST_PREVIEW_LIMIT }, (_, i) => `Mod ${i + 1}`);

    expect(formatBulkSuccessMessage(names, 'deleted')).toBe(
      'grid:bulk_toast.success|' +
        JSON.stringify({
          action: 'grid:bulk_toast.actions.deleted',
          count: BULK_TOAST_PREVIEW_LIMIT,
          names: names.join(', '),
        }),
    );
  });

  it('truncates past the preview limit and counts the remainder', () => {
    const names = Array.from({ length: BULK_TOAST_PREVIEW_LIMIT + 3 }, (_, i) => `Mod ${i + 1}`);

    expect(formatBulkSuccessMessage(names, 'pinned')).toBe(
      'grid:bulk_toast.success_with_more|' +
        JSON.stringify({
          action: 'grid:bulk_toast.actions.pinned',
          count: BULK_TOAST_PREVIEW_LIMIT + 3,
          extraCount: 3,
          names: names.slice(0, BULK_TOAST_PREVIEW_LIMIT).join(', '),
        }),
    );
  });
});

describe('formatBulkFailureMessage', () => {
  it('stays silent when nothing failed', () => {
    expect(formatBulkFailureMessage([], 'toggle')).toBe('');
  });

  it('names the single failure with its disabled prefix stripped', () => {
    expect(
      formatBulkFailureMessage([{ path: 'C:\\Mods\\DISABLED Ayaka', error: 'locked' }], 'toggle'),
    ).toBe(
      'grid:bulk_toast.failure_one|' +
        JSON.stringify({
          action: 'grid:bulk_toast.failure_actions.toggle',
          name: 'Ayaka',
          reason: 'formatted:locked',
        }),
    );
  });

  it('reports the first failure and counts the rest', () => {
    expect(
      formatBulkFailureMessage(
        [
          { path: '/mods/Ayaka', error: 'locked' },
          { path: '/mods/Nahida', error: 'busy' },
          { path: '/mods/Klee', error: 'busy' },
        ],
        'delete',
      ),
    ).toBe(
      'grid:bulk_toast.failure_other|' +
        JSON.stringify({
          action: 'grid:bulk_toast.failure_actions.delete',
          extraCount: 2,
          name: 'Ayaka',
          reason: 'formatted:locked',
        }),
    );
  });

  it('falls back to the whole path when it has no trailing segment', () => {
    expect(formatBulkFailureMessage([{ path: '/mods/', error: 'boom' }], 'import')).toContain(
      '"name":"/mods/"',
    );
  });
});
