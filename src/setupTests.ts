import '@testing-library/jest-dom';
import { vi } from 'vitest';
import React from 'react';

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

// Mock lucide-react dynamically
vi.mock('lucide-react', async (importOriginal) => {
  const actual = await importOriginal<typeof import('lucide-react')>();
  const mockExports: Record<string, unknown> = { ...actual };
  for (const key in actual) {
    if (key === 'default' || key === '__esModule') continue;
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

vi.mock('@tauri-apps/plugin-fs', () => ({
  exists: vi.fn().mockResolvedValue(true),
  mkdir: vi.fn().mockResolvedValue(undefined),
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
    t: (key: string) => key,
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
    div: ({
      children,
      layout: _layout,
      ...props
    }: { children?: React.ReactNode } & Record<string, unknown>) =>
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      React.createElement('div', props as any, children),
    button: ({
      children,
      layout: _layout,
      ...props
    }: { children?: React.ReactNode } & Record<string, unknown>) =>
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      React.createElement('button', props as any, children),
    span: ({ children, ...props }: { children?: React.ReactNode } & Record<string, unknown>) =>
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      React.createElement('span', props as any, children),
    h1: ({ children, ...props }: { children?: React.ReactNode } & Record<string, unknown>) =>
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      React.createElement('h1', props as any, children),
    h2: ({ children, ...props }: { children?: React.ReactNode } & Record<string, unknown>) =>
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      React.createElement('h2', props as any, children),
    p: ({ children, ...props }: { children?: React.ReactNode } & Record<string, unknown>) =>
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      React.createElement('p', props as any, children),
    section: ({ children, ...props }: { children?: React.ReactNode } & Record<string, unknown>) =>
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      React.createElement('section', props as any, children),
    article: ({ children, ...props }: { children?: React.ReactNode } & Record<string, unknown>) =>
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      React.createElement('article', props as any, children),
    ul: ({ children, ...props }: { children?: React.ReactNode } & Record<string, unknown>) =>
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      React.createElement('ul', props as any, children),
    li: ({ children, ...props }: { children?: React.ReactNode } & Record<string, unknown>) =>
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      React.createElement('li', props as any, children),
  },
  AnimatePresence: ({ children }: { children: React.ReactNode }) => children,
  useAnimation: () => ({
    start: vi.fn(),
    stop: vi.fn(),
  }),
  Reorder: {
    Group: ({ children, ...props }: { children?: React.ReactNode } & Record<string, unknown>) =>
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      React.createElement('div', props as any, children),
    Item: ({
      children,
      layout: _layout,
      ...props
    }: { children?: React.ReactNode } & Record<string, unknown>) =>
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      React.createElement('div', props as any, children),
  },
}));
