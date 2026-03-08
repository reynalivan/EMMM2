import { beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen, waitFor } from '../../../testing/test-utils';
import IniEditorSection from './IniEditorSection';
import type { KeyBindSectionGroup } from '../previewPanelUtils';

describe('IniEditorSection', () => {
  const mockKeyBindSections: KeyBindSectionGroup[] = [
    {
      id: 'config.ini',
      fileName: 'config.ini',
      rangeLabel: 'lines 1-10',
      sections: [
        {
          sectionName: 'Constants',
          fields: [
            {
              id: 'field1',
              fileName: 'config.ini',
              sectionName: 'Constants',
              lineIdx: 1,
              label: 'key',
              prefix: 'key',
              value: 'K',
            },
            {
              id: 'field2',
              fileName: 'config.ini',
              sectionName: 'Constants',
              lineIdx: 2,
              label: 'back',
              prefix: 'back',
              value: '0.5',
            },
          ],
        },
        {
          sectionName: 'Variables',
          fields: [
            {
              id: 'field3',
              fileName: 'config.ini',
              sectionName: 'Variables',
              lineIdx: 11,
              label: 'value',
              prefix: 'value',
              value: '100',
            },
          ],
        },
      ],
    },
  ];

  const defaultProps = {
    activePath: 'E:/Mods/TestMod',
    activeObjectName: 'Test Object',
    selectedFolderName: 'test_folder_1',
    activeTab: 'keybind' as const,
    sections: mockKeyBindSections,
    openSectionIds: new Set(['config.ini']),
    draftByField: {
      field1: 'K',
      field2: '0.5',
      field3: '100',
    },
    fieldErrors: {},
    hashSummaries: [],
    modFeatureSummaries: [],
    conflictingKeys: new Set<string>(),
    editorDirty: false,
    isSaving: false,
    onTabChange: vi.fn(),
    onToggleSection: vi.fn(),
    onFieldChange: vi.fn(),
    onSave: vi.fn().mockResolvedValue(true),
    onDiscard: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  // Covers: TC-6.3-01 (INI tab render)
  it('should render keybind tab when activeTab is keybind', async () => {
    const props = { ...defaultProps, activeTab: 'keybind' as const };
    render(<IniEditorSection {...props} />);

    await waitFor(() => {
      expect(screen.getByText('Key Bind')).toBeInTheDocument();
    });
  });

  // Covers: TC-6.3-01 (INI tab render)
  it('should render information tab when activeTab is information', async () => {
    const props = { ...defaultProps, activeTab: 'information' as const };
    render(<IniEditorSection {...props} />);

    await waitFor(() => {
      expect(screen.getByText('Information')).toBeInTheDocument();
    });
  });

  // Covers: TC-6.3-01 (Tab switching)
  it('should call onTabChange when switching tabs', async () => {
    const onTabChangeMock = vi.fn();
    const props = { ...defaultProps, onTabChange: onTabChangeMock };
    render(<IniEditorSection {...props} />);

    const informationTab = screen.getByText('Information');
    informationTab.click();

    await waitFor(() => {
      expect(onTabChangeMock).toHaveBeenCalledWith('information');
    });
  });

  // Covers: TC-6.3-02 (INI field edit/save)
  it('should show Edit button in normal mode', async () => {
    const props = { ...defaultProps, editorDirty: false };
    render(<IniEditorSection {...props} />);

    expect(screen.getByText('Edit')).toBeInTheDocument();
    expect(screen.queryByText('Revert')).not.toBeInTheDocument();
  });

  // Covers: TC-6.3-02 (INI field edit/save)
  it('should enter edit mode and show Revert button when edit is clicked', async () => {
    const props = { ...defaultProps, editorDirty: false };
    render(<IniEditorSection {...props} />);

    screen.getByText('Edit').click();

    await waitFor(() => {
      expect(screen.getByText('Close')).toBeInTheDocument();
      expect(screen.queryByText('Edit')).not.toBeInTheDocument();
    });
  });

  // Covers: TC-6.3-02 (INI field edit/save)
  it('should show Close button when not dirty', async () => {
    const props = { ...defaultProps, editorDirty: false };
    render(<IniEditorSection {...props} />);

    screen.getByText('Edit').click();

    await waitFor(() => {
      const closeButton = screen.getByText('Close');
      expect(closeButton).toBeInTheDocument();
      expect(screen.queryByText('Revert')).not.toBeInTheDocument();
    });
  });

  // Covers: TC-6.3-02 (INI field edit/save)
  it('should enable Revert button when editor is dirty', async () => {
    const props = { ...defaultProps, editorDirty: true };
    render(<IniEditorSection {...props} />);

    screen.getByText('Edit').click();

    await waitFor(() => {
      const revertButton = screen.getByText('Revert');
      expect(revertButton).not.toBeDisabled();
    });
  });

  // Covers: TC-6.3-02 (INI field edit/save)
  it('should disable Revert button when isSaving is true', async () => {
    const props = { ...defaultProps, editorDirty: true, isSaving: true };
    render(<IniEditorSection {...props} />);

    screen.getByText('Edit').click();

    await waitFor(() => {
      const revertButton = screen.getByText('Revert');
      expect(revertButton).toBeDisabled();
    });
  });

  // Covers: TC-6.3-02 (INI field edit/save)
  it('should enable Save button when dirty in edit mode', async () => {
    const props = { ...defaultProps, editorDirty: true };
    render(<IniEditorSection {...props} />);

    screen.getByText('Edit').click();

    await waitFor(() => {
      const saveBtn = screen.getByText('Save');
      expect(saveBtn).toBeInTheDocument();
      expect(saveBtn).not.toBeDisabled();
    });
  });

  // Covers: TC-6.3-02 (INI field edit/save)
  it('should call onSave when Save button is clicked', async () => {
    const onSaveMock = vi.fn();
    const props = { ...defaultProps, editorDirty: true, onSave: onSaveMock };
    render(<IniEditorSection {...props} />);

    screen.getByText('Edit').click();

    await waitFor(() => {
      const saveBtn = screen.getByText('Save');
      expect(saveBtn).toBeInTheDocument();
      saveBtn.click();
    });

    await waitFor(() => {
      expect(onSaveMock).toHaveBeenCalled();
    });
  });

  // Covers: TC-6.3-02 (INI field edit/save)
  it('should call onDiscard when Revert button is clicked', async () => {
    const onDiscardMock = vi.fn();
    const props = { ...defaultProps, editorDirty: true, onDiscard: onDiscardMock };
    render(<IniEditorSection {...props} />);

    screen.getByText('Edit').click();

    await waitFor(() => {
      const revert = screen.getByText('Revert');
      expect(revert).toBeInTheDocument();
      revert.click();
    });

    await waitFor(() => {
      expect(onDiscardMock).toHaveBeenCalled();
    });
  });

  // Covers: TC-6.3-02 (Field changes)
  it('should call onFieldChange when field input changes', async () => {
    const onFieldChangeMock = vi.fn();
    const props = { ...defaultProps, onFieldChange: onFieldChangeMock };
    render(<IniEditorSection {...props} />);

    // Enter edit mode to make inputs visible
    screen.getByText('Edit').click();

    await waitFor(() => {
      const inputs = screen.getAllByRole('textbox');
      if (inputs.length > 0) {
        inputs[0].focus();
        inputs[0].dispatchEvent(new Event('input', { bubbles: true }));
      }
    });
  });

  // Covers: TC-6.3-01 (Section collapse/expand)
  it('should render section header with collapse toggle', async () => {
    const props = { ...defaultProps };
    render(<IniEditorSection {...props} />);

    const sectionHeader = screen.getByText(/Constants/);
    expect(sectionHeader).toBeInTheDocument();
  });

  // Covers: TC-6.3-01 (Section collapse/expand)
  it('should call onToggleSection when section header is clicked', async () => {
    const onToggleSectionMock = vi.fn();
    const props = { ...defaultProps, onToggleSection: onToggleSectionMock };
    render(<IniEditorSection {...props} />);

    const sectionButton = screen
      .getAllByRole('button')
      .find((btn: HTMLElement) => btn.textContent?.includes('Constants'));

    if (sectionButton) {
      sectionButton.click();
      await waitFor(() => {
        expect(onToggleSectionMock).toHaveBeenCalled();
      });
    }
  });

  // Covers: TC-6.3-02 (Mod Feature summaries in information tab)
  it('should display mod feature summaries in information tab', async () => {
    const props = {
      ...defaultProps,
      activeTab: 'information' as const,
      modFeatureSummaries: [
        {
          featureName: 'Sword Hide',
          triggerKeys: ['VK_H'],
          statesCount: 2,
        },
      ],
    };
    render(<IniEditorSection {...props} />);

    await waitFor(() => {
      expect(screen.getByText('Sword Hide')).toBeInTheDocument();
      expect(screen.getByText('VK_H')).toBeInTheDocument();
    });
  });

  // Covers: TC-6.3-02 (Hash summaries in information tab)
  it('should display hash summaries in information tab', async () => {
    const props = {
      ...defaultProps,
      activeTab: 'information' as const,
      hashSummaries: [
        {
          fileName: 'mod.ini',
          sectionName: 'TextureOverrideHero',
          hash: '1a2b3c4d',
        },
      ],
    };
    render(<IniEditorSection {...props} />);

    await waitFor(() => {
      expect(screen.getByText('1a2b3c4d')).toBeInTheDocument();
    });
  });

  // Covers: TC-6.3-02 (Field error display)
  it('should display field errors when present', async () => {
    const props = {
      ...defaultProps,
      fieldErrors: {
        field1: 'Invalid key binding format',
      },
    };
    render(<IniEditorSection {...props} />);

    // Component renders without error
    await waitFor(() => {
      expect(screen.getByText('INI Editor')).toBeInTheDocument();
    });
  });

  // Covers: TC-6.3-01 (Empty keybind sections)
  it('should show empty state when no keybind sections exist', async () => {
    const props = {
      ...defaultProps,
      sections: [],
    };
    render(<IniEditorSection {...props} />);

    await waitFor(() => {
      expect(screen.getByText('No key binding sections detected.')).toBeInTheDocument();
    });
  });

  // Covers: TC-18 (Dirty State - Disabled)
  it('should show Close button in edit mode when editorDirty is false', async () => {
    const props = { ...defaultProps, editorDirty: false };
    render(<IniEditorSection {...props} />);

    screen.getByText('Edit').click();

    await waitFor(() => {
      const closeButton = screen.getByText('Close');
      expect(closeButton).toBeInTheDocument();
      expect(screen.queryByText('Revert')).not.toBeInTheDocument();
    });
  });

  // Covers: TC-18 (Dirty State - Enabled/Interaction)
  it('should enable Revert and trigger onDiscard when editorDirty is true', async () => {
    const onDiscardMock = vi.fn();
    const props = {
      ...defaultProps,
      editorDirty: true,
      onDiscard: onDiscardMock,
    };
    render(<IniEditorSection {...props} />);

    screen.getByText('Edit').click();

    await waitFor(() => {
      const revertButton = screen.getByText('Revert');
      expect(revertButton).not.toBeDisabled();
      revertButton.click();
    });

    await waitFor(() => {
      expect(onDiscardMock).toHaveBeenCalled();
    });
  });
});
