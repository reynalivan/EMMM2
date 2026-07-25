/** Pure mapping/formatting helpers for the archive-extraction flow. */

import type { ArchiveAnalysis, ArchiveInfo } from '../../../types/scanner';

export type ExtractionOutcome = { path: string; status: string };

export type MatchCheckOutcome = {
  folder: string;
  isMatch: boolean;
  matchedName?: string | null;
  matchScorePct: number;
};

/** Last path segment, tolerating both Windows and POSIX separators. */
export function fileNameOf(path: string): string {
  const segments = path.split(/[\\/]/);
  return segments[segments.length - 1] || path;
}

/**
 * Summary toast for a finished extraction batch, or null when nothing failed
 * (the happy path stays silent).
 */
export function buildExtractionSummary(results: ExtractionOutcome[]): string | null {
  const done = results.filter((r) => r.status === 'done').length;
  const failed = results.filter((r) => r.status === 'failed');
  const skipped = results.filter((r) => r.status === 'skipped').length;

  if (failed.length === 0) {
    return null;
  }

  const parts: string[] = [];
  if (done > 0) parts.push(`${done} extracted`);
  parts.push(`${failed.length} failed`);
  if (skipped > 0) parts.push(`${skipped} skipped`);

  const failedNames = failed.map((r) => fileNameOf(r.path)).join(', ');
  return `${parts.join(', ')}\nFailed: ${failedNames}`;
}

/**
 * Mismatch warning for items imported onto a specific object, or null when
 * every checked folder matched. `noun` names what was imported.
 */
export function buildMismatchWarning(
  checks: MatchCheckOutcome[],
  objectName: string,
  noun = 'archive(s)',
): { message: string; mismatchedPaths: string[] } | null {
  const mismatched = checks.filter((check) => !check.isMatch);
  if (mismatched.length === 0) {
    return null;
  }

  const first = mismatched[0];
  const detail = `${fileNameOf(first.folder)}: Best match is ${first.matchedName || 'Unknown'} (${first.matchScorePct}%)`;

  return {
    message: `${mismatched.length} of ${checks.length} ${noun} may not match ${objectName}\n→ ${detail}`,
    mismatchedPaths: mismatched.map((check) => check.folder),
  };
}

/**
 * Archive analysis → modal row. A null analysis means the probe failed; the
 * archive still has to appear in the modal, just without any detail.
 */
export function buildArchiveInfo(
  path: string,
  name: string,
  analysis: ArchiveAnalysis | null,
): ArchiveInfo {
  const base = { path, name, extension: name.split('.').pop() || '' };
  if (!analysis) {
    return {
      ...base,
      size_bytes: 0,
      has_ini: false,
      file_count: 0,
      is_encrypted: false,
      contains_nested_archives: false,
      entries: [],
    };
  }

  return {
    ...base,
    size_bytes: analysis.file_size_bytes ?? 0,
    has_ini: analysis.has_ini,
    file_count: analysis.file_count ?? 1,
    is_encrypted: analysis.is_encrypted || false,
    contains_nested_archives: analysis.contains_nested_archives || false,
    entries: analysis.entries,
  };
}
