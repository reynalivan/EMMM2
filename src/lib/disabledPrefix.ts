/**
 * DISABLED prefix utilities — mirrors Rust `DISABLED_RE` from mod_core_cmds.rs.
 *
 * The canonical convention: folders prefixed with "DISABLED " (uppercase + space)
 * are treated as disabled mods. However, users and other tools may create folders
 * with variants like `disabled_`, `DISABLED-`, `dis `, `DISabled_`, etc.
 *
 * These utilities detect and strip ALL known variants, matching the backend regex:
 *   (?i)^(disabled|disable|dis)[_\-\s]*
 */

/** Regex matching all DISABLED prefix variants (case-insensitive) */
const DISABLED_RE = /^(disabled|disable|dis)[_\-\s]*/i;

/** Canonical prefix used when disabling a folder */
export const DISABLED_PREFIX = 'DISABLED ';

/** Check if a folder name has any disabled prefix variant */
export function isDisabledName(name: string): boolean {
  return DISABLED_RE.test(name);
}

/** Strip any disabled prefix variant, returning the clean name */
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
