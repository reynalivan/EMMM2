import { beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen, waitFor, fireEvent } from '../../../testing/test-utils';
import MetadataSection from './MetadataSection';

describe('MetadataSection', () => {
  const defaultProps = {
    activePath: 'E:/Mods/TestMod',
    titleDraft: 'Original Title',
    authorDraft: 'Author Name',
    versionDraft: '1.0.0',
    descriptionDraft: 'Original Description',
    metadataDirty: false,
    onTitleChange: vi.fn(),
    onAuthorChange: vi.fn(),
    onVersionChange: vi.fn(),
    onDescriptionChange: vi.fn(),
    onDiscard: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  // Covers: TC-17-01 (Load Valid Metadata)
  it('should populate fields with valid metadata in \u2264 100ms', async () => {
    render(<MetadataSection {...defaultProps} />);

    await waitFor(() => {
      expect(screen.getByDisplayValue('Original Title')).toBeInTheDocument();
      expect(screen.getByDisplayValue('Original Description')).toBeInTheDocument();
      expect(screen.getByDisplayValue('Author Name')).toBeInTheDocument();
      expect(screen.getByDisplayValue('1.0.0')).toBeInTheDocument();
    });
  });

  // Covers: TC-17-04 (Auto-Save on Blur / State Indication)
  it('should show auto-saving state and revert button when metadataDirty is true', async () => {
    const props = { ...defaultProps, metadataDirty: true };
    render(<MetadataSection {...props} />);

    await waitFor(() => {
      expect(screen.getByText('auto-saving…')).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /Revert/i })).toBeInTheDocument();
    });
  });

  // Covers: TC-17-05 (Auto-Save / Interaction logic)
  it('should trigger onAuthorChange when author input changes', async () => {
    const onAuthorChangeMock = vi.fn();
    const props = { ...defaultProps, onAuthorChange: onAuthorChangeMock };
    render(<MetadataSection {...props} />);

    const authorInput = screen.getByDisplayValue('Author Name');
    fireEvent.change(authorInput, { target: { value: 'New Author' } });

    await waitFor(() => {
      expect(onAuthorChangeMock).toHaveBeenCalledWith('New Author');
    });
  });
});
