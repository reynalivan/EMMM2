import { describe, it, expect, vi } from 'vitest';
import { logger, initLogger } from './logger';
import { attachConsole, trace, info, error } from '@tauri-apps/plugin-log';

vi.mock('@tauri-apps/plugin-log', () => ({
  attachConsole: vi.fn(),
  trace: vi.fn(),
  info: vi.fn(),
  error: vi.fn(),
}));

describe('logger', () => {
  it('should initialize via attachConsole', async () => {
    await initLogger();
    expect(attachConsole).toHaveBeenCalled();
  });

  it('should export tauri log functions', () => {
    expect(logger.trace).toBe(trace);
    expect(logger.info).toBe(info);
    expect(logger.error).toBe(error);

    logger.info('Test Info');
    expect(info).toHaveBeenCalledWith('Test Info');
  });
});
