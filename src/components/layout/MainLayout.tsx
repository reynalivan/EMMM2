import TopBar from './TopBar';
import ResizableWorkspace from './ResizableWorkspace';
import Dashboard from '../../features/dashboard/Dashboard';
import ObjectList from '../../features/sidebar/ObjectList';
import FolderGrid from '../../features/explorer/FolderGrid';
import PreviewPanel from '../../features/details/PreviewPanel';
import { useAppStore } from '../../stores/useAppStore';

export default function MainLayout() {
  const { workspaceView } = useAppStore();

  return (
    <div className="flex flex-col h-screen overflow-hidden bg-base-100 font-sans text-base-content selection:bg-primary/20">
      {/* Top Navigation Bar */}
      <TopBar />

      {/* Main Workspace Area */}
      <div className="flex-1 min-h-0 relative">
        {workspaceView === 'dashboard' ? (
          <Dashboard />
        ) : (
          <ResizableWorkspace
            leftPanel={<ObjectList />}
            mainPanel={<FolderGrid />}
            rightPanel={<PreviewPanel />}
          />
        )}
      </div>
    </div>
  );
}
