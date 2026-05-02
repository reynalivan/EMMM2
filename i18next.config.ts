import { defineConfig } from 'i18next-cli';

export default defineConfig({
  locales: ['en', 'id', 'zh'],
  extract: {
    input: ['src/**/*.{ts,tsx}'],
    ignore: [
      'src/**/*.test.{ts,tsx}',
      'src/testing/**',
      'src/locales/**',
      'src-tauri/**',
      'test/**',
      'dist/**',
      'node_modules/**',
    ],
    output: 'src/locales/{{language}}/{{namespace}}.json',
    defaultNS: 'common',
    nsSeparator: ':',
    keySeparator: '.',
    functions: ['t', '*.t', 'i18next.t'],
    transComponents: ['Trans', 'Translation'],
    useTranslationNames: ['useTranslation'],
  },
});
