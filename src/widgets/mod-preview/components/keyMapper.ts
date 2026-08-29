/**
 * Maps a standard browser KeyboardEvent to a 3DMigoto valid key string.
 */
export function mapBrowserKeyTo3DMigoto(event: React.KeyboardEvent | KeyboardEvent): string | null {
  const code = event.code;
  const key = event.key;

  // Ignore modifier keys as standalones for the main key
  if (
    key === 'Shift' ||
    key === 'Control' ||
    key === 'Alt' ||
    key === 'Meta' ||
    key === 'OS' ||
    key === 'Tab' ||
    key === 'CapsLock' ||
    key === 'NumLock' ||
    key === 'ScrollLock'
  ) {
    return null;
  }

  // Handle Numpad codes specifically
  if (code.startsWith('Numpad')) {
    const numMatch = code.match(/^Numpad(\d)$/);
    if (numMatch) {
      return `VK_NUMPAD${numMatch[1]}`;
    }
    switch (code) {
      case 'NumpadMultiply':
        return 'VK_MULTIPLY';
      case 'NumpadAdd':
        return 'VK_ADD';
      case 'NumpadSubtract':
        return 'VK_SUBTRACT';
      case 'NumpadDecimal':
        return 'VK_DECIMAL';
      case 'NumpadDivide':
        return 'VK_DIVIDE';
      case 'NumpadEnter':
        return 'VK_RETURN';
    }
  }

  // Handle function keys F1-F24
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(key)) {
    return `VK_${key.toUpperCase()}`;
  }

  // Map known special keys to Virtual Keys (VK_*)
  const specialKeyMap: Record<string, string> = {
    ArrowUp: 'VK_UP',
    ArrowDown: 'VK_DOWN',
    ArrowLeft: 'VK_LEFT',
    ArrowRight: 'VK_RIGHT',
    Escape: 'VK_ESCAPE',
    Enter: 'VK_RETURN',
    ' ': 'VK_SPACE',
    Backspace: 'VK_BACK',
    Delete: 'VK_DELETE',
    Insert: 'VK_INSERT',
    Home: 'VK_HOME',
    End: 'VK_END',
    PageUp: 'VK_PRIOR',
    PageDown: 'VK_NEXT',
    ContextMenu: 'VK_APPS',
    Pause: 'VK_PAUSE',
    PrintScreen: 'VK_SNAPSHOT',
    Clear: 'VK_CLEAR',
  };

  if (specialKeyMap[key]) {
    return specialKeyMap[key];
  }

  // For regular alphanumeric characters or symbols, we use uppercase literal string (as allowed in standard 3DMigoto INIs),
  // or specific VK codes for robustness. But typical A-Z / 0-9 is preferred format.

  if (code.startsWith('Key')) {
    return code.replace('Key', '').toUpperCase();
  }

  if (code.startsWith('Digit')) {
    return code.replace('Digit', '');
  }

  // Some common punctuation (3DMigoto often expects VK_OEM_*)
  // We will map simple symbols back to raw character as users usually edit that way,
  // e.g. [, ], \, ;, ', ,, ., / etc.

  // Return literal character formatted (e.g., uppercase)
  if (key.length === 1) {
    return key.toUpperCase();
  }

  // Fallback
  return key.toUpperCase();
}
