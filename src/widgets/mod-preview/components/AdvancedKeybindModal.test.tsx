import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import commonEn from '../../../shared/i18n/locales/en/common.json';
import commonId from '../../../shared/i18n/locales/id/common.json';
import commonZh from '../../../shared/i18n/locales/zh/common.json';
import { AdvancedKeybindModal } from './AdvancedKeybindModal';

const translationValues = vi.hoisted<Record<string, string>>(() => ({
  'common:actions.apply': 'Apply',
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => translationValues[key] ?? key,
  }),
}));

describe('AdvancedKeybindModal', () => {
  it('renders the localized Apply action instead of the raw translation key', () => {
    render(
      <AdvancedKeybindModal
        isOpen
        initialValue="no_modifiers p"
        onClose={vi.fn()}
        onApply={vi.fn()}
      />,
    );

    expect(screen.getByRole('button', { name: 'Apply' })).toBeInTheDocument();
    expect(screen.queryByText('common:actions.apply')).not.toBeInTheDocument();
  });

  it.each([
    ['English', commonEn.actions.apply, 'Apply'],
    ['Indonesian', commonId.actions.apply, 'Terapkan'],
    ['Chinese', commonZh.actions.apply, '应用'],
  ])('defines the Apply translation for %s', (_locale, value, expected) => {
    expect(value).toBe(expected);
  });
});
