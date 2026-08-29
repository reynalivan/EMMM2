import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import LanguageDetector from 'i18next-browser-languagedetector';

// Import resources
import commonEn from '../../shared/i18n/locales/en/common.json';
import welcomeEn from '../../shared/i18n/locales/en/welcome.json';
import onboardingEn from '../../shared/i18n/locales/en/onboarding.json';
import dashboardEn from '../../shared/i18n/locales/en/dashboard.json';
import gridEn from '../../shared/i18n/locales/en/grid.json';
import objectsEn from '../../shared/i18n/locales/en/objects.json';
import browserEn from '../../shared/i18n/locales/en/browser.json';
import scannerEn from '../../shared/i18n/locales/en/scanner.json';
import collectionsEn from '../../shared/i18n/locales/en/collections.json';
import settingsEn from '../../shared/i18n/locales/en/settings.json';
import folderGridEn from '../../shared/i18n/locales/en/folder_grid.json';
import previewEn from '../../shared/i18n/locales/en/preview.json';
import layoutEn from '../../shared/i18n/locales/en/layout.json';
import matchWizardEn from '../../shared/i18n/locales/en/match_wizard.json';
import modInboxEn from '../../shared/i18n/locales/en/mod_inbox.json';

import commonId from '../../shared/i18n/locales/id/common.json';
import welcomeId from '../../shared/i18n/locales/id/welcome.json';
import onboardingId from '../../shared/i18n/locales/id/onboarding.json';
import dashboardId from '../../shared/i18n/locales/id/dashboard.json';
import gridId from '../../shared/i18n/locales/id/grid.json';
import objectsId from '../../shared/i18n/locales/id/objects.json';
import browserId from '../../shared/i18n/locales/id/browser.json';
import scannerId from '../../shared/i18n/locales/id/scanner.json';
import collectionsId from '../../shared/i18n/locales/id/collections.json';
import settingsId from '../../shared/i18n/locales/id/settings.json';
import folderGridId from '../../shared/i18n/locales/id/folder_grid.json';
import previewId from '../../shared/i18n/locales/id/preview.json';
import layoutId from '../../shared/i18n/locales/id/layout.json';
import matchWizardId from '../../shared/i18n/locales/id/match_wizard.json';
import modInboxId from '../../shared/i18n/locales/id/mod_inbox.json';

import commonZh from '../../shared/i18n/locales/zh/common.json';
import welcomeZh from '../../shared/i18n/locales/zh/welcome.json';
import onboardingZh from '../../shared/i18n/locales/zh/onboarding.json';
import dashboardZh from '../../shared/i18n/locales/zh/dashboard.json';
import gridZh from '../../shared/i18n/locales/zh/grid.json';
import objectsZh from '../../shared/i18n/locales/zh/objects.json';
import browserZh from '../../shared/i18n/locales/zh/browser.json';
import scannerZh from '../../shared/i18n/locales/zh/scanner.json';
import collectionsZh from '../../shared/i18n/locales/zh/collections.json';
import settingsZh from '../../shared/i18n/locales/zh/settings.json';
import folderGridZh from '../../shared/i18n/locales/zh/folder_grid.json';
import previewZh from '../../shared/i18n/locales/zh/preview.json';
import layoutZh from '../../shared/i18n/locales/zh/layout.json';
import matchWizardZh from '../../shared/i18n/locales/zh/match_wizard.json';
import modInboxZh from '../../shared/i18n/locales/zh/mod_inbox.json';

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
