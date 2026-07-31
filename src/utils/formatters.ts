/**
 * Centralized utility for data formatting.
 * Ensures consistent byte/size presentation across the app.
 */

/**
 * Formats a byte count into a human-readable string (e.g., "1.2 MB").
 * Supports units from B to PB.
 * Matches the logic previously duplicated inline in Dashboard and ConflictResolveDialog.
 *
 * @param bytes The number of bytes to format
 * @param decimals Precision (default: 1 for KB+, 0 for B)
 * @returns Formatted string
 */
export function formatBytes(bytes: number, decimals: number = 1): string {
  if (bytes <= 0) return '0 B';

  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB', 'PB', 'EB', 'ZB', 'YB'];

  const i = Math.floor(Math.log(bytes) / Math.log(k));

  // Determine actual precision based on index
  const precision = i === 0 ? 0 : dm;

  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(precision))} ${sizes[i]}`;
}

export type Translator = (key: string, options?: Record<string, unknown>) => string;

const HAS_TIMEZONE = /(?:Z|[+-]\d{2}:?\d{2})$/;

/**
 * Backend timestamps come in two shapes: naive UTC (`...T12:00:00`) and explicit
 * UTC (`...T12:00:00Z`). Only the naive form needs the `Z`, or JS parses it as
 * local time.
 */
function toEpochMs(dateInput: string | number): number {
  if (typeof dateInput === 'number') {
    return dateInput;
  }

  return new Date(HAS_TIMEZONE.test(dateInput) ? dateInput : `${dateInput}Z`).getTime();
}

// ponytail: kept the i18n keys instead of Intl.RelativeTimeFormat. The strings
// are hand-tuned compact forms ("5m ago", "5m yang lalu") across en/id/zh that
// Intl's CLDR output does not reproduce, and Intl would not follow i18next's
// active language without threading it through every call site.
export function formatRelativeDate(dateInput: string | number | null, t: Translator): string {
  if (!dateInput) {
    return t('common:date.unknown');
  }

  const then = toEpochMs(dateInput);
  const diffMs = Date.now() - then;
  const diffMin = Math.floor(diffMs / 60000);

  if (diffMin < 1) {
    return t('common:date.just_now');
  }

  if (diffMin < 60) {
    return t('common:date.mins_ago', { count: diffMin });
  }

  const diffHrs = Math.floor(diffMin / 60);
  if (diffHrs < 24) {
    return t('common:date.hours_ago', { count: diffHrs });
  }

  const diffDays = Math.floor(diffHrs / 24);
  if (diffDays < 30) {
    return t('common:date.days_ago', { count: diffDays });
  }

  return new Date(then).toLocaleDateString();
}
