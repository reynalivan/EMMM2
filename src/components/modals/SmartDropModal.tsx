import { motion, AnimatePresence } from 'framer-motion';
import { Package, FolderInput, FilePlus2, X } from 'lucide-react';

export type ImportStrategy = 'Raw' | 'AutoOrganize';

interface Props {
  isOpen: boolean;
  files: string[];
  targetDir: string;
  onConfirm: (strategy: ImportStrategy) => void;
  onCancel: () => void;
  isProcessing: boolean;
}

export default function SmartDropModal({
  isOpen,
  files,
  targetDir,
  onConfirm,
  onCancel,
  isProcessing,
}: Props) {
  if (!isOpen) return null;

  return (
    <AnimatePresence>
      <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
        <motion.div
          initial={{ opacity: 0, scale: 0.9 }}
          animate={{ opacity: 1, scale: 1 }}
          exit={{ opacity: 0, scale: 0.9 }}
          className="card w-full max-w-lg bg-base-100 shadow-xl border border-base-300"
        >
          <div className="card-body">
            <h2 className="card-title flex justify-between items-center">
              <span className="flex items-center gap-2">
                <FilePlus2 className="w-5 h-5 text-primary" />
                Import Mods
              </span>
              {!isProcessing && (
                <button className="btn btn-ghost btn-xs btn-circle" onClick={onCancel}>
                  <X className="w-4 h-4" />
                </button>
              )}
            </h2>

            <div className="py-2">
              <p className="text-sm text-base-content/70">
                You dropped <span className="font-bold">{files.length}</span> file(s).
              </p>
              <div className="text-xs font-mono bg-base-200 p-2 rounded mt-2 truncate">
                Target: {targetDir}
              </div>
            </div>

            <div className="grid grid-cols-1 gap-4 mt-2">
              <button
                className="btn btn-outline btn-info h-auto py-4 flex flex-col items-start gap-2 hover:bg-info/10 group text-left"
                onClick={() => onConfirm('AutoOrganize')}
                disabled={isProcessing}
              >
                <div className="flex items-center gap-2 w-full">
                  <Package className="w-5 h-5 group-hover:scale-110 transition-transform" />
                  <span className="font-bold text-base">Auto-Organize</span>
                  <span className="badge badge-sm badge-info ml-auto">Recommended</span>
                </div>
                <p className="text-xs font-normal opacity-80">
                  Automatically sort into categories (Character, Weapon, etc.) using Deep Matcher.
                </p>
              </button>

              <button
                className="btn btn-outline h-auto py-4 flex flex-col items-start gap-2 hover:bg-base-content/10 text-left"
                onClick={() => onConfirm('Raw')}
                disabled={isProcessing}
              >
                <div className="flex items-center gap-2">
                  <FolderInput className="w-5 h-5" />
                  <span className="font-bold text-base">Folder Import</span>
                </div>
                <p className="text-xs font-normal opacity-80">
                  Copy files directly to the target folder without organizing.
                </p>
              </button>
            </div>
          </div>
        </motion.div>
      </div>
    </AnimatePresence>
  );
}
