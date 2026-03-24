# Fix LightningCSS @property Warning

### Context

DaisyUI 5 uses `@property` for components like `radial-progress`. When these are nested within `@layer` rules, older versions of LightningCSS (pre-1.31.0) fail to parse them, resulting in "Unknown at rule: @property" warnings and broken CSS optimization.

### Changes

- Forced `lightningcss` update to `^1.32.0` via `pnpm.overrides` to support nested `@property`.
- Simplified `vite.config.ts` by removing redundant CSS transformer configuration.
- Added `vitest` types to `tsconfig.node.json` to resolve `defineConfig` import error in the IDE.
- Refined `manualChunks` logic in `vite.config.ts` to resolve circular dependencies between `vendor-core` and `vendor-utils`.

### Impacted Files

- `package.json` (modified)
- `vite.config.ts` (modified)
- `tsconfig.node.json` (modified)
- `pnpm-lock.yaml` (modified via install)

### Goal

Ensure clean CSS transformation and optimization for modern CSS rules used by DaisyUI 5.

### Impact

- Correct rendering of CSS Properties and Values API components.
- No build warnings during CSS optimization.
- Modern CSS features enabled for better performance in WebView2.
