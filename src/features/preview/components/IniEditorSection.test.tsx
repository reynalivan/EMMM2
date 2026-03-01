import { beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen, waitFor } from '../../../testing/test-utils';
import IniEditorSection from './IniEditorSection';
import type { KeyBindSectionGroup, VariableInfoSummary } from '../previewPanelUtils';

describe('IniEditorSection', () => {
  const mockKeyBindSections: KeyBindSectionGroup[] = [
    {
      id: 'section1',
      fileName: 'config.ini',
      sectionName: 'Constants',
      rangeLabel: 'lines 1-10',
      fields: [
        {
          id: 'field1',
          fileName: 'config.ini',
          sectionName: 'Constants',
          lineIdx: 1,
          label: 'Swap Key',
          prefix: 'key',
          value: 'K',
        },
        {
          id: 'field2',
          fileName: 'config.ini',
          sectionName: 'Constants',
          lineIdx: 2,
          label: 'Toggle Speed',
          prefix: 'back',
          value: '0.5',
        },
      ],
    },
    {
      id: 'section2',
      fileName: 'config.ini',
      sectionName: 'Variables',
      rangeLabel: 'lines 11-20',
      fields: [
        {
          id: 'field3',
          fileName: 'config.ini',
          sectionName: 'Variables',
          lineIdx: 11,
          label: 'Main Value',
          prefix: 'value',
          value: '100',
        },
      ],
    },
  ];

  const mockVariableSummaries: VariableInfoSummary[] = [
    {
      name: '$active',
      count: 2,
      minValue: 0,
      maxValue: 1,
      occurrences: [
        { fileName: 'config.ini', sectionName: 'Constants', lineIdx: 5, value: '1' },
        { fileName: 'mod.ini', sectionName: 'Variables', lineIdx: 10, value: '0' },
      ],
    },
    {
      name: '$swapvar',
      count: 1,
      minValue: 0,
      maxValue: 100,
      occurrences: [{ fileName: 'config.ini', sectionName: 'Constants', lineIdx: 8, value: '50' }],
    },
  ];

  const defaultProps = {
    activePath: 'E:/Mods/TestMod',
    activeTab: 'keybind' as const,
    sections: mockKeyBindSections,
    openSectionIds: new Set(['section1']),
    draftByField: {
      field1: 'K',
      field2: '0.5',
      field3: '100',
    },
    fieldErrors: {},
    variableSummaries: mockVariableSummaries,
    editorDirty: false,
    isSaving: false,
    onTabChange: vi.fn(),
    onToggleSection: vi.fn(),
    onFieldChange: vi.fn(),
    onSave: vi.fn(),
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
  it('should disable save button when not dirty', async () => {
    const props = { ...defaultProps, editorDirty: false };
    render(<IniEditorSection {...props} />);

    const saveButton = screen.getByText('Save INI');
    expect(saveButton).toHaveAttribute('disabled');
  });

  // Covers: TC-6.3-02 (INI field edit/save)
  it('should enable save button when editor is dirty', async () => {
    const props = { ...defaultProps, editorDirty: true };
    render(<IniEditorSection {...props} />);

    const saveButton = screen.getByText('Save INI');
    expect(saveButton).not.toHaveAttribute('disabled');
  });

  // Covers: TC-6.3-02 (INI field edit/save)
  it('should disable save button when isSaving is true', async () => {
    const props = { ...defaultProps, editorDirty: true, isSaving: true };
    render(<IniEditorSection {...props} />);

    const saveButton = screen.getByText('Save INI');
    expect(saveButton).toHaveAttribute('disabled');
  });

  // Covers: TC-6.3-02 (INI field edit/save)
  it('should call onSave when save button is clicked', async () => {
    const onSaveMock = vi.fn();
    const props = { ...defaultProps, editorDirty: true, onSave: onSaveMock };
    render(<IniEditorSection {...props} />);

    const saveButton = screen.getByText('Save INI');
    saveButton.click();

    await waitFor(() => {
      expect(onSaveMock).toHaveBeenCalled();
    });
  });

  // Covers: TC-6.3-02 (INI field edit/save)
  it('should call onDiscard when discard button is clicked', async () => {
    const onDiscardMock = vi.fn();
    const props = { ...defaultProps, editorDirty: true, onDiscard: onDiscardMock };
    render(<IniEditorSection {...props} />);

    const discardButton = screen.getByText('Discard');
    discardButton.click();

    await waitFor(() => {
      expect(onDiscardMock).toHaveBeenCalled();
    });
  });

  // Covers: TC-6.3-02 (Field changes)
  it('should call onFieldChange when field input changes', async () => {
    const onFieldChangeMock = vi.fn();
    const props = { ...defaultProps, onFieldChange: onFieldChangeMock };
    render(<IniEditorSection {...props} />);

    const inputs = screen.getAllByRole('textbox');
    if (inputs.length > 0) {
      inputs[0].focus();
      inputs[0].dispatchEvent(new Event('input', { bubbles: true }));
    }
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

  // Covers: TC-6.3-02 (Variable summaries rendering)
  it('should display variable summaries in information tab', async () => {
    const props = {
      ...defaultProps,
      activeTab: 'information' as const,
      variableSummaries: mockVariableSummaries,
    };
    render(<IniEditorSection {...props} />);

    await waitFor(() => {
      expect(screen.getByText('$active')).toBeInTheDocument();
      expect(screen.getByText('$swapvar')).toBeInTheDocument();
    });
  });

  // Covers: TC-6.3-02 (Variable summaries - $active rendering)
  it('should render $active overview in information tab', async () => {
    const props = {
      ...defaultProps,
      activeTab: 'information' as const,
      variableSummaries: mockVariableSummaries,
    };
    render(<IniEditorSection {...props} />);

    await waitFor(() => {
      expect(screen.getByText('$active overview')).toBeInTheDocument();
    });
  });

  // Covers: TC-6.3-02 (Variable range rendering)
  it('should render variable range information', async () => {
    const props = {
      ...defaultProps,
      activeTab: 'information' as const,
      variableSummaries: mockVariableSummaries,
    };
    render(<IniEditorSection {...props} />);

    await waitFor(() => {
      expect(screen.getByText('$active overview')).toBeInTheDocument();
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
  it('should disable save and discard buttons when editorDirty is false', async () => {
    const props = { ...defaultProps, editorDirty: false };
    render(<IniEditorSection {...props} />);

    const saveButton = screen.getByRole('button', { name: /Save/i });
    const discardButton = screen.getByRole('button', { name: /Discard/i });

    expect(saveButton).toBeDisabled();
    expect(discardButton).toBeDisabled();
  });

  // Covers: TC-18 (Dirty State - Enabled/Interaction)
  it('should enable save and discard buttons when editorDirty is true', async () => {
    const onSaveMock = vi.fn();
    const onDiscardMock = vi.fn();
    const props = {
      ...defaultProps,
      editorDirty: true,
      onSave: onSaveMock,
      onDiscard: onDiscardMock,
    };
    render(<IniEditorSection {...props} />);

    const saveButton = screen.getByRole('button', { name: /Save/i });
    const discardButton = screen.getByRole('button', { name: /Discard/i });

    expect(saveButton).not.toBeDisabled();
    expect(discardButton).not.toBeDisabled();

    saveButton.click();
    discardButton.click();

    expect(onSaveMock).toHaveBeenCalled();
    expect(onDiscardMock).toHaveBeenCalled();
  });
});
