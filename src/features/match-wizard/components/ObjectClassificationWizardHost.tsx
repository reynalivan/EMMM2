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
          <span className="label-text">{t('classification_disable_after_apply')}</span>
        </label>
        <div className="mt-3 flex justify-end">
          <button className="btn btn-sm" disabled={busy} onClick={selectAllHighConfidence}>
            {t('classification_select_high_confidence')}
          </button>
        </div>
        <div className="overflow-auto flex-1 mt-3 border border-base-300 rounded-box">
          <table className="table table-sm table-pin-rows">
            <thead>
              <tr>
                <th>{t('classification_select')}</th>
                <th>{t('columns.source')}</th>
                <th>{t('columns.canonical')}</th>
                <th>{t('columns.confidence')}</th>
              </tr>
            </thead>
            <tbody>
              {items.map((item) => {
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
                  <tr key={item.objectId}>
                    <td>
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
                    </td>
                    <td className="min-w-56">
                      <div className="font-medium">{item.objectName}</div>
                      <div className="text-xs opacity-50">{item.currentCategory}</div>
                      <div
                        className="mt-1 max-w-64 truncate text-xs opacity-50"
                        title={item.sourcePath}
                      >
                        {item.sourcePath}
                      </div>
                    </td>
                    <td className="min-w-96">
                      {draft?.mode === 'manual' ? (
                        <div className="space-y-2">
                          <div className="flex items-center justify-between gap-2">
                            <span className="text-sm font-medium">
                              {t('classification_manual')}
                            </span>
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
                          <select
                            aria-label={t('columns.category')}
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
                              <option key={category}>{category}</option>
                            ))}
                          </select>
                          {categoryDef?.subcategories && categoryDef.subcategories.length > 0 && (
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
                            disabled={busy}
                            emptyLabel={t('classification_no_catalog_match')}
                            entries={catalog}
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
                                {selectedEntry.category}
                              </span>
                            )}
                            {selectedEntry &&
                              metadataChips(selectedEntry, schema).map((chip) => (
                                <span className="badge badge-ghost badge-sm" key={chip.label}>
                                  {chip.label}: {chip.value}
                                </span>
                              ))}
                          </div>
                          <button
                            className="btn btn-ghost btn-xs"
                            disabled={busy}
                            type="button"
                            onClick={() =>
                              updateDraft(item.objectId, (current) => ({
                                ...current,
                                mode: 'manual',
                                selected: false,
                              }))
                            }
                          >
                            {t('classification_manual')}
                          </button>
                        </div>
                      )}
                    </td>
                    <td className="min-w-56">
                      {selectedSuggestion ? (
                        <>
                          <div>
                            {t('classification_confidence_tier', {
                              score: selectedSuggestion.confidencePercentage,
                              tier: selectedSuggestion.confidenceTier,
                            })}
                          </div>
                          <details className="mt-1 text-xs text-info">
                            <summary>{t('classification_why_suggested')}</summary>
                            <ul className="mt-1 list-disc pl-4 text-base-content">
                              {selectedSuggestion.evidence.map((evidence) => (
                                <li key={`${evidence.source}-${evidence.value}`}>
                                  {evidence.value}
                                </li>
                              ))}
                            </ul>
                          </details>
                        </>
                      ) : (
                        '-'
                      )}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
        <div className="modal-action">
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
      </div>
    </dialog>
  );
}
