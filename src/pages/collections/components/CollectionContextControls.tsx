import { ApplyCollectionModal } from './ApplyCollectionModal';
import { SaveCollectionModal } from './SaveCollectionModal';
import { useCollectionRuntimeDescriptor } from '../hooks/useCollectionRuntime';
import { useCollections } from '../hooks/useCollections';
import { getCollectionDisplayName, useRuntimeLabels } from '@/shared/lib/runtimeLabels';
import { useEffect, useState } from 'react';
import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';
import { useAppStore } from '@/app/store';
import { Save, Loader2, Layers } from 'lucide-react';
import { LiquidSurface } from '@/shared/ui/liquid';

export default function ContextControls() {
  const { t } = useTranslation('layout');
  const activeGameId = useAppStore((state) => state.activeGameId);
  const setWorkspaceView = useAppStore((state) => state.setWorkspaceView);
  const { data: collections = [], isLoading } = useCollections(activeGameId);
  const runtimeQuery = useCollectionRuntimeDescriptor(activeGameId);

  const [saveModalOpen, setSaveModalOpen] = useState(false);
  const [applyModalCollectionId, setApplyModalCollectionId] = useState<string | null>(null);
  const [moreMenuTarget, setMoreMenuTarget] = useState<HTMLElement | null>(null);

  const activeNamedCollectionId = runtimeQuery.data?.active_collection_id ?? null;
  const runtimeStatus = runtimeQuery.data?.runtime_status;
  const isDirty = runtimeStatus === 'modified' || runtimeStatus === 'unsaved';
  const runtimeLabels = useRuntimeLabels();
  const triggerText =
    activeGameId && runtimeQuery.status === 'pending'
      ? t('context.loading')
      : getCollectionDisplayName({
          name: isDirty ? null : runtimeQuery.data?.active_collection_name,
          isUnsaved: runtimeStatus === 'unsaved',
          labels: runtimeLabels,
        });

  useEffect(() => {
    setMoreMenuTarget(document.getElementById('topbar-more-collection-portal'));
  }, []);

  const handleApplyClick = (e: React.MouseEvent, id: string) => {
    e.stopPropagation();
    if (!activeGameId) return;
    if (activeNamedCollectionId === id) return;

    setApplyModalCollectionId(id);
    if (document.activeElement instanceof HTMLElement) {
      document.activeElement.blur();
    }
  };

  return (
    <>
      <div className="hidden items-center xl:flex">
        <div className="dropdown dropdown-bottom dropdown-end">
          <button
            type="button"
            tabIndex={0}
            className={`flex min-w-40 max-w-52 cursor-pointer flex-col items-center rounded-md border-0 bg-transparent px-2.5 py-1 text-center transition-[background-color,color] duration-150 hover:bg-base-content/5 ${activeNamedCollectionId === null && (collections || []).length > 0 ? 'italic opacity-90' : ''}`}
            title={`${t('context.active_collection', 'Active collection')}: ${triggerText}`}
            aria-label={`${t('context.active_collection', 'Active collection')}: ${triggerText}`}
          >
            <span className="text-[9px] font-semibold uppercase tracking-[0.16em] text-base-content/45">
              {t('nav.mods_manager')}
            </span>
            <span className="flex min-w-0 items-center justify-center gap-1.5 text-xs font-medium text-base-content/80">
              <Layers size={13} className="shrink-0 text-base-content/45" aria-hidden="true" />
              <span className="truncate">{triggerText}</span>
              <span className="shrink-0 text-[9px] text-muted">▼</span>
            </span>
          </button>
          <LiquidSurface
            liquidRole="overlay"
            className="dropdown-content z-[var(--workspace-layer-popover)] mt-2 w-56 rounded-box shadow-xl"
          >
            <ul tabIndex={0} className="menu w-full p-2">
              <li className="menu-title text-[10px] uppercase text-muted px-2 pb-1 tracking-widest flex justify-between items-center">
                <span>{t('context.collections_title')}</span>
              </li>

              <li>
                <button
                  className="hover:bg-primary/20 text-primary text-sm gap-2"
                  onClick={() => {
                    setSaveModalOpen(true);
                    const elem = document.activeElement as HTMLElement;
                    if (elem) elem.blur();
                  }}
                >
                  <Save size={14} />
                  {t('context.save_current')}
                </button>
              </li>

              <div className="divider my-1 before:bg-base-content/10 after:bg-base-content/10 mx-2"></div>

              {isLoading ? (
                <li className="disabled">
                  <span className="text-xs opacity-75 flex gap-2">
                    <Loader2 size={12} className="animate-spin" /> {t('context.loading')}
                  </span>
                </li>
              ) : (
                (() => {
                  return (collections || []).length === 0 ? (
                    <li className="disabled">
                      <span className="text-xs opacity-75 px-2">{t('context.no_collections')}</span>
                    </li>
                  ) : (
                    <div className="max-h-[30vh] overflow-y-auto pr-1 custom-scrollbar">
                      {collections.map((c) => (
                        <li key={c.id}>
                          <button
                            className={`text-sm justify-between ${
                              activeNamedCollectionId === c.id
                                ? 'bg-primary/10 text-primary font-medium cursor-default'
                                : 'hover:bg-base-content/10'
                            }`}
                            onClick={(e) => handleApplyClick(e, c.id)}
                            disabled={activeNamedCollectionId === c.id}
                          >
                            <span className="truncate max-w-32.5">{c.name}</span>
                            <span className="badge badge-xs badge-ghost opacity-75">
                              {c.mod_count}
                            </span>
                          </button>
                        </li>
                      ))}
                    </div>
                  );
                })()
              )}

              <div className="divider my-1 before:bg-base-content/10 after:bg-base-content/10 mx-2"></div>

              <li>
                <button
                  className="hover:bg-base-content/10 text-sm gap-2 text-base-content/70"
                  onClick={() => {
                    setWorkspaceView('collections');
                    const elem = document.activeElement as HTMLElement;
                    if (elem) elem.blur();
                  }}
                >
                  <Layers size={14} />
                  {t('context.manage_collections')}
                </button>
              </li>
            </ul>
          </LiquidSurface>
        </div>
      </div>

      {moreMenuTarget &&
        createPortal(
          <div className="!block w-full min-w-0 max-w-full border-b border-base-content/10 px-2 pb-2">
            <span className="mb-1.5 block text-[10px] font-medium uppercase tracking-widest text-base-content/45">
              {t('context.collections_title')}
            </span>

            <button
              type="button"
              className="flex min-h-9 w-full items-center gap-2 rounded-lg px-2.5 text-sm text-primary transition-colors hover:bg-primary/10"
              onClick={(event) => {
                setSaveModalOpen(true);
                event.currentTarget.blur();
              }}
            >
              <Save size={14} aria-hidden="true" />
              {t('context.save_current')}
            </button>

            {isLoading ? (
              <div className="flex min-h-9 items-center gap-2 px-2.5 text-xs text-base-content/50">
                <Loader2 size={13} className="animate-spin motion-reduce:animate-none" />
                {t('context.loading')}
              </div>
            ) : collections.length === 0 ? (
              <div className="px-2.5 py-2 text-xs text-base-content/50">
                {t('context.no_collections')}
              </div>
            ) : (
              <div className="max-h-40 overflow-y-auto py-1">
                {collections.map((collection) => {
                  const isActive = activeNamedCollectionId === collection.id;

                  return (
                    <button
                      key={collection.id}
                      type="button"
                      className={`flex min-h-9 w-full items-center justify-between gap-2 rounded-lg px-2.5 text-left text-sm transition-colors ${
                        isActive
                          ? 'bg-base-content/5 font-medium text-base-content'
                          : 'text-base-content/70 hover:bg-base-content/10 hover:text-base-content'
                      }`}
                      disabled={isActive}
                      aria-current={isActive ? 'true' : undefined}
                      onClick={(event) => handleApplyClick(event, collection.id)}
                    >
                      <span className="truncate">{collection.name}</span>
                      <span className="shrink-0 font-mono text-xs tabular-nums text-base-content/45">
                        {collection.mod_count}
                      </span>
                    </button>
                  );
                })}
              </div>
            )}

            <button
              type="button"
              className="flex min-h-9 w-full items-center gap-2 rounded-lg px-2.5 text-sm text-base-content/70 transition-colors hover:bg-base-content/10 hover:text-base-content"
              onClick={(event) => {
                setWorkspaceView('collections');
                event.currentTarget.blur();
              }}
            >
              <Layers size={14} aria-hidden="true" />
              {t('context.manage_collections')}
            </button>
          </div>,
          moreMenuTarget,
        )}

      {saveModalOpen && <SaveCollectionModal onClose={() => setSaveModalOpen(false)} />}
      {applyModalCollectionId && (
        <ApplyCollectionModal
          collectionId={applyModalCollectionId}
          onClose={() => setApplyModalCollectionId(null)}
        />
      )}
    </>
  );
}
