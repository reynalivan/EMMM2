import { useEffect, useMemo, useState } from 'react';
import { Loader2, PlayCircle, Save, Package } from 'lucide-react';
import { ModGroupList } from './ModGroupList';
import { buildGroupedModsWithObjectStates } from '../utils/groupMods';
import type {
  Collection,
  CollectionObjectState,
  CollectionPreviewMod,
} from '../../../types/collection';

interface WorkspaceObjectState extends CollectionObjectState {
  name: string;
  object_type: string;
}

interface CollectionWorkspaceProps {
  collection: Collection;
  sourceKind: 'current_runtime' | 'stored_unsaved_snapshot' | 'named_collection';
  primaryActionKind: 'save_current' | 'save_snapshot' | 'apply';
  previewRoots: CollectionPreviewMod[];
  isPreviewLoading: boolean;
  onPrimaryAction: (collection: Collection) => void;
  isApplying: boolean;
  objectStates: WorkspaceObjectState[];
  allowObjectStateEditing: boolean;
  isSavingObjectStates: boolean;
  onSaveObjectStates: (states: CollectionObjectState[]) => Promise<boolean>;
  onWorkspaceStateChange?: (draftStates: CollectionObjectState[], hasChanges: boolean) => void;
}

function hasDraftObjectStateChanges(
  draftObjectStates: WorkspaceObjectState[],
  objectStates: WorkspaceObjectState[],
): boolean {
  if (draftObjectStates.length !== objectStates.length) {
    return true;
  }

  const originalMap = new Map(objectStates.map((state) => [state.object_id, state.is_enabled]));
  return draftObjectStates.some((state) => originalMap.get(state.object_id) !== state.is_enabled);
}

export default function CollectionWorkspace({
  collection,
  sourceKind,
  primaryActionKind,
  previewRoots,
  isPreviewLoading,
  onPrimaryAction,
  isApplying,
  objectStates,
  allowObjectStateEditing,
  isSavingObjectStates,
  onSaveObjectStates,
  onWorkspaceStateChange,
}: CollectionWorkspaceProps) {
  const [draftObjectStates, setDraftObjectStates] = useState<WorkspaceObjectState[]>([]);
  const [expansionMode, setExpansionMode] = useState<'default' | 'all' | 'none'>('default');

  useEffect(() => {
    setDraftObjectStates(objectStates);
    onWorkspaceStateChange?.(
      objectStates.map(({ object_id, is_enabled }) => ({ object_id, is_enabled })),
      false,
    );
  }, [objectStates, onWorkspaceStateChange]);

  useEffect(() => {
    setExpansionMode('default');
  }, [collection.id]);
  const groupedObjects = useMemo(
    () =>
      buildGroupedModsWithObjectStates(
        previewRoots,
        draftObjectStates.map((state) => ({
          ...state,
          is_editable: allowObjectStateEditing,
        })),
        {
          mode: 'workspace',
        },
      ),
    [allowObjectStateEditing, previewRoots, draftObjectStates],
  );

  const hasObjectStateChanges = useMemo(() => {
    return hasDraftObjectStateChanges(draftObjectStates, objectStates);
  }, [draftObjectStates, objectStates]);

  const badgeLabel =
    sourceKind === 'current_runtime'
      ? 'Current'
      : sourceKind === 'stored_unsaved_snapshot'
        ? 'Last Unsaved'
        : null;

  const primaryButtonClass = primaryActionKind === 'apply' ? 'btn-primary' : 'btn-secondary';
  const primaryButtonDisabled =
    primaryActionKind === 'apply' ? isApplying : false;
  const primaryButtonContent =
    primaryActionKind === 'save_current' ? (
      <>
        <Save size={14} />
        Save Current State
      </>
    ) : primaryActionKind === 'save_snapshot' ? (
      <>
        <Save size={14} />
        Save Snapshot
      </>
    ) : isApplying ? (
      <Loader2 size={14} className="animate-spin" />
    ) : (
      <>
        <PlayCircle size={14} />
        Apply Collection
      </>
    );

  if (isPreviewLoading) {
    return (
      <div className="flex flex-col h-full bg-base-100 flex-1 relative items-center justify-center min-h-125">
        <Loader2 size={32} className="animate-spin text-primary opacity-50 mb-4" />
        <p className="text-base-content/50">Loading collection details...</p>
      </div>
    );
  }

  const toggleObjectState = (objectId: string) => {
    if (!allowObjectStateEditing) {
      return;
    }
    const nextDraftStates = draftObjectStates.map((state) =>
      state.object_id === objectId ? { ...state, is_enabled: !state.is_enabled } : state,
    );
    setDraftObjectStates(nextDraftStates);
    onWorkspaceStateChange?.(
      nextDraftStates.map(({ object_id, is_enabled }) => ({ object_id, is_enabled })),
      hasDraftObjectStateChanges(nextDraftStates, objectStates),
    );
  };

  const saveObjectStates = async () => {
    await onSaveObjectStates(
      draftObjectStates.map(({ object_id, is_enabled }) => ({ object_id, is_enabled })),
    );
  };

  return (
    <div className="flex flex-col h-full w-full relative">
      <div className="h-14 bg-base-300/50 backdrop-blur-md border-b border-white/5 flex items-center justify-between px-4 shrink-0 z-10">
        <div className="flex items-center gap-3 min-w-0 flex-1">
          <div className="flex flex-col min-w-0">
            <h2 className="font-bold text-sm leading-tight flex items-center gap-2 truncate">
              <span className="truncate">{collection.name}</span>
              {badgeLabel && (
                <span className="badge badge-sm badge-warning opacity-90 text-[10px] py-0 h-4 uppercase font-bold tracking-wider shrink-0">
                  {badgeLabel}
                </span>
              )}
            </h2>
            <span className="text-[10px] text-base-content/50 truncate">
              {groupedObjects.length} objects • {previewRoots.length} mods
            </span>
          </div>
        </div>

        <div className="flex items-center gap-2 shrink-0">
          <button
            onClick={() => setExpansionMode('all')}
            className="btn btn-xs btn-ghost text-base-content/50"
          >
            Expand All
          </button>
          <button
            onClick={() => setExpansionMode('none')}
            className="btn btn-xs btn-ghost text-base-content/50"
          >
            Collapse All
          </button>
          {allowObjectStateEditing && (
            <button
              className="btn btn-sm btn-primary"
              disabled={!hasObjectStateChanges || isSavingObjectStates}
              onClick={() => {
                void saveObjectStates();
              }}
            >
              {isSavingObjectStates ? <Loader2 size={14} className="animate-spin" /> : <Save size={14} />}
              Save
            </button>
          )}
          <button
            onClick={() => onPrimaryAction(collection)}
            disabled={primaryButtonDisabled}
            className={`btn btn-sm min-w-30 ${primaryButtonClass}`}
          >
            {primaryButtonContent}
          </button>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto custom-scrollbar p-4 bg-base-100/50">
        {groupedObjects.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-base-content/40">
            <Package size={48} className="mb-4 opacity-20" />
            <p>Collection is empty.</p>
          </div>
        ) : (
          <div className="max-w-3xl mx-auto">
            <ModGroupList
              groups={groupedObjects}
              colorClass="text-base-content/50"
              emptyGroupMessage="No mods in this object."
              onToggleObjectState={toggleObjectState}
              expansionMode={expansionMode}
              resetKey={collection.id}
            />
          </div>
        )}
      </div>
    </div>
  );
}
