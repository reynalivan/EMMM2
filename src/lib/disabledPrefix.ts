/**
 * DISABLED prefix utilities — mirrors Rust `DISABLED_RE` from mod_core_cmds.rs.
 *
 * The canonical convention: folders prefixed with "DISABLED " are treated
 * as disabled mods. Detection stays strict to avoid false positives such as
 * `distance_mod` or `display`.
 */

/** Regex matching the canonical DISABLED prefix (case-insensitive) */
const DISABLED_RE = /^disabled\s+/i;

/** Canonical prefix used when disabling a folder */
export const DISABLED_PREFIX = 'DISABLED ';

/** Check if a folder name has the canonical disabled prefix */
export function isDisabledName(name: string): boolean {
  return DISABLED_RE.test(name);
}

/** Strip the canonical disabled prefix, returning the clean name */
export function stripDisabledPrefix(name: string): string {
  return name.replace(DISABLED_RE, '').trim();
}

/**
 * Apply or remove the DISABLED prefix from a folder path's basename.
 * Returns the updated full path (with `/` separators).
 */
export function toggleDisabledInPath(folderPath: string, enable: boolean): string {
  const parts = folderPath.split(/[/\\]/);
  const basename = parts[parts.length - 1];

  if (enable) {
    parts[parts.length - 1] = stripDisabledPrefix(basename);
  } else if (!isDisabledName(basename)) {
    parts[parts.length - 1] = DISABLED_PREFIX + basename;
  }

  return parts.join('/');
}
