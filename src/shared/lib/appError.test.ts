import { describe, expect, it } from 'vitest';
import { archiveErrorKindFromStoredMessage, extractArchiveErrorKind } from './appError';

describe('archive error mapping', () => {
  it('reads the structured archive error returned by Tauri', () => {
    expect(
      extractArchiveErrorKind({
        type: 'ArchiveUnsupported',
        payload: { reason: 'dictionary_too_large' },
      }),
    ).toBe('dictionary_too_large');
  });

  it('recognizes legacy Windows diagnostics already stored in import jobs', () => {
    expect(
      archiveErrorKindFromStoredMessage(
        "Validation error: Unsupported archive: Extraction error: OS Error 42 (FormatMessageW returned error 317) 'Declared dictionary size is not supported'",
      ),
    ).toBe('dictionary_too_large');
  });
});
