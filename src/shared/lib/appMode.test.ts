import { describe, expect, it } from 'vitest';
import { resolveAppMode } from './appMode';

describe('resolveAppMode', () => {
  it('enables demo only for a development server', () => {
    expect(resolveAppMode({ dev: true, requestedMode: 'demo' })).toBe('demo');
  });

  it('keeps an unspecified mode on the application path', () => {
    expect(resolveAppMode({ dev: true, requestedMode: undefined })).toBe('app');
    expect(resolveAppMode({ dev: false, requestedMode: 'app' })).toBe('app');
  });

  it('rejects demo outside development', () => {
    expect(() => resolveAppMode({ dev: false, requestedMode: 'demo' })).toThrow(
      'Demo mode is only available',
    );
  });

  it('rejects unknown modes', () => {
    expect(() => resolveAppMode({ dev: true, requestedMode: 'preview' })).toThrow(
      'Unknown application mode',
    );
  });
});
