import '@testing-library/jest-dom';
import { vi } from 'vitest';
import React from 'react';
import browserEn from './locales/en/browser.json';
import collectionsEn from './locales/en/collections.json';
import commonEn from './locales/en/common.json';
import dashboardEn from './locales/en/dashboard.json';
import folderGridEn from './locales/en/folder_grid.json';
import gridEn from './locales/en/grid.json';
import layoutEn from './locales/en/layout.json';
import objectsEn from './locales/en/objects.json';
import onboardingEn from './locales/en/onboarding.json';
import previewEn from './locales/en/preview.json';
import safeModeEn from './locales/en/safe_mode.json';
import scannerEn from './locales/en/scanner.json';
import settingsEn from './locales/en/settings.json';
import welcomeEn from './locales/en/welcome.json';

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
  readFile: vi.fn().mockResolvedValue(new Uint8Array()),
  writeFile: vi.fn().mockResolvedValue(undefined),
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

type TranslationNode =
  | string
  | number
  | boolean
  | null
  | TranslationNode[]
  | { readonly [key: string]: TranslationNode };

type TranslationOptions = Record<string, string | number | boolean | null | undefined>;
type NamespaceInput = string | readonly string[] | undefined;

const englishResources: Record<string, TranslationNode> = {
  browser: browserEn,
  collections: collectionsEn,
  common: commonEn,
  dashboard: dashboardEn,
  folder_grid: folderGridEn,
  grid: gridEn,
  layout: layoutEn,
  objects: objectsEn,
  onboarding: onboardingEn,
  preview: previewEn,
  safe_mode: safeModeEn,
  scanner: scannerEn,
  settings: settingsEn,
  welcome: welcomeEn,
};

function normalizeNamespaces(namespace: NamespaceInput): string[] {
  if (Array.isArray(namespace)) {
    return [...namespace];
  }

  if (typeof namespace === 'string' && namespace.length > 0) {
    return [namespace];
  }

  return ['common'];
}

function resolveTranslationPath(resource: TranslationNode, path: string): string | null {
  const value = path.split('.').reduce<TranslationNode | undefined>((current, segment) => {
    if (!current || typeof current !== 'object' || Array.isArray(current)) {
      return undefined;
    }

    return current[segment];
  }, resource);

  if (typeof value === 'string') {
    return value;
  }

  if (typeof value === 'number' || typeof value === 'boolean') {
    return String(value);
  }

  return null;
}

function interpolateTranslation(text: string, options: TranslationOptions | undefined): string {
  if (!options) {
    return text;
  }

  return text.replace(/\{\{\s*([a-zA-Z0-9_]+)\s*\}\}/g, (match, key: string) => {
    const value = options[key];
    return value === undefined || value === null ? match : String(value);
  });
}

function candidateKeys(key: string, options: TranslationOptions | undefined): string[] {
  if (typeof options?.count === 'number' && options.count === 1) {
    return [`${key}_one`, key];
  }

  if (typeof options?.count === 'number' && options.count !== 1) {
    return [`${key}_other`, `${key}_plural`, key];
  }

  return [key];
}

function translateKey(
  key: string,
  namespaces: readonly string[],
  options?: TranslationOptions,
): string {
  const separatorIndex = key.indexOf(':');
  const explicitNamespace = separatorIndex >= 0 ? key.slice(0, separatorIndex) : null;
  const explicitKey = separatorIndex >= 0 ? key.slice(separatorIndex + 1) : key;
  const searchNamespaces = explicitNamespace
    ? [explicitNamespace]
    : [...namespaces, 'common', ...Object.keys(englishResources)];

  for (const namespace of searchNamespaces) {
    const resource = englishResources[namespace];
    if (!resource) {
      continue;
    }

    for (const candidateKey of candidateKeys(explicitKey, options)) {
      const value = resolveTranslationPath(resource, candidateKey);
      if (value !== null) {
        return interpolateTranslation(value, options);
      }
    }
  }

  return key;
}

// Mock react-i18next with English resources so component tests assert user-facing copy.
vi.mock('react-i18next', () => ({
  useTranslation: (namespace?: NamespaceInput) => ({
    t: (key: string, options?: TranslationOptions) =>
      translateKey(key, normalizeNamespaces(namespace), options),
    i18n: {
      changeLanguage: () => Promise.resolve(),
      language: 'en',
    },
  }),
  initReactI18next: {
    type: '3rdParty',
    init: () => {},
  },
  Trans: ({
    i18nKey,
    values,
    children,
  }: {
    i18nKey?: string;
    values?: TranslationOptions;
    children?: React.ReactNode;
  }) =>
    React.createElement(
      React.Fragment,
      null,
      children ?? translateKey(i18nKey ?? '', ['common'], values),
    ),
}));

type MockMotionProps = {
  children?: React.ReactNode;
  layout?: unknown;
} & Record<string, unknown>;

function createMockMotionElement(tag: string) {
  return ({ children, layout: _layout, ...props }: MockMotionProps) =>
    React.createElement(tag, props, children);
}

// Mock motion/react to avoid animation issues in jsdom
vi.mock('motion/react', () => ({
  motion: {
    div: createMockMotionElement('div'),
    button: createMockMotionElement('button'),
    span: createMockMotionElement('span'),
    h1: createMockMotionElement('h1'),
    h2: createMockMotionElement('h2'),
    p: createMockMotionElement('p'),
    section: createMockMotionElement('section'),
    article: createMockMotionElement('article'),
    ul: createMockMotionElement('ul'),
    li: createMockMotionElement('li'),
  },
  AnimatePresence: ({ children }: { children: React.ReactNode }) => children,
  useAnimation: () => ({
    start: vi.fn(),
    stop: vi.fn(),
  }),
  Reorder: {
    Group: createMockMotionElement('div'),
    Item: createMockMotionElement('div'),
  },
}));
