import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import DuplicateWarningModal from './DuplicateWarningModal';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, vars?: Record<string, unknown>) => {
      if (key === 'duplicate_warning.title') {
        return 'Duplicate Character Active';
      }
      if (key === 'duplicate_warning.description') {
        return `Duplicate warning for ${String(vars?.targetName ?? '')}`;
      }
      if (key === 'duplicate_warning.currently_enabled') {
        return 'Currently Enabled';
      }
      if (key === 'duplicate_warning.enable_only_this') {
        return 'Enable Only This';
      }
      if (key === 'duplicate_warning.force_enable') {
        return 'Force Enable';
      }
      if (key === 'duplicate_warning.dont_warn_again') {
        return "Don't warn again for this combination";
      }
      if (key === 'common:actions.cancel') {
        return 'Cancel';
      }
      if (key === 'common:actions.close') {
        return 'Close';
      }

      return key;
    },
  }),
}));

// Mock dialog behavior
beforeEach(() => {
  HTMLDialogElement.prototype.showModal = vi.fn();
  HTMLDialogElement.prototype.close = vi.fn();
});

describe('DuplicateWarningModal (TC-29 Conflict Detection)', () => {
  const mockDuplicates = [
    {
      mod_id: '1',
      object_id: 'object-1',
      folder_path: 'C:/Mods/A',
      actual_name: 'Mod A',
      is_variant: false,
      parent_path: 'C:/Mods',
    },
    {
      mod_id: '2',
      object_id: 'object-1',
      folder_path: 'C:/Mods/B',
      actual_name: 'Mod B',
      is_variant: false,
      parent_path: 'C:/Mods',
    },
  ];

  const onForceEnableMock = vi.fn();
  const onEnableOnlyThisMock = vi.fn();
  const onCancelMock = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  // TC-29-002: Show warning modal instead of enabling
  it('TC-29-002: Renders warning modal with duplicate info', () => {
    render(
      <DuplicateWarningModal
        open={true}
        targetName="New Target Mod"
        duplicates={mockDuplicates}
        onForceEnable={onForceEnableMock}
        onEnableOnlyThis={onEnableOnlyThisMock}
        onCancel={onCancelMock}
      />,
    );

    // Verify Title indicating conflict
    expect(screen.getByText('Duplicate Character Active')).toBeInTheDocument();

    // Verify target name is bolded in text
    expect(screen.getByText(/New Target Mod/)).toBeInTheDocument();

    // Verify conflicting mods are listed
    expect(screen.getByText('Mod A')).toBeInTheDocument();
    expect(screen.getByText('Mod B')).toBeInTheDocument();
  });

  // TC-29-003: Resolve duplicate using 'Enable ONLY This'
  it('TC-29-003: Calls onEnableOnlyThis when resolving conflict optimally', async () => {
    render(
      <DuplicateWarningModal
        open={true}
        targetName="New Target Mod"
        duplicates={mockDuplicates}
        onForceEnable={onForceEnableMock}
        onEnableOnlyThis={onEnableOnlyThisMock}
        onCancel={onCancelMock}
      />,
    );

    const enableOnlyBtn = screen.getByText(/Enable Only This/i);
    fireEvent.click(enableOnlyBtn);

    expect(onEnableOnlyThisMock).toHaveBeenCalledTimes(1);
    expect(onForceEnableMock).not.toHaveBeenCalled();
    expect(onCancelMock).not.toHaveBeenCalled();
  });

  // TC-29-Extra: Resolve using 'Force Enable'
  it('TC-29: Calls onForceEnable when bypassing warning', async () => {
    render(
      <DuplicateWarningModal
        open={true}
        targetName="New Target Mod"
        duplicates={mockDuplicates}
        onForceEnable={onForceEnableMock}
        onEnableOnlyThis={onEnableOnlyThisMock}
        onCancel={onCancelMock}
      />,
    );

    const forceEnableBtn = screen.getByText(/Force Enable/i);
    fireEvent.click(forceEnableBtn);

    expect(onForceEnableMock).toHaveBeenCalledTimes(1);
    expect(onForceEnableMock).toHaveBeenCalledWith(false);
    expect(onEnableOnlyThisMock).not.toHaveBeenCalled();
  });

  // TC-29-004: Ignore future warnings for this combination
  it('TC-29-004: Passes ignoreFuture=true when the checkbox is ticked before Force Enable', () => {
    render(
      <DuplicateWarningModal
        open={true}
        targetName="New Target Mod"
        duplicates={mockDuplicates}
        onForceEnable={onForceEnableMock}
        onEnableOnlyThis={onEnableOnlyThisMock}
        onCancel={onCancelMock}
      />,
    );

    fireEvent.click(screen.getByRole('checkbox', { hidden: true }));
    fireEvent.click(screen.getByText(/Force Enable/i));

    expect(onForceEnableMock).toHaveBeenCalledTimes(1);
    expect(onForceEnableMock).toHaveBeenCalledWith(true);
  });
});
