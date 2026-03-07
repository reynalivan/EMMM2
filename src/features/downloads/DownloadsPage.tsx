import { useState } from 'react';
import { useDownloads } from '../browser/hooks/useDownloads';
import { Download, PlayCircle, Trash2, CheckCircle2, AlertCircle, RefreshCw } from 'lucide-react';
import { GamePickerModal } from '../browser/components/GamePickerModal';
import { invoke } from '@tauri-apps/api/core';

export default function DownloadsPage() {
  const { downloads, deleteDownload, cancelDownload, clearImported } = useDownloads();
  const [importIds, setImportIds] = useState<string[]>([]);
  const [isGamePickerOpen, setIsGamePickerOpen] = useState(false);

  const formatBytes = (n: number) => {
    if (n < 1024) return `${n} B`;
    if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
    return `${(n / (1024 * 1024)).toFixed(1)} MB`;
  };

  const handleImport = (id: string) => {
    setImportIds([id]);
    setIsGamePickerOpen(true);
  };

  const handleGameConfirm = async (gameId: string) => {
    try {
      await invoke('browser_import_downloads', { downloadIds: importIds, gameId });
    } catch (err) {
      console.error('Failed to import:', err);
    } finally {
      setIsGamePickerOpen(false);
      setImportIds([]);
    }
  };

  const renderStatus = (status: string) => {
    switch (status) {
      case 'in_progress':
        return (
          <span className="badge badge-info gap-1">
            <RefreshCw size={12} className="animate-spin" /> Downloading
          </span>
        );
      case 'finished':
        return (
          <span className="badge badge-success gap-1">
            <CheckCircle2 size={12} /> Ready
          </span>
        );
      case 'failed':
        return (
          <span className="badge badge-error gap-1">
            <AlertCircle size={12} /> Failed
          </span>
        );
      case 'canceled':
        return <span className="badge badge-warning gap-1">Canceled</span>;
      case 'imported':
        return <span className="badge badge-ghost gap-1">Imported</span>;
      default:
        return <span className="badge badge-neutral gap-1">Queued</span>;
    }
  };

  return (
    <div className="flex flex-col h-full bg-base-100 overflow-hidden relative">
      {/* Header */}
      <div className="w-full bg-base-200 border-b border-base-300 p-6 z-10">
        <div className="flex items-center justify-between max-w-7xl mx-auto">
          <div>
            <h1 className="text-3xl font-bold flex items-center gap-3">
              <Download size={32} className="text-primary" />
              Downloads Manager
            </h1>
            <p className="text-base-content/60 mt-1">
              Manage your intercepted mod downloads and import them to your games.
            </p>
          </div>
          <div>
            <button className="btn btn-outline btn-sm gap-2" onClick={() => clearImported()}>
              <Trash2 size={16} /> Clear Imported
            </button>
          </div>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto w-full p-6">
        <div className="max-w-7xl mx-auto space-y-4">
          {downloads.length === 0 ? (
            <div className="flex flex-col items-center justify-center p-20 text-center opacity-50">
              <Download size={64} className="mb-4" />
              <h2 className="text-xl font-semibold">No downloads found</h2>
              <p>Browse the web inside EMMM2 to intercept and download mods.</p>
            </div>
          ) : (
            <div className="overflow-x-auto bg-base-200/50 rounded-2xl border border-base-300">
              <table className="table">
                <thead>
                  <tr>
                    <th>File Name</th>
                    <th>Status</th>
                    <th>Progress</th>
                    <th className="text-right">Actions</th>
                  </tr>
                </thead>
                <tbody>
                  {downloads.map((item) => {
                    const progress = item.bytes_total
                      ? Math.round((item.bytes_received / item.bytes_total) * 100)
                      : 0;

                    return (
                      <tr key={item.id} className="hover">
                        <td className="w-1/3">
                          <p
                            className="font-semibold text-base-content truncate max-w-[300px]"
                            title={item.filename}
                          >
                            {item.filename}
                          </p>
                        </td>
                        <td className="w-1/6">
                          {renderStatus(item.status)}
                          {item.error_msg && (
                            <p
                              className="text-xs text-error mt-1 truncate max-w-[150px]"
                              title={item.error_msg}
                            >
                              {item.error_msg}
                            </p>
                          )}
                        </td>
                        <td className="w-1/4">
                          {item.status === 'in_progress' ? (
                            <div>
                              <progress
                                className="progress progress-primary w-full"
                                value={progress}
                                max="100"
                              />
                              <div className="flex justify-between text-xs mt-1 text-base-content/60">
                                <span>{formatBytes(item.bytes_received)}</span>
                                <span>
                                  {item.bytes_total
                                    ? formatBytes(item.bytes_total)
                                    : 'Unknown size'}
                                </span>
                              </div>
                            </div>
                          ) : (
                            <span className="text-sm text-base-content/60">
                              {item.bytes_total
                                ? formatBytes(item.bytes_total)
                                : formatBytes(item.bytes_received)}
                            </span>
                          )}
                        </td>
                        <td className="w-1/4 text-right">
                          <div className="flex items-center justify-end gap-2">
                            {item.status === 'finished' && (
                              <button
                                className="btn btn-sm btn-primary gap-2"
                                onClick={() => handleImport(item.id)}
                              >
                                <PlayCircle size={16} /> Import
                              </button>
                            )}
                            {item.status === 'in_progress' && (
                              <button
                                className="btn btn-sm btn-warning"
                                onClick={() => cancelDownload(item.id)}
                              >
                                Cancel
                              </button>
                            )}
                            <button
                              className="btn btn-sm btn-ghost text-error"
                              onClick={() => deleteDownload({ id: item.id, deleteFile: false })}
                              title="Remove from list"
                            >
                              <Trash2 size={16} />
                            </button>
                          </div>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          )}
        </div>
      </div>

      <div className="fixed z-10010">
        <GamePickerModal
          downloadIds={importIds}
          open={isGamePickerOpen}
          onClose={() => setIsGamePickerOpen(false)}
          onConfirm={handleGameConfirm}
        />
      </div>
    </div>
  );
}
