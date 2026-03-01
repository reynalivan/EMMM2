import { useRef, useState, useEffect } from 'react';
import { Package, Lock, AlertTriangle, FileArchive } from 'lucide-react';
import type { ArchiveInfo } from '../../../types/scanner';

interface Props {
  archives: ArchiveInfo[];
  isOpen: boolean;
  onExtract: (selectedPaths: string[], password?: string, overwrite?: boolean) => Promise<void>;
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
  // Uncontrolled with key pattern: State initializes from props on mount.
  // Parent must change 'key' prop to reset selection.
  const [selectedPaths, setSelectedPaths] = useState<Set<string>>(
    () => new Set(archives.map((a) => a.path)),
  );
  const [password, setPassword] = useState('');
  const [showPasswordInput, setShowPasswordInput] = useState(false);
  const [overwrite, setOverwrite] = useState(false);

  // Derived state: Force expand if there's a password error
  const isPasswordError = error ? error.toLowerCase().includes('password') : false;
  const shouldShowPassword = showPasswordInput || isPasswordError;

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
    if (next.has(path)) {
      next.delete(path);
    } else {
      next.add(path);
    }
    setSelectedPaths(next);
  };

  const handleExtract = () => {
    onExtract(Array.from(selectedPaths), password || undefined, overwrite);
  };

  if (!archives.length) return null;

  return (
    <dialog ref={dialogRef} className="modal modal-bottom sm:modal-middle" onClose={onSkip}>
      <div className="modal-box w-11/12 max-w-2xl bg-base-100 p-0 overflow-hidden">
        {/* Header */}
        <div className="bg-base-200 p-4 flex items-center gap-3 border-b border-base-300">
          <div className="w-10 h-10 rounded-lg bg-primary/10 flex items-center justify-center text-primary">
            <Package className="w-6 h-6" />
          </div>
          <div>
            <h3 className="font-bold text-lg">Archives Detected</h3>
            <p className="text-xs text-base-content/60">
              Found {archives.length} archive(s) in your mods folder.
            </p>
          </div>
        </div>

        {/* Content */}
        <div className="p-4 space-y-4">
          <div role="alert" className="alert alert-info alert-soft text-sm">
            <FileArchive className="w-4 h-4" />
            <span>Select archives to extract. Originals will be moved to backup.</span>
          </div>

          <div className="overflow-x-auto border border-base-300 rounded-lg">
            <table className="table table-sm table-pin-rows">
              <thead>
                <tr className="bg-base-200">
                  <th className="w-10">
                    <label>
                      <input
                        type="checkbox"
                        className="checkbox checkbox-xs"
                        checked={selectedPaths.size === archives.length}
                        onChange={(e) => {
                          if (e.target.checked) {
                            setSelectedPaths(new Set(archives.map((a) => a.path)));
                          } else {
                            setSelectedPaths(new Set());
                          }
                        }}
                      />
                    </label>
                  </th>
                  <th>Archive Name</th>
                  <th className="text-right">Size</th>
                  <th className="w-10"></th>
                </tr>
              </thead>
              <tbody>
                {archives.map((archive) => (
                  <tr key={archive.path} className="hover:bg-base-200/50">
                    <td>
                      <label>
                        <input
                          type="checkbox"
                          className="checkbox checkbox-xs checkbox-primary"
                          checked={selectedPaths.has(archive.path)}
                          onChange={() => toggleSelection(archive.path)}
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
                    <td>
                      {archive.has_ini === false && (
                        <div
                          className="tooltip tooltip-left text-warning"
                          data-tip="No INI found (might not be a mod)"
                        >
                          <AlertTriangle className="w-4 h-4" />
                        </div>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          {/* Password Option */}
          <div className="collapse collapse-arrow bg-base-200/50 border border-base-300 rounded-lg">
            <input
              type="checkbox"
              checked={shouldShowPassword}
              disabled={!!isPasswordError}
              onChange={(e) => setShowPasswordInput(e.target.checked)}
            />
            <div className="collapse-title text-sm font-medium flex items-center gap-2">
              <Lock className="w-4 h-4" />
              Archives are encrypted?
            </div>
            <div className="collapse-content">
              <input
                type="password"
                placeholder="Enter password..."
                className="input input-sm input-bordered w-full"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
              />
              {error && (
                <div role="alert" className="alert alert-error mt-2 py-1 text-xs">
                  <AlertTriangle className="w-3 h-3" />
                  <span>{error}</span>
                </div>
              )}
            </div>
            {/* Overwrite Option */}
            <div className="form-control">
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
        </div>

        {/* Actions */}
        <div className="modal-action bg-base-200 p-4 m-0 flex justify-between items-center border-t border-base-300">
          <button className="btn btn-ghost btn-sm" onClick={onSkip} disabled={isExtracting}>
            Skip Extraction
          </button>
          <div className="flex gap-2">
            <div className="text-xs text-base-content/50 self-center mr-2">
              {selectedPaths.size} selected
            </div>
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
