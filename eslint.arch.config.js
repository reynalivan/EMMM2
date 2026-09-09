import path from 'node:path';
import tseslint from 'typescript-eslint';
import reactHooks from 'eslint-plugin-react-hooks';
import reactRefresh from 'eslint-plugin-react-refresh';

const LAYERS = ['shared', 'entities', 'features', 'widgets', 'pages', 'app'];
const SLICED_LAYERS = new Set(['entities', 'features', 'widgets', 'pages']);
const FORBIDDEN_CAPABILITIES = new Set([
  '@tauri-apps/plugin-fs',
  '@tauri-apps/plugin-process',
  '@tauri-apps/plugin-updater',
]);

function sourceLocation(filename) {
  const relative = path.relative(process.cwd(), filename).replaceAll('\\', '/');
  const parts = relative.split('/');
  if (parts[0] !== 'src' || !LAYERS.includes(parts[1])) return null;
  return { layer: parts[1], slice: parts[2] };
}

function targetLocation(specifier, filename) {
  let normalized = specifier;
  if (specifier.startsWith('.')) {
    const absolute = path.resolve(path.dirname(filename), specifier);
    const relative = path.relative(process.cwd(), absolute).replaceAll('\\', '/');
    if (!relative.startsWith('src/')) return null;
    normalized = `@/${relative.slice(4)}`;
  }
  if (!normalized.startsWith('@/')) return null;
  const parts = normalized.slice(2).split('/');
  if (!LAYERS.includes(parts[0])) return null;
  return {
    layer: parts[0],
    slice: parts[1],
    depth: parts.length,
    crossConsumer: parts[2] === '@x' ? parts[3] : null,
  };
}

const boundariesRule = {
  meta: {
    type: 'problem',
    schema: [],
    messages: {
      capability: 'Frontend capability "{{specifier}}" is owned by Rust commands.',
      upward: '{{source}} must not import upward from {{target}}.',
      crossSlice: '{{layer}} slices must not import each other directly.',
      deepImport: 'Import {{target}} through its public API: {{publicApi}}.',
    },
  },
  create(context) {
    function check(node) {
      const specifier = node.source?.value;
      if (typeof specifier !== 'string') return;
      const violation = evaluateBoundary(context.filename, specifier);
      if (violation) context.report({ node, ...violation });
    }

    return {
      ImportDeclaration: check,
      ExportAllDeclaration: check,
      ExportNamedDeclaration: check,
    };
  },
};

export function evaluateBoundary(filename, specifier) {
  if (FORBIDDEN_CAPABILITIES.has(specifier)) {
    return { messageId: 'capability', data: { specifier } };
  }
  const source = sourceLocation(filename);
  const target = targetLocation(specifier, filename);
  if (!source || !target) return null;
  if (target.layer === 'app' && specifier === '@/app/store') return null;
  if (LAYERS.indexOf(target.layer) > LAYERS.indexOf(source.layer)) {
    return {
      messageId: 'upward',
      data: { source: source.layer, target: target.layer },
    };
  }

  const sameSlice = source.layer === target.layer && source.slice === target.slice;
  if (
    source.layer === target.layer &&
    target.crossConsumer === source.slice &&
    target.depth === 4
  ) {
    return null;
  }
  if (target.crossConsumer === source.layer && target.depth === 4) return null;
  if (source.layer === target.layer && SLICED_LAYERS.has(source.layer) && !sameSlice) {
    return { messageId: 'crossSlice', data: { layer: source.layer } };
  }
  if (!sameSlice && SLICED_LAYERS.has(target.layer) && target.depth > 2) {
    return {
      messageId: 'deepImport',
      data: { target: specifier, publicApi: `@/${target.layer}/${target.slice}` },
    };
  }
  return null;
}

export default tseslint.config({
  files: ['src/**/*.{ts,tsx}'],
  ignores: ['src/shared/api/tauri/bindings.gen.ts'],
  linterOptions: { reportUnusedDisableDirectives: false },
  languageOptions: { parser: tseslint.parser },
  plugins: {
    architecture: { rules: { boundaries: boundariesRule } },
    '@typescript-eslint': tseslint.plugin,
    'react-hooks': reactHooks,
    'react-refresh': reactRefresh,
  },
  rules: { 'architecture/boundaries': 'error' },
});
