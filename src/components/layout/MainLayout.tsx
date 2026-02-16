import { useState, useCallback } from 'react';
import TopBar from './TopBar';
import ResizableWorkspace from './ResizableWorkspace';
import Dashboard from '../../features/dashboard/Dashboard';
import ObjectList from '../../features/sidebar/ObjectList';
import FolderGrid from '../../features/explorer/FolderGrid';
import PreviewPanel from '../../features/details/PreviewPanel';
import SettingsPage from '../../features/settings/SettingsPage';
import CollectionsPage from '../../features/collections/CollectionsPage';
import ExplorerEmptyState from '../../features/explorer/ExplorerEmptyState';
import { useAppStore } from '../../stores/useAppStore';
import { ExternalChangeHandler } from '../ExternalChangeHandler';
import { ErrorBoundary } from '../ui/ErrorBoundary';

// Smart Drop Integration
import { useFileDrop } from '../../hooks/useFileDrop';
import SmartDropModal, { ImportStrategy } from '../modals/SmartDropModal';
import { useImportMods } from '../../hooks/useFolders';
import { useActiveGame } from '../../hooks/useActiveGame';
import { scanService } from '../../services/scanService';

export default function MainLayout() {
  const { workspaceView, selectedObject } = useAppStore();
  const { activeGame } = useActiveGame();

  // Drag & Drop Logic (Tauri v2 events)
  const [droppedFiles, setDroppedFiles] = useState<string[]>([]);
  const handleDrop = useCallback((paths: string[]) => setDroppedFiles(paths), []);
  const clearFiles = useCallback(() => setDroppedFiles([]), []);
  useFileDrop({ onDrop: handleDrop });
  const importMods = useImportMods();
  const [, setImportError] = useState<string | null>(null);

  const handleImport = async (strategy: ImportStrategy) => {
    if (!activeGame || droppedFiles.length === 0) return;

    try {
      let dbJson = null;
      if (strategy === 'AutoOrganize') {
        dbJson = await scanService.getMasterDb(activeGame.game_type);
      }

      await importMods.mutateAsync({
        paths: droppedFiles,
        targetDir: activeGame.mod_path,
        strategy,
        dbJson,
      });

      clearFiles();
    } catch (e) {
      console.error('Import failed', e);
      setImportError(String(e));
    }
  };

  return (
    <div className="flex flex-col h-screen overflow-hidden bg-base-100 font-sans text-base-content selection:bg-primary/20 relative">
      <ExternalChangeHandler />

      <SmartDropModal
        isOpen={droppedFiles.length > 0}
        files={droppedFiles}
        targetDir={activeGame?.mod_path || ''}
        onConfirm={handleImport}
        onCancel={clearFiles}
        isProcessing={importMods.isPending}
      />

      {/* Top Navigation Bar */}
      <TopBar />

      {/* Main Workspace Area */}
      <div className="flex-1 min-h-0 relative">
        <ErrorBoundary>
          {workspaceView === 'dashboard' ? (
            <Dashboard />
          ) : workspaceView === 'collections' ? (
            <CollectionsPage />
          ) : workspaceView === 'settings' ? (
            <SettingsPage />
          ) : (
            <ResizableWorkspace
              leftPanel={<ObjectList />}
              mainPanel={
                selectedObject ? (
                  <ErrorBoundary>
                    <FolderGrid />
                  </ErrorBoundary>
                ) : (
                  <ExplorerEmptyState />
                )
              }
              rightPanel={<PreviewPanel />}
            />
          )}
        </ErrorBoundary>
      </div>
    </div>
  );
}
