import { useBrowserStore } from '../../../stores/useBrowserStore';
import { useDownloads } from '../hooks/useDownloads';
import type { BrowserDownloadItem, DownloadStatus } from '../types';

const STATUS_BADGE: Record<DownloadStatus, { label: string; cls: string }> = {
  requested: { label: 'Queued', cls: 'badge-neutral' },
  in_progress: { label: 'Downloading', cls: 'badge-info' },
  finished: { label: 'Ready', cls: 'badge-success' },
  failed: { label: 'Failed', cls: 'badge-error' },
  canceled: { label: 'Canceled', cls: 'badge-warning' },
  imported: { label: 'Imported', cls: 'badge-ghost' },
};

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

interface Props {
  onImportSelected: (ids: string[], gameId: string) => void;
}

export function DownloadManagerPanel({ onImportSelected }: Props) {
  const {
    isDownloadPanelOpen,
    closeDownloadPanel,
    selectedDownloadIds,
    toggleSelectDownload,
    selectAll,
    clearSelection,
  } = useBrowserStore();

  const { downloads, deleteDownload, cancelDownload, clearImported } = useDownloads();

  const finishedDownloads = downloads.filter((d) => d.status === 'finished');
  const finishedIds = finishedDownloads.map((d) => d.id);
  const allFinishedSelected =
    finishedIds.length > 0 && finishedIds.every((id) => selectedDownloadIds.has(id));

  const handleSelectAll = () => {
    if (allFinishedSelected) {
      clearSelection();
    } else {
      selectAll(finishedIds);
    }
  };

  const handleImport = async () => {
    const selectedIds = Array.from(selectedDownloadIds);
    if (selectedIds.length === 0) return;
    // onImportSelected will open GamePickerModal; the game_id comes back via callback
    onImportSelected(selectedIds, '');
  };

  const handleImportSingle = (id: string) => {
    onImportSelected([id], '');
  };

  return (
    <div
      id="download-manager-panel"
      className={`
        fixed top-0 right-0 h-full w-[400px] z-60 bg-base-200 shadow-2xl
        transition-transform duration-300 ease-in-out flex flex-col
        ${isDownloadPanelOpen ? 'translate-x-0' : 'translate-x-full'}
      `}
    >
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-base-300">
        <div className="flex items-center gap-2">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            className="w-5 h-5 text-primary"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"
            />
          </svg>
          <h2 className="font-semibold text-base-content">
            Downloads
            {downloads.length > 0 && (
              <span className="ml-2 badge badge-primary badge-sm">{downloads.length}</span>
            )}
          </h2>
        </div>
        <button
          id="download-panel-close-btn"
          className="btn btn-ghost btn-sm btn-circle"
          onClick={closeDownloadPanel}
          aria-label="Close download panel"
        >
          ✕
        </button>
      </div>

      {/* Toolbar */}
      {downloads.length > 0 && (
        <div className="flex items-center gap-2 px-4 py-2 border-b border-base-300">
          <input
            id="download-select-all-checkbox"
            type="checkbox"
            className="checkbox checkbox-sm checkbox-primary"
            checked={allFinishedSelected}
            onChange={handleSelectAll}
            title={allFinishedSelected ? 'Deselect all' : 'Select all finished'}
          />
          <span className="text-xs text-base-content/60">
            {selectedDownloadIds.size > 0
              ? `${selectedDownloadIds.size} selected`
              : 'Select finished'}
          </span>

          <div className="ml-auto flex gap-2">
            {selectedDownloadIds.size > 0 && (
              <button
                id="download-import-selected-btn"
                className="btn btn-primary btn-xs"
                onClick={handleImport}
              >
                Import Selected
              </button>
            )}
            <button
              id="download-clear-imported-btn"
              className="btn btn-ghost btn-xs"
              onClick={() => clearImported()}
            >
              Clear Imported
            </button>
          </div>
        </div>
      )}

      {/* List */}
      <div className="flex-1 overflow-y-auto py-2">
        {downloads.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full gap-3 text-base-content/40">
            <svg
              xmlns="http://www.w3.org/2000/svg"
              className="w-12 h-12"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={1}
                d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4"
              />
            </svg>
            <p className="text-sm">No downloads yet</p>
          </div>
        ) : (
          downloads.map((item) => (
            <DownloadRow
              key={item.id}
              item={item}
              selected={selectedDownloadIds.has(item.id)}
              onToggle={() => toggleSelectDownload(item.id)}
              onDelete={(deleteFile) => deleteDownload({ id: item.id, deleteFile })}
              onCancel={() => cancelDownload(item.id)}
              onImport={() => handleImportSingle(item.id)}
            />
          ))
        )}
      </div>
    </div>
  );
}

interface RowProps {
  item: BrowserDownloadItem;
  selected: boolean;
  onToggle: () => void;
  onDelete: (deleteFile: boolean) => void;
  onCancel: () => void;
  onImport: () => void;
}

function DownloadRow({ item, selected, onToggle, onDelete, onCancel, onImport }: RowProps) {
  const badge = STATUS_BADGE[item.status];
  const progress =
    item.bytes_total && item.bytes_total > 0
      ? Math.round((item.bytes_received / item.bytes_total) * 100)
      : null;

  return (
    <div
      className={`
        flex items-start gap-3 px-4 py-3 border-b border-base-300/50
        hover:bg-base-300/30 transition-colors
        ${selected ? 'bg-primary/5' : ''}
      `}
    >
      {/* Checkbox (only for finished) */}
      {item.status === 'finished' ? (
        <input
          type="checkbox"
          className="checkbox checkbox-sm checkbox-primary mt-1"
          checked={selected}
          onChange={onToggle}
        />
      ) : (
        <div className="w-4" />
      )}

      {/* Info */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <p className="text-sm font-medium text-base-content truncate max-w-[200px]">
            {item.filename}
          </p>
          <span className={`badge badge-sm ${badge.cls}`}>{badge.label}</span>
        </div>

        {/* Progress bar */}
        {item.status === 'in_progress' && progress !== null && (
          <div className="mt-1">
            <progress className="progress progress-primary w-full h-1" value={progress} max="100" />
            <p className="text-xs text-base-content/50 mt-0.5">
              {formatBytes(item.bytes_received)}
              {item.bytes_total ? ` / ${formatBytes(item.bytes_total)}` : ''}
            </p>
          </div>
        )}

        {/* Error */}
        {item.status === 'failed' && item.error_msg && (
          <p className="text-xs text-error mt-1 truncate">{item.error_msg}</p>
        )}
      </div>

      {/* Actions */}
      <div className="flex gap-1">
        {item.status === 'finished' && (
          <button
            className="btn btn-ghost btn-xs text-success"
            onClick={onImport}
            title="Import this file"
          >
            Import
          </button>
        )}
        {item.status === 'in_progress' && (
          <button
            className="btn btn-ghost btn-xs text-warning"
            onClick={onCancel}
            title="Cancel download"
          >
            ✕
          </button>
        )}
        {(item.status === 'finished' ||
          item.status === 'failed' ||
          item.status === 'canceled' ||
          item.status === 'imported') && (
          <button
            className="btn btn-ghost btn-xs text-error"
            onClick={() => onDelete(false)}
            title="Remove from list"
          >
            Delete
          </button>
        )}
      </div>
    </div>
  );
}
