import assert from 'node:assert/strict';
import { build } from 'vite';
import { pathToFileURL } from 'node:url';

export function selectMainBootstrapEntry(chunks) {
  const matches = chunks.filter(
    (chunk) => chunk.isEntry && /(?:^|[\\/])index\.html$/.test(chunk.facadeModuleId ?? ''),
  );
  assert.equal(matches.length, 1, 'Application entry index.html is missing or ambiguous');
  return matches[0];
}

export function assertHotkeyNoticeAsset(bundle) {
  assert.ok(
    Object.values(bundle).some(
      (entry) => entry.type === 'asset' && entry.fileName === 'hotkey-notice.html',
    ),
    'Bundled hotkey-notice.html is missing',
  );
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  process.env.NODE_ENV = 'production';
  let verified = false;
  await build({
    logLevel: 'warn',
    build: { write: false },
    plugins: [
      {
        name: 'verify-bootstrap-dependencies',
        enforce: 'post',
        generateBundle(_options, bundle) {
          assertHotkeyNoticeAsset(bundle);
          const chunks = new Map(
            Object.values(bundle)
              .filter((entry) => entry.type === 'chunk')
              .map((chunk) => [chunk.fileName, chunk]),
          );
          const entry = selectMainBootstrapEntry([...chunks.values()]);
          const visited = new Set();
          const visit = (name) => {
            if (visited.has(name)) return;
            visited.add(name);
            const chunk = chunks.get(name);
            assert.ok(chunk, `Missing static dependency: ${name}`);
            const eagerCharts = Object.keys(chunk.modules).filter((id) =>
              /node_modules[\\/]recharts[\\/]/.test(id),
            );
            assert.equal(
              eagerCharts.length,
              0,
              `Recharts must stay outside bootstrap: ${name} contains ${eagerCharts.length} modules`,
            );
            chunk.imports.forEach(visit);
          };
          visit(entry.fileName);
          assert.ok(
            [...chunks.values()].some((chunk) =>
              Object.keys(chunk.modules).some((id) => /node_modules[\\/]recharts[\\/]/.test(id)),
            ),
            'Charts must remain available to the lazily loaded Dashboard',
          );
          verified = true;
          console.info(`Bootstrap dependency guard passed: ${visited.size} static chunks`);
        },
      },
    ],
  });

  assert.ok(verified, 'Bundle dependency guard did not run');
}
