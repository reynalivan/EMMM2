/**
 * DISABLED prefix utilities — mirrors Rust `DISABLED_RE` from mod_core_cmds.rs.
 *
 * EMMM writes the canonical "DISABLED " spelling. Detection mirrors package
 * configs, which exclude every folder whose name starts with `DISABLED`.
 */

/** Regex matching the 3DMigoto package `DISABLED*` exclusion. */
const DISABLED_RE = /^disabled[\s_-]*/i;

/** Canonical prefix used when disabling a folder */
export const DISABLED_PREFIX = 'DISABLED ';

/** Check if a folder name is excluded by the runtime. */
export function isDisabledName(name: string): boolean {
  return DISABLED_RE.test(name);
}

/** Strip the runtime disabled prefix, returning the clean name. */
export function stripDisabledPrefix(name: string): string {
  return name.replace(DISABLED_RE, '').trim();
}

/**
 * This remains specific to rename-field shorthand such as `dis_`; runtime
 * detection itself intentionally mirrors `DISABLED*`.
 */
const TYPED_DISABLED_RE = /^(disabled|disable|dis)[_\-\s]+/i;

export function stripTypedDisabledPrefix(value: string): string {
  return value.replace(TYPED_DISABLED_RE, '');
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
