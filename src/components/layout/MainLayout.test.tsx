import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import MainLayout from './MainLayout';

let mockView = 'dashboard';
let mockSelectedObject: unknown = null;

vi.mock('../../stores/useAppStore', () => ({
  useAppStore: () => ({
    workspaceView: mockView,
    selectedObject: mockSelectedObject,
  }),
}));

// Mock children
vi.mock('./top-bar/index', () => ({ default: () => <div data-testid="top-bar">TopBar</div> }));
vi.mock('../../features/dashboard/Dashboard', () => ({
  default: () => <div data-testid="dashboard">Dashboard</div>,
}));
vi.mock('../../features/collections/CollectionsPage', () => ({
  default: () => <div data-testid="collections">Collections</div>,
}));
vi.mock('../../features/settings/SettingsPage', () => ({
  default: () => <div data-testid="settings">Settings</div>,
}));
vi.mock('../../features/object-list/ObjectList', () => ({
  default: () => <div data-testid="object-list">object-list</div>,
}));
vi.mock('../../features/folder-grid/FolderGrid', () => ({
  default: () => <div data-testid="folder-grid">folder-grid</div>,
}));
vi.mock('../../features/preview/PreviewPanel', () => ({
  default: () => <div data-testid="preview-panel">PreviewPanel</div>,
}));
vi.mock('../../features/folder-grid/ExplorerEmptyState', () => ({
  default: () => <div data-testid="explorer-empty">ExplorerEmptyState</div>,
}));
vi.mock('../../features/file-watcher/ExternalChangeHandler', () => ({
  ExternalChangeHandler: () => null,
}));
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

describe('MainLayout (TC-05)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders Dashboard when view is dashboard', () => {
    mockView = 'dashboard';
    render(<MainLayout />);
    expect(screen.getByTestId('top-bar')).toBeInTheDocument();
    expect(screen.getByTestId('dashboard')).toBeInTheDocument();
  });

  it('renders Settings when view is settings', () => {
    mockView = 'settings';
    render(<MainLayout />);
    expect(screen.getByTestId('settings')).toBeInTheDocument();
  });

  it('renders Collections when view is collections', () => {
    mockView = 'collections';
    render(<MainLayout />);
    expect(screen.getByTestId('collections')).toBeInTheDocument();
  });

  it('renders ResizableWorkspace and EmptyState when view is string and NO selected object', () => {
    mockView = 'explorer';
    mockSelectedObject = null;
    render(<MainLayout />);

    expect(screen.getByTestId('resizable')).toBeInTheDocument();
    // Panels inside resizable wrapper
    expect(screen.getByTestId('object-list')).toBeInTheDocument();
    expect(screen.getByTestId('explorer-empty')).toBeInTheDocument();
    expect(screen.getByTestId('preview-panel')).toBeInTheDocument();
  });

  it('renders ResizableWorkspace and folder-grid when view is string AND HAS selected object', () => {
    mockView = 'explorer';
    mockSelectedObject = { id: 'obj1' }; // some truthy value
    render(<MainLayout />);

    expect(screen.getByTestId('resizable')).toBeInTheDocument();
    expect(screen.getByTestId('object-list')).toBeInTheDocument();
    expect(screen.getByTestId('folder-grid')).toBeInTheDocument();
    expect(screen.getByTestId('preview-panel')).toBeInTheDocument();
  });
});
