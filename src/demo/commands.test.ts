import { describe, expect, it } from 'vitest';
import { resolveDemoCommand } from './commands';

describe('demo Mod Inbox commands', () => {
  it('reserves selected entries and records their completed move in memory', () => {
    const created = resolveDemoCommand('createModInboxBatch', [
      {
        gameId: 'demo-zenless',
        entryKeys: ['demo-inbox-archive', 'demo-inbox-folder'],
      },
    ]);

    expect(created).toMatchObject({
      handled: true,
      value: {
        id: 'demo-mod-inbox-batch',
        status: 'awaiting_review',
        items: [{ status: 'ready' }, { status: 'ready' }],
      },
    });
    expect(resolveDemoCommand('getModInbox', []).value).toMatchObject({
      readyEntries: [
        { pendingBatchId: 'demo-mod-inbox-batch' },
        { pendingBatchId: 'demo-mod-inbox-batch' },
      ],
    });

    const committed = resolveDemoCommand('commitImportBatch', [
      { batchId: 'demo-mod-inbox-batch', itemIds: [] },
    ]);

    expect(committed).toMatchObject({
      handled: true,
      value: { moved: 2, failed: 0 },
    });
    expect(resolveDemoCommand('getModInbox', []).value).toMatchObject({
      readyEntries: [],
      processedSources: [{ name: 'Nekomata Streetwear.zip' }, { name: 'Lumina Square Recolor' }],
    });
  });
});
