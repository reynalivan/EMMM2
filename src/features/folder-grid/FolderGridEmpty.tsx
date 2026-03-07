import { FolderOpen, ChevronLeft, Upload, FolderInput } from 'lucide-react';

export interface FolderGridEmptyProps {
  explorerSearchQuery: string;
  currentPath: string[];
  setExplorerSearch: (query: string) => void;
  handleBreadcrumbClick: (index: number) => void;
  handleImportFiles: (paths: string[]) => void;
}

export default function FolderGridEmpty({
  explorerSearchQuery,
  currentPath,
  setExplorerSearch,
  handleBreadcrumbClick,
  handleImportFiles,
}: FolderGridEmptyProps) {
  return (
    <div className="flex-1 flex flex-col items-center justify-center gap-4 p-6">
      <FolderOpen size={40} className="text-base-content/15" />
      <p className="text-sm text-base-content/40 text-center">
        {explorerSearchQuery
          ? 'No mods match your search'
          : currentPath.length > 0
            ? 'This folder is empty.'
            : 'No mod folders found. Add mods to your game directory to get started.'}
      </p>

      {explorerSearchQuery && (
        <button
          className="btn btn-sm btn-ghost gap-2 text-primary mt-2"
          onClick={() => setExplorerSearch('')}
        >
          Clear Search
        </button>
      )}

      {/* When navigated into an empty folder → show back + import suggestion */}
      {!explorerSearchQuery && currentPath.length > 0 && (
        <div className="flex flex-col items-center gap-3 mt-2">
          <button
            className="btn btn-ghost btn-sm gap-2 text-base-content/60"
            onClick={() => handleBreadcrumbClick(currentPath.length - 2)}
          >
            <ChevronLeft size={16} />
            Go back
          </button>

          <div className="divider text-base-content/20 text-[10px] my-0">OR</div>

          <p className="text-xs text-base-content/30">Import mods into this folder</p>
          <div className="flex gap-2">
            <button
              className="btn btn-outline btn-sm gap-2"
              onClick={async () => {
                const { open } = await import('@tauri-apps/plugin-dialog');
                const selected = await open({
                  multiple: true,
                  filters: [{ name: 'Archives', extensions: ['zip', 'rar', '7z'] }],
                });
                if (selected) {
                  const paths = Array.isArray(selected) ? selected : [selected];
                  handleImportFiles(paths);
                }
              }}
            >
              <Upload size={14} />
              Import Archive
            </button>
            <button
              className="btn btn-outline btn-sm gap-2"
              onClick={async () => {
                const { open } = await import('@tauri-apps/plugin-dialog');
                const selected = await open({ directory: true, multiple: false });
                if (selected) {
                  const paths = Array.isArray(selected) ? selected : [selected];
                  handleImportFiles(paths);
                }
              }}
            >
              <FolderInput size={14} />
              Import Folder
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
