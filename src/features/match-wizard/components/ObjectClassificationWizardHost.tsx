import { useCallback, useEffect, useMemo, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { commands } from '../../../shared/api/tauri/bindings';
import type {
  CanonicalClassificationCatalogEntry,
  JsonValue,
  GameSchema,
  ObjectClassificationDecision,
  ObjectClassificationPreviewItem,
  StableCategory,
} from '../../../shared/api/tauri/bindings.gen';
import { formatAppError } from '../../../shared/lib/appError';
import { toast } from '@/shared/ui/toast';
import VirtualList from '@/shared/ui/components/ui/VirtualList';
import { publishQueryScopes } from '@/shared/lib/queryRefresh';
import {
  subscribeObjectClassificationWizard,
  type ObjectClassificationLaunchRequest,
} from '@/features/import-batches/@x/match-wizard';
import { CanonicalObjectCombobox } from './CanonicalObjectCombobox';

const CATEGORIES: StableCategory[] = ['Character', 'Weapon', 'UI', 'Other'];

type Draft = {
  selected: boolean;
  mode: 'canonical' | 'manual';
  canonicalEntryKey: string | null;
  category: StableCategory;
  subCategory: string | null;
  metadata: JsonValue;
};

function isJsonObject(value: JsonValue): value is Record<string, JsonValue> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function metadataChips(entry: CanonicalClassificationCatalogEntry, schema: GameSchema | null) {
  const entryMetadata = entry.metadata;
  if (!isJsonObject(entryMetadata)) return [];
  const category = schema?.categories.find((candidate) => candidate.name === entry.category);
  const configured =
    category?.filters
      ?.map((filter) => ({ label: filter.label, value: entryMetadata[filter.key] }))
      .filter((item): item is { label: string; value: JsonValue } => item.value !== undefined) ??
    [];
  if (configured.length > 0) {
    return configured.map((item) => ({ label: item.label, value: String(item.value) }));
  }
  return Object.entries(entryMetadata)
    .filter(([, value]) => ['string', 'number', 'boolean'].includes(typeof value))
    .slice(0, 3)
    .map(([label, value]) => ({ label, value: String(value) }));
}

function canonicalDecision(entryKey: string): ObjectClassificationDecision {
  return { kind: 'canonical', entryKey };
}

function manualDecision(draft: Draft): ObjectClassificationDecision {
  return {
    kind: 'manual',
    category: draft.category,
    subCategory: draft.subCategory,
    metadata: draft.metadata,
  };
}

export function ObjectClassificationWizardHost() {
  const { t } = useTranslation(['match_wizard', 'common']);
  const queryClient = useQueryClient();
  const [request, setRequest] = useState<ObjectClassificationLaunchRequest | null>(null);
  const [items, setItems] = useState<ObjectClassificationPreviewItem[]>([]);
  const [catalog, setCatalog] = useState<CanonicalClassificationCatalogEntry[]>([]);
  const [drafts, setDrafts] = useState<Record<string, Draft>>({});
  const [busy, setBusy] = useState(false);
  const [schema, setSchema] = useState<GameSchema | null>(null);
  const [disableAfterApply, setDisableAfterApply] = useState(false);

  const launch = useCallback(
    async (next: ObjectClassificationLaunchRequest) => {
      setBusy(true);
      setRequest(next);
      setDisableAfterApply(false);
      try {
        const [preview, entries, games] = await Promise.all([
          next.initialItems ??
            commands.previewObjectClassificationBatch({
              gameId: next.gameId,
              objectIds: next.objectIds,
            }),
          commands.listCanonicalClassificationCatalog(next.gameId),
          commands.getGames(),
        ]);
        const game = games.find((candidate) => candidate.id === next.gameId);
        setSchema(game ? await commands.getGameSchema(game.game_type) : null);
        const catalogKeys = new Set(entries.map((entry) => entry.entryKey));
        setItems(preview);
        setCatalog(entries);
        setDrafts(
          Object.fromEntries(
            preview.map((item) => [
              item.objectId,
              {
                selected: false,
                mode: 'canonical',
                canonicalEntryKey:
                  item.canonicalSuggestions.find((suggestion) =>
                    catalogKeys.has(suggestion.entryKey),
                  )?.entryKey ?? null,
                category: 'Other',
                subCategory: null,
                metadata: {},
              } satisfies Draft,
            ]),
          ),
        );
      } catch (error) {
        toast.error(t('errors.launch', { error: formatAppError(error) }));
        setRequest(null);
        next.onComplete?.('cancelled');
      } finally {
        setBusy(false);
      }
    },
    [t],
  );

  useEffect(() => subscribeObjectClassificationWizard((next) => void launch(next)), [launch]);

  const selectedItems = useMemo(
    () =>
      items.filter((item) => {
        const draft = drafts[item.objectId];
        return draft?.selected && (draft.mode === 'manual' || draft.canonicalEntryKey !== null);
      }),
    [drafts, items],
  );
  const categoryLabel = (category: string) =>
    t(`categories.${category}`, { defaultValue: category });

  if (!request) return null;

  const updateDraft = (objectId: string, update: (current: Draft) => Draft) => {
    setDrafts((current) => ({
      ...current,
      [objectId]: update(current[objectId]),
    }));
  };

  const selectAllHighConfidence = () => {
    const catalogKeys = new Set(catalog.map((entry) => entry.entryKey));
    setDrafts((current) =>
      Object.fromEntries(
        items.map((item) => {
          const suggestion = item.canonicalSuggestions[0];
          if (
            suggestion?.confidenceTier === 'high' &&
            suggestion.matchStatus === 'auto_matched' &&
            catalogKeys.has(suggestion.entryKey)
          ) {
            return [
              item.objectId,
              {
                ...current[item.objectId],
                selected: true,
                mode: 'canonical',
                canonicalEntryKey: suggestion.entryKey,
              } satisfies Draft,
            ];
          }
          return [item.objectId, current[item.objectId]];
        }),
      ),
    );
  };

  const apply = async () => {
    setBusy(true);
    try {
      const result = await commands.applyObjectClassificationBatch({
        gameId: request.gameId,
        disableAfterApply,
        items: selectedItems.map((item) => {
          const draft = drafts[item.objectId];
          return {
            objectId: item.objectId,
            decision:
              draft.mode === 'canonical' && draft.canonicalEntryKey
                ? canonicalDecision(draft.canonicalEntryKey)
                : manualDecision(draft),
            fingerprint: item.fingerprint,
          };
        }),
      });
      await publishQueryScopes(queryClient, ['workspaceViewModel', 'objectRows', 'folderMetadata']);
      toast.success(
        t('classification_applied', {
          count: result.applied,
          mods: result.childModsUpdated,
        }),
      );
      if (result.disableWarning) toast.warning(result.disableWarning);
      const onComplete = request.onComplete;
      setRequest(null);
      onComplete?.('applied');
    } catch (error) {
      toast.error(t('errors.commit', { error: formatAppError(error) }));
    } finally {
      setBusy(false);
    }
  };

  const ignoreSuggestion = async (objectId: string) => {
    if (!request?.onIgnore) return;
    setBusy(true);
    try {
      await request.onIgnore(objectId);
      setItems((current) => current.filter((item) => item.objectId !== objectId));
      setDrafts((current) => {
        const next = { ...current };
        delete next[objectId];
        return next;
      });
    } catch (error) {
      toast.error(t('errors.commit', { error: formatAppError(error) }));
    } finally {
      setBusy(false);
    }
  };

  return (
    <dialog open className="modal modal-open" aria-label={t('classification_title')}>
      <div className="modal-box max-w-6xl h-[82vh] flex flex-col">
        <h2 className="text-xl font-bold">{t('classification_title')}</h2>
        <p className="text-sm opacity-60">{t('classification_no_move')}</p>
        <label className="label cursor-pointer justify-start gap-3 mt-2 rounded-lg bg-base-200 px-3">
          <input
            type="checkbox"
            className="checkbox checkbox-sm"
            checked={disableAfterApply}
            disabled={busy}
            onChange={(event) => setDisableAfterApply(event.target.checked)}
          />
          <span>
            <span className="label-text block">{t('classification_disable_after_apply')}</span>
            <span className="text-xs text-base-content/60">{t('classification_disable_help')}</span>
          </span>
        </label>
        <div className="mt-3 flex justify-end">
          <button className="btn btn-sm" disabled={busy} onClick={selectAllHighConfidence}>
            {t('classification_select_high_confidence')}
          </button>
        </div>
        <div className="mt-3 flex min-h-0 flex-1 flex-col overflow-hidden rounded-box border border-base-300">
          <div
            className="grid grid-cols-[3.5rem_minmax(14rem,1fr)_minmax(22rem,1.7fr)_minmax(14rem,1fr)] gap-3 border-b border-base-300 bg-base-200 px-3 py-2 text-sm font-medium text-base-content/70"
            role="row"
          >
            <div role="columnheader">{t('classification_select')}</div>
            <div role="columnheader">{t('columns.source')}</div>
            <div role="columnheader">{t('columns.canonical')}</div>
            <div role="columnheader">{t('columns.confidence')}</div>
          </div>
          <VirtualList
            ariaLabel={t('classification_title')}
            className="min-h-0 flex-1"
            contentClassName="p-0"
            estimateSize={() => 188}
            getItemKey={(item) => item.objectId}
            items={items}
            renderItem={(item) => {
              const draft = drafts[item.objectId];
              const selectedEntry = catalog.find(
                (entry) => entry.entryKey === draft?.canonicalEntryKey,
              );
              const selectedSuggestion = item.canonicalSuggestions.find(
                (suggestion) => suggestion.entryKey === selectedEntry?.entryKey,
              );
              const categoryDef = schema?.categories.find(
                (category) => category.name === draft?.category,
              );
              const draftMetadata = draft?.metadata ?? null;
              const metadata = isJsonObject(draftMetadata) ? draftMetadata : {};
              const canSelect = draft?.mode === 'manual' || selectedEntry !== undefined;
              return (
                <div
                  className="grid grid-cols-[3.5rem_minmax(14rem,1fr)_minmax(22rem,1.7fr)_minmax(14rem,1fr)] gap-3 border-b border-base-300 px-3 py-3"
                  role="row"
                >
                  <div role="cell">
                    <input
                      aria-label={t('classification_select_row', { name: item.objectName })}
                      checked={draft?.selected ?? false}
                      className="checkbox checkbox-sm"
                      disabled={busy || !canSelect}
                      type="checkbox"
                      onChange={(event) =>
                        updateDraft(item.objectId, (current) => ({
                          ...current,
                          selected: event.target.checked,
                        }))
                      }
                    />
                  </div>
                  <div className="min-w-0" role="cell">
                    <div className="font-medium">{item.objectName}</div>
                    <div className="text-xs opacity-50">{categoryLabel(item.currentCategory)}</div>
                    <div
                      className="mt-1 max-w-64 truncate text-xs opacity-50"
                      title={item.sourcePath}
                    >
                      {item.sourcePath}
                    </div>
                    {request.onIgnore && (
                      <button
                        className="btn btn-ghost btn-xs mt-2"
                        disabled={busy}
                        type="button"
                        onClick={() => void ignoreSuggestion(item.objectId)}
                      >
                        {t('classification_ignore_suggestion')}
                      </button>
                    )}
                  </div>
                  <div className="min-w-0" role="cell">
                    {draft?.mode === 'manual' ? (
                      <div className="space-y-2">
                        <div className="flex items-center justify-between gap-2">
                          <span className="text-sm font-medium">{t('classification_manual')}</span>
                          <button
                            className="btn btn-ghost btn-xs"
                            disabled={busy}
                            type="button"
                            onClick={() =>
                              updateDraft(item.objectId, (current) => ({
                                ...current,
                                mode: 'canonical',
                              }))
                            }
                          >
                            {t('classification_use_catalog')}
                          </button>
                        </div>
                        <label className="block">
                          <span className="text-xs text-base-content/60">
                            {t('classification_object_type')}
                          </span>
                          <select
                            aria-label={t('classification_object_type')}
                            className="select select-bordered select-sm w-full"
                            disabled={busy}
                            value={draft.category}
                            onChange={(event) => {
                              const category = event.target.value as StableCategory;
                              updateDraft(item.objectId, (current) => ({
                                ...current,
                                category,
                                subCategory: null,
                                metadata: {},
                              }));
                            }}
                          >
                            {CATEGORIES.map((category) => (
                              <option key={category} value={category}>
                                {categoryLabel(category)}
                              </option>
                            ))}
                          </select>
                        </label>
                        {categoryDef?.subcategories && categoryDef.subcategories.length > 0 && (
                          <label className="block">
                            <span className="text-xs text-base-content/60">
                              {t('classification_subcategory')}
                            </span>
                            <select
                              aria-label={t('classification_subcategory')}
                              className="select select-bordered select-xs w-full"
                              disabled={busy}
                              value={draft.subCategory ?? ''}
                              onChange={(event) =>
                                updateDraft(item.objectId, (current) => ({
                                  ...current,
                                  subCategory: event.target.value || null,
                                }))
                              }
                            >
                              <option value="" />
                              {categoryDef.subcategories.map((subcategory) => (
                                <option key={subcategory}>{subcategory}</option>
                              ))}
                            </select>
                          </label>
                        )}
                        {categoryDef?.filters?.map((filter) => (
                          <label className="block" key={filter.key}>
                            <span className="text-xs opacity-60">{filter.label}</span>
                            <select
                              className="select select-bordered select-xs w-full"
                              disabled={busy}
                              value={String(metadata[filter.key] ?? '')}
                              onChange={(event) =>
                                updateDraft(item.objectId, (current) => ({
                                  ...current,
                                  metadata: {
                                    ...metadata,
                                    [filter.key]: event.target.value,
                                  },
                                }))
                              }
                            >
                              <option value="" />
                              {filter.options.map((option) => (
                                <option key={option}>{option}</option>
                              ))}
                            </select>
                          </label>
                        ))}
                      </div>
                    ) : (
                      <div className="space-y-2">
                        <CanonicalObjectCombobox
                          ariaLabel={t('classification_catalog_search_label')}
                          categoryLabel={categoryLabel}
                          disabled={busy}
                          emptyLabel={t('classification_no_catalog_match')}
                          entries={catalog}
                          manualOptionHint={t('classification_catalog_manual_hint')}
                          manualOptionLabel={t('classification_manual')}
                          onSelectManual={() =>
                            updateDraft(item.objectId, (current) => ({
                              ...current,
                              mode: 'manual',
                              selected: false,
                            }))
                          }
                          searchPlaceholder={t('classification_catalog_search_placeholder')}
                          selectedEntryKey={draft?.canonicalEntryKey ?? null}
                          suggestions={item.canonicalSuggestions}
                          onSelect={(entryKey) =>
                            updateDraft(item.objectId, (current) => ({
                              ...current,
                              mode: 'canonical',
                              canonicalEntryKey: entryKey,
                              selected: true,
                            }))
                          }
                        />
                        <div className="flex flex-wrap gap-1">
                          {selectedEntry && (
                            <span className="badge badge-outline badge-sm">
                              {categoryLabel(selectedEntry.category)}
                            </span>
                          )}
                          {selectedEntry &&
                            metadataChips(selectedEntry, schema).map((chip) => (
                              <span className="badge badge-ghost badge-sm" key={chip.label}>
                                {chip.label}: {chip.value}
                              </span>
                            ))}
                        </div>
                      </div>
                    )}
                  </div>
                  <div className="min-w-0" role="cell">
                    {selectedSuggestion ? (
                      <>
                        <div>
                          {t('classification_confidence_tier', {
                            score: selectedSuggestion.confidencePercentage,
                            tier: t(`confidence.${selectedSuggestion.confidenceTier}`),
                          })}
                        </div>
                        <details className="mt-1 text-xs text-info">
                          <summary>{t('classification_why_suggested')}</summary>
                          <ul className="mt-1 list-disc pl-4 text-base-content">
                            {selectedSuggestion.evidence.map((evidence) => (
                              <li key={`${evidence.source}-${evidence.value}`}>{evidence.value}</li>
                            ))}
                          </ul>
                        </details>
                      </>
                    ) : (
                      '-'
                    )}
                  </div>
                </div>
              );
            }}
          />
        </div>
        <footer className="-mx-6 mt-4 flex shrink-0 flex-col gap-3 border-t border-base-300 bg-base-100 px-6 py-4 sm:flex-row sm:items-center sm:justify-between">
          <p aria-live="polite" className="text-sm text-base-content/65">
            {selectedItems.length === 0
              ? t('classification_footer_pending')
              : t('classification_footer_ready', { count: selectedItems.length })}
          </p>
          <div className="flex justify-end gap-2">
            <button
              className="btn btn-ghost"
              disabled={busy}
              onClick={() => {
                const onComplete = request.onComplete;
                setRequest(null);
                onComplete?.('cancelled');
              }}
            >
              {t('common:actions.cancel')}
            </button>
            <button
              className="btn btn-primary"
              disabled={busy || selectedItems.length === 0}
              onClick={() => void apply()}
            >
              {t('classification_apply_selected', { count: selectedItems.length })}
            </button>
          </div>
        </footer>
      </div>
    </dialog>
  );
}
