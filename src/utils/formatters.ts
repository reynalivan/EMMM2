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

/**
 * Alias for formatBytes to match expectations of components like DuplicateTable.
 */
export const formatSize = formatBytes;
