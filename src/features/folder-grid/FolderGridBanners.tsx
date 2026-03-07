import { FolderOpen, AlertTriangle } from 'lucide-react';
import { useAppStore } from '../../stores/useAppStore';
import type { ConflictGroup } from '../../types/mod';

export interface FolderGridBannersProps {
  isLoading: boolean;
  isError: boolean;
  nameConflicts: ConflictGroup[];
  isFlatModRoot: boolean;
  selfIsEnabled: boolean;
  selfReasons: string[];
  isMobile: boolean;
  isPreviewOpen: boolean;
  setMobilePane: (pane: 'sidebar' | 'grid' | 'details') => void;
  togglePreview: () => void;
  handleToggleSelf: (enabled: boolean) => void;
}

export default function FolderGridBanners({
  isLoading,
  isError,
  nameConflicts,
  isFlatModRoot,
  selfIsEnabled,
  selfReasons,
  isMobile,
  isPreviewOpen,
  setMobilePane,
  togglePreview,
  handleToggleSelf,
}: FolderGridBannersProps) {
  if (isLoading || isError) {
    return null;
  }

  return (
    <>
      {/* Naming Conflict Banner */}
      {nameConflicts.length > 0 && (
        <div className="mb-3 flex items-center gap-2 bg-warning/10 border border-warning/20 rounded-lg px-3 py-2">
          <AlertTriangle size={16} className="text-warning shrink-0" />
          <span className="text-xs text-warning flex-1">
            {nameConflicts.length} name conflict{nameConflicts.length > 1 ? 's' : ''} detected —
            both enabled and disabled versions exist
          </span>
          <button
            className="btn btn-xs btn-warning btn-outline"
            onClick={() => {
              const c = nameConflicts[0];
              if (c.members.length >= 2) {
                const enabled = c.members.find((m) => m.is_enabled);
                const disabled = c.members.find((m) => !m.is_enabled);
                if (enabled && disabled) {
                  useAppStore.getState().openConflictDialog({
                    type: 'RenameConflict',
                    attempted_target: enabled.path,
                    existing_path: disabled.path,
                    base_name: c.base_name,
                  });
                }
              }
            }}
          >
            Resolve…
          </button>
        </div>
      )}

      {/* Flat Mod Root Banner */}
      {isFlatModRoot && (
        <div className="mb-4 bg-base-200 border border-base-content/10 rounded-xl p-6 flex flex-col md:flex-row items-start md:items-center gap-4 mx-4 shadow-sm relative overflow-hidden">
          {/* Decorative left accent */}
          <div
            className={`absolute left-0 top-0 bottom-0 w-1 ${selfIsEnabled ? 'bg-success' : 'bg-base-content/20'}`}
          />

          <div className="flex-1 pl-2">
            <h3 className="text-lg font-bold flex items-center gap-2">
              <FolderOpen
                className={selfIsEnabled ? 'text-success' : 'text-base-content/40'}
                size={20}
              />
              This Folder is a Mod
            </h3>
            <p className="text-sm text-base-content/60 mt-1 max-w-2xl">
              This directory contains mod wrapper files directly in its root. EMMM2 manages this
              folder as a single mod rather than a container of sub-mods.
            </p>
            {selfReasons.length > 0 && (
              <div className="mt-3 flex flex-wrap gap-1.5">
                {selfReasons.map((reason, i) => (
                  <span
                    key={i}
                    className="text-[10px] font-mono bg-base-300 px-2 py-0.5 rounded text-base-content/50 border border-base-content/5"
                  >
                    {reason}
                  </span>
                ))}
              </div>
            )}
          </div>

          <div className="flex items-center gap-2 mt-4 md:mt-0 whitespace-nowrap">
            <button
              className={`btn btn-sm ${selfIsEnabled ? 'btn-outline text-error hover:bg-error hover:text-error-content hover:border-error' : 'btn-success'}`}
              onClick={() => handleToggleSelf(!selfIsEnabled)}
            >
              {selfIsEnabled ? 'Disable Mod' : 'Enable Mod'}
            </button>
            <button
              className="btn btn-sm btn-outline btn-ghost"
              onClick={() => {
                // Only needed for mobile viewing, otherwise Details pane is always visible on desktop
                if (isMobile) {
                  setMobilePane('details');
                } else if (!isPreviewOpen) {
                  togglePreview();
                }
              }}
            >
              View Details
            </button>
          </div>
        </div>
      )}
    </>
  );
}
