import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import FolderGridBanners from './FolderGridBanners';

vi.mock('@/app/store', () => ({
  useAppStore: (selector: (state: unknown) => unknown) =>
    selector({
      activeGameId: 'game',
      folderConflictsByGame: {},
      renameConfirmationsByGame: {},
    }),
}));

vi.mock('react-i18next', async (importOriginal) => {
  const actual = await importOriginal<typeof import('react-i18next')>();
  return {
    ...actual,
    useTranslation: () => ({ t: (key: string) => key }),
  };
});

vi.mock('@/features/workspace-runtime', () => ({
  openFolderConflictManagerDialog: vi.fn(),
  openRenameConfirmationDialog: vi.fn(),
}));

describe('FolderGridBanners', () => {
  it('keeps the disabled-parent notice below the toolbar instead of making it sticky', () => {
    render(
      <FolderGridBanners
        isLoading={false}
        isError={false}
        isFlatModRoot={false}
        selfIsEnabled={true}
        selfReasons={[]}
        isMobile={false}
        isPreviewOpen={false}
        currentPath={['Group', 'Child']}
        setMobilePane={vi.fn()}
        togglePreview={vi.fn()}
        handleToggleSelf={vi.fn()}
        ancestorDisabledBy="Group"
        onOpenEnableParentDialog={vi.fn()}
        diskSourceUnavailableMessage={null}
        mutationsDisabled={false}
      />,
    );

    const notices = screen.getByTestId('folder-grid-notices');
    expect(notices).toHaveClass('relative', 'z-20');
    expect(notices).not.toHaveClass('sticky');
  });
});
