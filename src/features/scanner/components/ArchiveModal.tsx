import { useRef, useState, useEffect, useMemo } from 'react';
import { Package, Lock, AlertTriangle, CheckCircle2 } from 'lucide-react';
import type { ArchiveInfo } from '../../../types/scanner';

interface Props {
  archives: ArchiveInfo[];
  isOpen: boolean;
  onExtract: (
    selectedPaths: string[],
    passwords: Record<string, string>,
    overwrite?: boolean,
  ) => Promise<void>;
  onSkip: () => void;
  isExtracting: boolean;
  error?: string | null;
}

export default function ArchiveModal({
  archives,
  isOpen,
  onExtract,
  onSkip,
  isExtracting,
  error,
}: Props) {
  const dialogRef = useRef<HTMLDialogElement>(null);

  const [selectedPaths, setSelectedPaths] = useState<Set<string>>(() => {
    const validPaths = archives
      .filter((a) => a.file_count > 0 && a.has_ini !== false)
      .map((a) => a.path);
    return new Set(validPaths);
  });
  const [passwords, setPasswords] = useState<Record<string, string>>({});
  const [overwrite, setOverwrite] = useState(false);

  // Group archives by encryption
  const { encrypted, unencrypted } = useMemo(() => {
    const enc: ArchiveInfo[] = [];
    const unenc: ArchiveInfo[] = [];
    for (const a of archives) {
      if (a.is_encrypted) enc.push(a);
      else unenc.push(a);
    }
    return { encrypted: enc, unencrypted: unenc };
  }, [archives]);

  // Handle modal open/close
  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;

    if (isOpen) {
      dialog.showModal();
    } else {
      dialog.close();
    }
  }, [isOpen]);

  const toggleSelection = (path: string) => {
    const next = new Set(selectedPaths);
    if (next.has(path)) next.delete(path);
    else next.add(path);
    setSelectedPaths(next);
  };

  const setPasswordForPath = (path: string, pw: string) => {
    setPasswords((prev) => ({ ...prev, [path]: pw }));
  };

  const handleExtract = () => {
    onExtract(Array.from(selectedPaths), passwords, overwrite);
  };

  if (!archives.length) return null;

  return (
    <dialog ref={dialogRef} className="modal modal-bottom sm:modal-middle" onClose={onSkip}>
      <div className="modal-box w-11/12 max-w-2xl bg-base-100 p-0 overflow-hidden flex flex-col max-h-[90vh]">
        {/* Header */}
        <div className="bg-base-200 p-4 flex items-center gap-3 border-b border-base-300 shrink-0">
          <div className="w-10 h-10 rounded-lg bg-primary/10 flex items-center justify-center text-primary">
            <Package className="w-6 h-6" />
          </div>
          <div>
            <h3 className="font-bold text-lg">Archives Detected</h3>
            <p className="text-xs text-base-content/60">
              Found {archives.length} archive(s) ready for import.
            </p>
          </div>
        </div>

        {/* Content (Scrollable) */}
        <div className="p-4 space-y-6 overflow-y-auto">
          {error && (
            <div role="alert" className="alert alert-error text-sm py-2">
              <AlertTriangle className="w-4 h-4 shrink-0" />
              <span>{error}</span>
            </div>
          )}

          {/* Unencrypted Group */}
          {unencrypted.length > 0 && (
            <div className="space-y-2">
              <div className="flex items-center gap-2 text-sm font-semibold text-success px-1">
                <CheckCircle2 className="w-4 h-4" />
                No Password Required ({unencrypted.length})
              </div>
              <div className="overflow-hidden border border-base-300 rounded-lg">
                <table className="table table-sm">
                  <tbody className="divide-y divide-base-300">
                    {unencrypted.map((archive) => {
                      const isEmpty = archive.file_count === 0 || archive.has_ini === false;
                      const titleAttr = isEmpty ? 'Archive contains no mod files' : undefined;
                      return (
                        <tr
                          key={archive.path}
                          className={`hover:bg-base-200/50 ${isEmpty ? 'opacity-50' : ''}`}
                          title={titleAttr}
                        >
                          <td className="w-10 text-center">
                            <label>
                              <input
                                type="checkbox"
                                className="checkbox checkbox-sm checkbox-primary disabled:opacity-50 cursor-pointer disabled:cursor-not-allowed"
                                checked={selectedPaths.has(archive.path)}
                                onChange={() => toggleSelection(archive.path)}
                                disabled={isEmpty}
                                title={titleAttr}
                              />
                            </label>
                          </td>
                          <td>
                            <div className="flex flex-col">
                              <span className="font-medium text-sm">{archive.name}</span>
                              <span className="text-[10px] text-base-content/50 uppercase">
                                {archive.extension}
                              </span>
                            </div>
                          </td>
                          <td className="text-right text-xs font-mono opacity-70">
                            {(archive.size_bytes / 1024 / 1024).toFixed(2)} MB
                          </td>
                          <td className="w-10 text-center">
                            {isEmpty && (
                              <div
                                className="tooltip tooltip-left text-warning"
                                data-tip="Archive contains no mod files"
                              >
                                <AlertTriangle className="w-4 h-4 cursor-help" />
                              </div>
                            )}
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            </div>
          )}

          {/* Encrypted Group */}
          {encrypted.length > 0 && (
            <div className="space-y-2">
              <div className="flex items-center gap-2 text-sm font-semibold text-warning px-1">
                <Lock className="w-4 h-4" />
                Password Protected ({encrypted.length})
              </div>
              <div className="overflow-hidden border border-warning/30 rounded-lg">
                <table className="table table-sm">
                  <tbody className="divide-y divide-warning/10">
                    {encrypted.map((archive) => {
                      const isEmpty = archive.file_count === 0 || archive.has_ini === false;
                      const titleAttr = isEmpty ? 'Archive contains no mod files' : undefined;
                      return (
                        <tr
                          key={archive.path}
                          className={`hover:bg-warning/5 border-warning/10 ${isEmpty ? 'opacity-50' : ''}`}
                          title={titleAttr}
                        >
                          <td className="w-10 text-center align-top pt-3">
                            <label>
                              <input
                                type="checkbox"
                                className="checkbox checkbox-sm checkbox-warning disabled:opacity-50 cursor-pointer disabled:cursor-not-allowed"
                                checked={selectedPaths.has(archive.path)}
                                onChange={() => toggleSelection(archive.path)}
                                disabled={isEmpty}
                                title={titleAttr}
                              />
                            </label>
                          </td>
                          <td className="py-2">
                            <div className="flex flex-col mb-1.5">
                              <span className="font-medium text-sm">{archive.name}</span>
                              <div className="flex items-center gap-2">
                                <span className="text-[10px] text-base-content/50 uppercase">
                                  {archive.extension}
                                </span>
                                <span className="text-[10px] font-mono opacity-50">
                                  {(archive.size_bytes / 1024 / 1024).toFixed(2)} MB
                                </span>
                              </div>
                            </div>
                            <input
                              type="password"
                              placeholder="Key"
                              className="input input-xs input-bordered w-full max-w-[200px] bg-base-100"
                              value={passwords[archive.path] || ''}
                              onChange={(e) => setPasswordForPath(archive.path, e.target.value)}
                              disabled={!selectedPaths.has(archive.path) || isEmpty}
                            />
                          </td>
                          <td className="w-10 text-center align-top pt-3">
                            {isEmpty ? (
                              <div
                                className="tooltip tooltip-left text-error"
                                data-tip="Archive contains no mod files"
                              >
                                <AlertTriangle className="w-4 h-4 cursor-help" />
                              </div>
                            ) : (
                              <div
                                className="tooltip tooltip-left"
                                data-tip="Requires password to extract"
                              >
                                <Lock className="w-4 h-4 text-warning/70" />
                              </div>
                            )}
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            </div>
          )}

          {/* Overwrite Option */}
          <div className="form-control bg-base-200/50 rounded-lg p-2 border border-base-300">
            <label className="label cursor-pointer justify-start gap-4">
              <input
                type="checkbox"
                className="checkbox checkbox-sm checkbox-warning"
                checked={overwrite}
                onChange={(e) => setOverwrite(e.target.checked)}
              />
              <span className="label-text">Overwrite existing folders? (Danger)</span>
            </label>
          </div>
        </div>

        {/* Actions */}
        <div className="modal-action bg-base-200 p-4 m-0 flex justify-between items-center border-t border-base-300 shrink-0">
          <button className="btn btn-ghost btn-sm" onClick={onSkip} disabled={isExtracting}>
            Skip Extraction
          </button>
          <div className="flex gap-2 items-center">
            <div className="text-xs text-base-content/50 mr-2">{selectedPaths.size} selected</div>
            <button
              className="btn btn-primary btn-sm"
              onClick={handleExtract}
              disabled={selectedPaths.size === 0 || isExtracting}
            >
              {isExtracting ? (
                <>
                  <span className="loading loading-spinner loading-xs"></span>
                  Extracting...
                </>
              ) : (
                'Extract Selected'
              )}
            </button>
          </div>
        </div>
      </div>
      <form method="dialog" className="modal-backdrop">
        <button onClick={onSkip}>close</button>
      </form>
    </dialog>
  );
}
