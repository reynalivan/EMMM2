import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import LanguageDetector from 'i18next-browser-languagedetector';

// Import resources
import commonEn from '../locales/en/common.json';
import welcomeEn from '../locales/en/welcome.json';
import onboardingEn from '../locales/en/onboarding.json';
import dashboardEn from '../locales/en/dashboard.json';
import gridEn from '../locales/en/grid.json';
import objectsEn from '../locales/en/objects.json';
import browserEn from '../locales/en/browser.json';
import scannerEn from '../locales/en/scanner.json';
import collectionsEn from '../locales/en/collections.json';
import settingsEn from '../locales/en/settings.json';
import folderGridEn from '../locales/en/folder_grid.json';
import previewEn from '../locales/en/preview.json';
import layoutEn from '../locales/en/layout.json';
import matchWizardEn from '../locales/en/match_wizard.json';
import modInboxEn from '../locales/en/mod_inbox.json';

import commonId from '../locales/id/common.json';
import welcomeId from '../locales/id/welcome.json';
import onboardingId from '../locales/id/onboarding.json';
import dashboardId from '../locales/id/dashboard.json';
import gridId from '../locales/id/grid.json';
import objectsId from '../locales/id/objects.json';
import browserId from '../locales/id/browser.json';
import scannerId from '../locales/id/scanner.json';
import collectionsId from '../locales/id/collections.json';
import settingsId from '../locales/id/settings.json';
import folderGridId from '../locales/id/folder_grid.json';
import previewId from '../locales/id/preview.json';
import layoutId from '../locales/id/layout.json';
import matchWizardId from '../locales/id/match_wizard.json';
import modInboxId from '../locales/id/mod_inbox.json';

import commonZh from '../locales/zh/common.json';
import welcomeZh from '../locales/zh/welcome.json';
import onboardingZh from '../locales/zh/onboarding.json';
import dashboardZh from '../locales/zh/dashboard.json';
import gridZh from '../locales/zh/grid.json';
import objectsZh from '../locales/zh/objects.json';
import browserZh from '../locales/zh/browser.json';
import scannerZh from '../locales/zh/scanner.json';
import collectionsZh from '../locales/zh/collections.json';
import settingsZh from '../locales/zh/settings.json';
import folderGridZh from '../locales/zh/folder_grid.json';
import previewZh from '../locales/zh/preview.json';
import layoutZh from '../locales/zh/layout.json';
import matchWizardZh from '../locales/zh/match_wizard.json';
import modInboxZh from '../locales/zh/mod_inbox.json';

const resources = {
  en: {
    common: commonEn,
    welcome: welcomeEn,
    onboarding: onboardingEn,
    dashboard: dashboardEn,
    grid: gridEn,
    objects: objectsEn,
    browser: browserEn,
    scanner: scannerEn,
    collections: collectionsEn,
    settings: settingsEn,
    folder_grid: folderGridEn,
    preview: previewEn,
    layout: layoutEn,
    match_wizard: matchWizardEn,
    mod_inbox: modInboxEn,
  },
  id: {
    common: commonId,
    welcome: welcomeId,
    onboarding: onboardingId,
    dashboard: dashboardId,
    grid: gridId,
    objects: objectsId,
    browser: browserId,
    scanner: scannerId,
    collections: collectionsId,
    settings: settingsId,
    folder_grid: folderGridId,
    preview: previewId,
    layout: layoutId,
    match_wizard: matchWizardId,
    mod_inbox: modInboxId,
  },
  zh: {
    common: commonZh,
    welcome: welcomeZh,
    onboarding: onboardingZh,
    dashboard: dashboardZh,
    grid: gridZh,
    objects: objectsZh,
    browser: browserZh,
    scanner: scannerZh,
    collections: collectionsZh,
    settings: settingsZh,
    folder_grid: folderGridZh,
    preview: previewZh,
    layout: layoutZh,
    match_wizard: matchWizardZh,
    mod_inbox: modInboxZh,
  },
};

i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources,
    fallbackLng: 'en',
    debug: false,
    interpolation: {
      escapeValue: false,
    },
    ns: [
      'common',
      'welcome',
      'onboarding',
      'layout',
      'dashboard',
      'grid',
      'objects',
      'browser',
      'scanner',
      'collections',
      'settings',
      'folder_grid',
      'preview',
      'match_wizard',
      'mod_inbox',
    ],
    defaultNS: 'common',
  });

export const namespaces = [
  'common',
  'welcome',
  'onboarding',
  'layout',
  'dashboard',
  'grid',
  'objects',
  'browser',
  'scanner',
  'collections',
  'settings',
  'folder_grid',
  'preview',
  'match_wizard',
  'mod_inbox',
] as const;

export default i18n;
