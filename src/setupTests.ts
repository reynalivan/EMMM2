import '@testing-library/jest-dom';
import { vi } from 'vitest';

// Mock Tauri API globally
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(vi.fn()),
  emit: vi.fn(),
}));

// Mock typed commands bridge
vi.mock('@/lib/bindings', () => ({
  commands: new Proxy(
    {},
    {
      get: () => vi.fn().mockResolvedValue(null),
    },
  ),
  GameType: {
    GIMI: 0,
    SRMI: 1,
    WWMI: 2,
    ZZMI: 3,
    EFMI: 4,
  },
  ItemStatus: {
    Disabled: 0,
    Enabled: 1,
  },
}));

import React from 'react';

// Mock lucide-react dynamically
vi.mock('lucide-react', async (importOriginal) => {
  const actual = await importOriginal<typeof import('lucide-react')>();
  const mockExports: Record<string, unknown> = { ...actual };
  for (const key in actual) {
    if (key === 'default' || key === '__esModule') continue;
    // Mock every icon as an empty element to prevent text pollution in getByText
    mockExports[key] = () =>
      React.createElement('span', { 'data-testid': `icon-${key.toLowerCase()}` });
  }
  return mockExports;
});

vi.mock('@tanstack/react-query', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@tanstack/react-query')>();
  return {
    ...actual,
    useQuery: vi.fn().mockReturnValue({ data: null, isLoading: false, error: null }),
    useMutation: vi
      .fn()
      .mockReturnValue({ mutate: vi.fn(), mutateAsync: vi.fn(), isPending: false }),
    useQueryClient: vi.fn().mockReturnValue({ invalidateQueries: vi.fn(), setQueryData: vi.fn() }),
  };
});

vi.mock('@tauri-apps/plugin-log', () => ({
  info: vi.fn(),
  warn: vi.fn(),
  error: vi.fn(),
  trace: vi.fn(),
  debug: vi.fn(),
  attachConsole: vi.fn(),
}));

// Mock ResizeObserver for JSDOM
class ResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
}

window.ResizeObserver = ResizeObserver;

// Mock matchMedia for JSDOM
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: vi.fn().mockImplementation((query) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(), // deprecated
    removeListener: vi.fn(), // deprecated
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
});

// Mock HTMLDialogElement for JSDOM
if (typeof HTMLDialogElement !== 'undefined') {
  HTMLDialogElement.prototype.show = vi.fn(function mock(this: HTMLDialogElement) {
    this.open = true;
  });
  HTMLDialogElement.prototype.showModal = vi.fn(function mock(this: HTMLDialogElement) {
    this.open = true;
  });
  HTMLDialogElement.prototype.close = vi.fn(function mock(this: HTMLDialogElement) {
    this.open = false;
  });
}

// Mock react-i18next
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, options?: Record<string, unknown>) => {
      const keys: Record<string, string> = {
        'scanner:resolution.title': 'Confirm Resolution',
        'scanner:resolution.summary': `You are about to resolve ${options?.count} duplicate group(s). This will result in ${options?.deleted} file deletion(s) and ${options?.ignored} whitelist addition(s).`,
        'scanner:resolution.keep_specific': 'Keep Specific',
        'scanner:resolution.whitelist': 'Whitelist',
        'scanner:resolution.keep_label': 'KEEP:',
        'scanner:resolution.delete_others': `Delete ${options?.count} other identical item(s)`,
        'scanner:resolution.ignore_label': 'IGNORE:',
        'scanner:resolution.ignore_desc': `Whitelist all ${options?.count} members`,
        'scanner:resolution.confirm_button': 'Confirm & Resolve',
        'scanner:resolution.processing': 'Processing...',
        'scanner:resolution_modal.title': 'Confirm Resolution',
        'common:cancel': 'Cancel',
      };
      return keys[key] || key;
    },
    i18n: {
      changeLanguage: () => Promise.resolve(),
      language: 'en',
    },
  }),
  initReactI18next: {
    type: '3rdParty',
    init: () => {},
  },
}));

// Mock motion/react to avoid animation issues in jsdom
vi.mock('motion/react', () => ({
  motion: {
    div: ({ children, layout, ...props }: any) => React.createElement('div', props, children),
    button: ({ children, layout, ...props }: any) => React.createElement('button', props, children),
    span: ({ children, ...props }: any) => React.createElement('span', props, children),
    h1: ({ children, ...props }: any) => React.createElement('h1', props, children),
    h2: ({ children, ...props }: any) => React.createElement('h2', props, children),
    p: ({ children, ...props }: any) => React.createElement('p', props, children),
    section: ({ children, ...props }: any) => React.createElement('section', props, children),
    article: ({ children, ...props }: any) => React.createElement('article', props, children),
    ul: ({ children, ...props }: any) => React.createElement('ul', props, children),
    li: ({ children, ...props }: any) => React.createElement('li', props, children),
  },
  AnimatePresence: ({ children }: any) => children,
}));
