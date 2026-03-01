import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import FolderTooltip from './FolderTooltip';

vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: vi.fn(),
  invoke: vi.fn().mockResolvedValue([]),
}));

describe('FolderTooltip', () => {
  it('renders trigger children', () => {
    render(
      <FolderTooltip folderPath="C:\mods\folder1" thumbnailPath={null} gameId="g1">
        <button>Hover Me</button>
      </FolderTooltip>,
    );
    expect(screen.getByText('Hover Me')).toBeInTheDocument();
  });
});
