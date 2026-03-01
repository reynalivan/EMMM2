export const MODIFIER_TOKENS = new Set([
  'ctrl',
  'alt',
  'shift',
  'no_ctrl',
  'no_alt',
  'no_shift',
  'no_modifiers',
]);

export const VALID_VK_NAMES = new Set([
  // Original VK
  'LBUTTON',
  'RBUTTON',
  'CANCEL',
  'MBUTTON',
  'XBUTTON1',
  'XBUTTON2',
  'BACK',
  'TAB',
  'CLEAR',
  'RETURN',
  'SHIFT',
  'CONTROL',
  'MENU',
  'PAUSE',
  'CAPITAL',
  // IME
  'KANA',
  'HANGUL',
  'IME_ON',
  'JUNJA',
  'FINAL',
  'HANJA',
  'KANJI',
  'IME_OFF',
  'ESCAPE',
  'CONVERT',
  'NONCONVERT',
  'ACCEPT',
  'MODECHANGE',
  'SPACE',
  'PRIOR',
  'NEXT',
  'END',
  'HOME',
  'LEFT',
  'UP',
  'RIGHT',
  'DOWN',
  'SELECT',
  'PRINT',
  'EXECUTE',
  'SNAPSHOT',
  'INSERT',
  'DELETE',
  'HELP',
  // Windows keys
  'LWIN',
  'RWIN',
  'APPS',
  'SLEEP',
  // Numpad
  'NUMPAD0',
  'NUMPAD1',
  'NUMPAD2',
  'NUMPAD3',
  'NUMPAD4',
  'NUMPAD5',
  'NUMPAD6',
  'NUMPAD7',
  'NUMPAD8',
  'NUMPAD9',
  'MULTIPLY',
  'ADD',
  'SEPARATOR',
  'SUBTRACT',
  'DECIMAL',
  'DIVIDE',
  // Function keys
  'F1',
  'F2',
  'F3',
  'F4',
  'F5',
  'F6',
  'F7',
  'F8',
  'F9',
  'F10',
  'F11',
  'F12',
  'F13',
  'F14',
  'F15',
  'F16',
  'F17',
  'F18',
  'F19',
  'F20',
  'F21',
  'F22',
  'F23',
  'F24',
  // Locks
  'NUMLOCK',
  'SCROLL',
  // Specific modifiers
  'LSHIFT',
  'RSHIFT',
  'LCONTROL',
  'RCONTROL',
  'LMENU',
  'RMENU',
  // Browser
  'BROWSER_BACK',
  'BROWSER_FORWARD',
  'BROWSER_REFRESH',
  'BROWSER_STOP',
  'BROWSER_SEARCH',
  'BROWSER_FAVORITES',
  'BROWSER_HOME',
  // Volume
  'VOLUME_MUTE',
  'VOLUME_DOWN',
  'VOLUME_UP',
  // Media
  'MEDIA_NEXT_TRACK',
  'MEDIA_PREV_TRACK',
  'MEDIA_STOP',
  'MEDIA_PLAY_PAUSE',
  // Launch
  'LAUNCH_MAIL',
  'LAUNCH_MEDIA_SELECT',
  'LAUNCH_APP1',
  'LAUNCH_APP2',
  // OEM
  'OEM_1',
  'OEM_PLUS',
  'OEM_COMMA',
  'OEM_MINUS',
  'OEM_PERIOD',
  'OEM_2',
  'OEM_3',
  'OEM_4',
  'OEM_5',
  'OEM_6',
  'OEM_7',
  'OEM_8',
  'OEM_102',
  'OEM_CLEAR',
  // Gamepad
  'GAMEPAD_A',
  'GAMEPAD_B',
  'GAMEPAD_X',
  'GAMEPAD_Y',
  'GAMEPAD_RIGHT_SHOULDER',
  'GAMEPAD_LEFT_SHOULDER',
  'GAMEPAD_LEFT_TRIGGER',
  'GAMEPAD_RIGHT_TRIGGER',
  'GAMEPAD_DPAD_UP',
  'GAMEPAD_DPAD_DOWN',
  'GAMEPAD_DPAD_LEFT',
  'GAMEPAD_DPAD_RIGHT',
  'GAMEPAD_MENU',
  'GAMEPAD_VIEW',
  'GAMEPAD_LEFT_THUMBSTICK_BUTTON',
  'GAMEPAD_RIGHT_THUMBSTICK_BUTTON',
  'GAMEPAD_LEFT_THUMBSTICK_UP',
  'GAMEPAD_LEFT_THUMBSTICK_DOWN',
  'GAMEPAD_LEFT_THUMBSTICK_RIGHT',
  'GAMEPAD_LEFT_THUMBSTICK_LEFT',
  'GAMEPAD_RIGHT_THUMBSTICK_UP',
  'GAMEPAD_RIGHT_THUMBSTICK_DOWN',
  'GAMEPAD_RIGHT_THUMBSTICK_RIGHT',
  'GAMEPAD_RIGHT_THUMBSTICK_LEFT',
  // Misc
  'PROCESSKEY',
  'PACKET',
  'ATTN',
  'CRSEL',
  'EXSEL',
  'EREOF',
  'PLAY',
  'ZOOM',
  'NONAME',
  'PA1',
  // UI Navigation
  'NAVIGATION_VIEW',
  'NAVIGATION_MENU',
  'NAVIGATION_UP',
  'NAVIGATION_DOWN',
  'NAVIGATION_LEFT',
  'NAVIGATION_RIGHT',
  'NAVIGATION_ACCEPT',
  'NAVIGATION_CANCEL',
  // Hangul alias
  'HANGEUL',
  // OEM extensions
  'OEM_NEC_EQUAL',
  'OEM_FJ_JISHO',
  'OEM_FJ_MASSHOU',
  'OEM_FJ_TOUROKU',
  'OEM_FJ_LOYA',
  'OEM_FJ_ROYA',
  // ABNT
  'ABNT_C1',
  'ABNT_C2',
  // Extended/legacy
  'OEM_AX',
  'ICO_HELP',
  'ICO_00',
  'ICO_CLEAR',
  // Nokia/Ericsson
  'OEM_RESET',
  'OEM_JUMP',
  'OEM_PA1',
  'OEM_PA2',
  'OEM_PA3',
  'OEM_WSCTRL',
  'OEM_CUSEL',
  'OEM_ATTN',
  'OEM_FINISH',
  'OEM_COPY',
  'OEM_AUTO',
  'OEM_ENLW',
  'OEM_BACKTAB',
]);

/**
 * Checks if a token is a valid single printable character, hex-code, or VK name.
 */
export function isValidKeyToken(token: string): boolean {
  if (!token) return false;

  const upperToken = token.toUpperCase();

  // 1. Single printable character (ASCII 0x20 - 0x7E)
  if (token.length === 1) {
    const charCode = token.charCodeAt(0);
    if (charCode >= 0x20 && charCode <= 0x7e) {
      return true;
    }
  }

  // 2. Exact hex code matching (e.g. 0x1B, 0x01)
  if (/^0x[0-9a-fA-F]{1,4}$/i.test(token)) {
    return true;
  }

  // 3. Known VK_* name (with or without 'VK_' prefix)
  const canonicalName = upperToken.startsWith('VK_') ? upperToken.slice(3) : upperToken;
  if (VALID_VK_NAMES.has(canonicalName)) {
    return true;
  }

  return false;
}

/**
 * Validates a complete keybinding string (e.g. "ctrl no_alt F1")
 * Returns null if valid, or an error message string if invalid.
 */
export function validateKeyBinding(value: string): string | null {
  const trimmed = value.trim();
  if (!trimmed) {
    return 'Keybinding cannot be empty.';
  }

  // Split by whitespace
  const tokens = trimmed.split(/\s+/);
  if (tokens.length === 0) {
    return 'Keybinding cannot be empty.';
  }

  // The very last token MUST be the actual key token (char, VK name, hex)
  const keyToken = tokens[tokens.length - 1];
  if (!isValidKeyToken(keyToken)) {
    return `Invalid key token: "${keyToken}".`;
  }

  // If there are preceding tokens, they MUST be modifiers
  const modifierTokens = tokens.slice(0, -1);
  const seenModifiers = new Set<string>();

  for (const mod of modifierTokens) {
    const lowerMod = mod.toLowerCase();
    if (!MODIFIER_TOKENS.has(lowerMod)) {
      return `Invalid modifier: "${mod}".`;
    }
    if (seenModifiers.has(lowerMod)) {
      return `Duplicate modifier: "${mod}".`;
    }
    seenModifiers.add(lowerMod);
  }

  return null;
}
