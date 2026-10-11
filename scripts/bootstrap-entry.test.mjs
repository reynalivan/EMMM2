import assert from 'node:assert/strict';
import { test } from 'node:test';
import { assertHotkeyNoticeAsset, selectMainBootstrapEntry } from './check-bootstrap-bundle.mjs';

test('selects index.html even when an unrelated notification entry appears first', () => {
  const notice = {
    isEntry: true,
    facadeModuleId: 'E:\\repo\\hotkey-notice.html',
    fileName: 'notice.js',
  };
  const main = { isEntry: true, facadeModuleId: 'E:\\repo\\index.html', fileName: 'main.js' };
  assert.equal(selectMainBootstrapEntry([notice, main]), main);
});

test('fails closed when the main entry is absent or ambiguous', () => {
  assert.throws(
    () => selectMainBootstrapEntry([{ isEntry: true, facadeModuleId: '/repo/hotkey-notice.html' }]),
    /Application entry/,
  );
  assert.throws(
    () =>
      selectMainBootstrapEntry([
        { isEntry: true, facadeModuleId: '/repo/index.html' },
        { isEntry: true, facadeModuleId: '/other/index.html' },
      ]),
    /Application entry/,
  );
});

test('requires the actually emitted notification HTML, not just a synthetic chunk', () => {
  assert.throws(
    () => assertHotkeyNoticeAsset({ main: { type: 'asset', fileName: 'index.html' } }),
    /hotkey-notice.html is missing/,
  );
  assert.doesNotThrow(() =>
    assertHotkeyNoticeAsset({ notice: { type: 'asset', fileName: 'hotkey-notice.html' } }),
  );
});
