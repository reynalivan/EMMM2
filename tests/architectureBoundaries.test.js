import { describe, expect, it } from 'vitest';

import { evaluateBoundary } from '../eslint.arch.config.js';

const file = (relative) => `${process.cwd()}/src/${relative}`;

describe('frontend architecture boundaries', () => {
  it('rejects upward dependencies', () => {
    expect(evaluateBoundary(file('shared/lib/value.ts'), '@/features/example')).toMatchObject({
      messageId: 'upward',
    });
  });

  it('rejects same-layer cross-slice dependencies', () => {
    expect(
      evaluateBoundary(file('features/alpha/index.ts'), '@/features/beta'),
    ).toMatchObject({ messageId: 'crossSlice' });
  });

  it('rejects relative same-layer cross-slice dependencies', () => {
    expect(
      evaluateBoundary(file('features/alpha/view.ts'), '../beta/internal'),
    ).toMatchObject({ messageId: 'crossSlice' });
  });

  it('rejects deep imports across slices', () => {
    expect(
      evaluateBoundary(file('widgets/example/view.tsx'), '@/features/action/internal/useAction'),
    ).toMatchObject({ messageId: 'deepImport' });
  });

  it('rejects frontend filesystem, process, and updater capabilities', () => {
    for (const capability of [
      '@tauri-apps/plugin-fs',
      '@tauri-apps/plugin-process',
      '@tauri-apps/plugin-updater',
    ]) {
      expect(evaluateBoundary(file('app/entrypoint/main.tsx'), capability)).toMatchObject({
        messageId: 'capability',
      });
    }
  });

  it('allows imports through a lower-layer public API', () => {
    expect(evaluateBoundary(file('pages/home/index.ts'), '@/features/action')).toBeNull();
  });

  it('allows only the explicit global app-store facade', () => {
    expect(evaluateBoundary(file('features/action/index.ts'), '@/app/store')).toBeNull();
    expect(
      evaluateBoundary(file('features/action/index.ts'), '@/app/providers/queryClient'),
    ).toMatchObject({ messageId: 'upward' });
  });
});
