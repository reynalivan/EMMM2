import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import DuplicateWarningModal from './DuplicateWarningModal';

// Mock dialog behavior
beforeEach(() => {
  HTMLDialogElement.prototype.showModal = vi.fn();
  HTMLDialogElement.prototype.close = vi.fn();
});

describe('DuplicateWarningModal (TC-29 Conflict Detection)', () => {
  const mockDuplicates = [
    { mod_id: '1', folder_path: 'C:/Mods/A', actual_name: 'Mod A' },
    { mod_id: '2', folder_path: 'C:/Mods/B', actual_name: 'Mod B' },
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
    expect(screen.getByText('New Target Mod')).toBeInTheDocument();

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
    expect(onEnableOnlyThisMock).not.toHaveBeenCalled();
  });
});
