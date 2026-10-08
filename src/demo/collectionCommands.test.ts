import { beforeEach, describe, expect, it, vi } from 'vitest';

async function command(name: string, args: unknown[] = []): Promise<unknown> {
  const { resolveDemoCollectionCommand } = await import('./collectionCommands');
  const result = resolveDemoCollectionCommand(name, args);
  if (!result) throw new Error(`Unsupported demo collection command: ${name}`);
  return await result.value;
}

describe('demo collection flow', () => {
  beforeEach(() => vi.resetModules());

  it('provides a typed apply preview with missing members and an Object being disabled', async () => {
    expect(
      await command('previewApplyCollection', ['demo-zenless', 'demo-collection-photo']),
    ).toMatchObject({
      collection_name: 'Photo Session',
      target_tree_nodes: expect.arrayContaining([
        expect.objectContaining({ id: 'demo-object-interface', is_enabled: false }),
        expect.objectContaining({
          id: 'demo-object-nekomata',
          children: expect.arrayContaining([
            expect.objectContaining({ status_kind: 'missing', is_effectively_active: false }),
          ]),
        }),
      ]),
    });
  });

  it('allows Skip Missing while retaining the original saved member and a modified live runtime', async () => {
    await expect(
      command('applyCollection', ['demo-zenless', 'demo-collection-photo', false]),
    ).rejects.toMatchObject({ type: 'MissingMods' });
    expect(
      await command('applyCollection', ['demo-zenless', 'demo-collection-photo', true]),
    ).toMatchObject({ partial_apply: true, mods_disabled: 1 });
    expect(await command('getCollectionRuntimeState')).toMatchObject({
      runtime_status: 'modified',
      projected_state: { summary: { missing_root_count: 0 } },
    });
    expect(await command('getCollectionPreview', ['demo-collection-photo'])).toMatchObject({
      collection: { is_active: false },
      projected_state: { summary: { missing_root_count: 1 } },
    });
  });
});
