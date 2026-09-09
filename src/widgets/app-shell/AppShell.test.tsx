import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import AppShell, { type AppShellProps } from './AppShell';

vi.mock('./ResizableWorkspace', () => ({
  default: ({
    leftPanel,
    mainPanel,
    rightPanel,
  }: {
    leftPanel: React.ReactNode;
    mainPanel: React.ReactNode;
    rightPanel: React.ReactNode;
  }) => (
    <div data-testid="resizable">
      {leftPanel}
      {mainPanel}
      {rightPanel}
    </div>
  ),
}));

const slots: AppShellProps = {
  topBar: <div data-testid="top-bar">Top bar</div>,
  dashboard: <div data-testid="dashboard">Dashboard</div>,
  collections: <div data-testid="collections">Collections</div>,
  settings: <div data-testid="settings">Settings</div>,
  modInbox: <div data-testid="mod-inbox">Mod inbox</div>,
  objectList: <div data-testid="object-list">Object list</div>,
  folderGrid: <div data-testid="folder-grid">Folder grid</div>,
  previewPanel: <div data-testid="preview-panel">Preview</div>,
  explorerEmptyState: <div data-testid="explorer-empty">Empty</div>,
};

describe('AppShell (TC-05)', () => {
  it('renders dashboard composition', () => {
    render(<AppShell {...slots} workspaceView="dashboard" />);
    expect(screen.getByTestId('top-bar')).toBeInTheDocument();
    expect(screen.getByTestId('dashboard')).toBeInTheDocument();
  });

  it('renders settings composition', () => {
    render(<AppShell {...slots} workspaceView="settings" />);
    expect(screen.getByTestId('settings')).toBeInTheDocument();
  });

  it('renders collections composition', () => {
    render(<AppShell {...slots} workspaceView="collections" />);
    expect(screen.getByTestId('collections')).toBeInTheDocument();
  });

  it('renders mod inbox composition', () => {
    render(<AppShell {...slots} workspaceView="mod-inbox" />);
    expect(screen.getByTestId('mod-inbox')).toBeInTheDocument();
  });

  it('renders explorer empty state without a selected object', () => {
    render(<AppShell {...slots} workspaceView="mods" selectedObjectFolderPath={null} />);
    expect(screen.getByTestId('resizable')).toBeInTheDocument();
    expect(screen.getByTestId('object-list')).toBeInTheDocument();
    expect(screen.getByTestId('explorer-empty')).toBeInTheDocument();
    expect(screen.getByTestId('preview-panel')).toBeInTheDocument();
  });

  it('renders folder grid with a selected object', () => {
    render(<AppShell {...slots} workspaceView="mods" selectedObjectFolderPath="Objects/Albedo" />);
    expect(screen.getByTestId('resizable')).toBeInTheDocument();
    expect(screen.getByTestId('folder-grid')).toBeInTheDocument();
  });
});
