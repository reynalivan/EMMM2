import { useCallback, useEffect, useMemo, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { commands } from '../../../shared/api/tauri/bindings';
import type {
  JsonValue,
  GameSchema,
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

const CATEGORIES: StableCategory[] = ['Character', 'Weapon', 'UI', 'Other'];

type Draft = {
  category: StableCategory;
  subCategory: string | null;
  metadata: JsonValue;
  canonicalIndex: number | null;
};

export function ObjectClassificationWizardHost() {
  const { t } = useTranslation(['match_wizard', 'common']);
  const queryClient = useQueryClient();
  const [request, setRequest] = useState<ObjectClassificationLaunchRequest | null>(null);
  const [items, setItems] = useState<ObjectClassificationPreviewItem[]>([]);
  const [drafts, setDrafts] = useState<Record<string, Draft>>({});
  const [phase, setPhase] = useState<'category' | 'canonical'>('category');
  const [busy, setBusy] = useState(false);
  const [schema, setSchema] = useState<GameSchema | null>(null);
  const [disableAfterApply, setDisableAfterApply] = useState(false);

  const launch = useCallback(
    async (next: ObjectClassificationLaunchRequest) => {
      setBusy(true);
      setRequest(next);
      setPhase('category');
      setDisableAfterApply(false);
      try {
        const preview =
          next.initialItems ??
          (await commands.previewObjectClassificationBatch({
            gameId: next.gameId,
            objectIds: next.objectIds,
            drafts: [],
          }));
        const games = await commands.getGames();
        const game = games.find((candidate) => candidate.id === next.gameId);
        setSchema(game ? await commands.getGameSchema(game.game_type) : null);
        setItems(preview);
        setDrafts(
          Object.fromEntries(
            preview.map((item) => {
              const suggestion = item.categorySuggestions[0];
              return [
                item.objectId,
                {
                  category: suggestion?.category ?? 'Other',
                  subCategory: suggestion?.subCategory ?? null,
                  metadata: suggestion?.metadata ?? {},
                  canonicalIndex: null,
                } satisfies Draft,
              ];
            }),
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

  const categoryReady = useMemo(
    () => items.length > 0 && items.every((item) => drafts[item.objectId]),
    [drafts, items],
  );

  if (!request) return null;

  const refreshCanonical = async () => {
    setBusy(true);
    try {
      const preview = await commands.previewObjectClassificationBatch({
        gameId: request.gameId,
        objectIds: request.objectIds,
        drafts: items.map((item) => ({
          objectId: item.objectId,
          category: drafts[item.objectId].category,
          subCategory: drafts[item.objectId].subCategory,
          metadata: drafts[item.objectId].metadata,
        })),
      });
      setItems(preview);
      setDrafts((current) =>
        Object.fromEntries(
          preview.map((item) => [
            item.objectId,
            {
              ...current[item.objectId],
              canonicalIndex: item.canonicalSuggestions.length > 0 ? 0 : null,
            },
          ]),
        ),
      );
      setPhase('canonical');
    } catch (error) {
      toast.error(t('errors.update', { error: formatAppError(error) }));
    } finally {
      setBusy(false);
    }
  };

  const apply = async () => {
    setBusy(true);
    try {
      const result = await commands.applyObjectClassificationBatch({
        gameId: request.gameId,
        disableAfterApply,
        items: items.map((item) => {
          const draft = drafts[item.objectId];
          const canonical =
            draft.canonicalIndex === null
              ? null
              : (item.canonicalSuggestions[draft.canonicalIndex] ?? null);
          return {
            objectId: item.objectId,
            category: draft.category,
            subCategory: draft.subCategory,
            metadata: draft.metadata,
            canonicalEntryKey: canonical?.entryKey ?? null,
            canonicalAlias: canonical?.matchedAlias ?? null,
            confidencePercentage: canonical?.confidencePercentage ?? null,
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
      <div className="modal-box max-w-5xl h-[82vh] flex flex-col">
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
        <div className="overflow-auto flex-1 mt-4 border border-base-300 rounded-box">
          <table className="table table-sm table-pin-rows">
            <thead>
              <tr>
                <th>{t('columns.source')}</th>
                <th>{t('columns.category')}</th>
                <th>{t('columns.canonical')}</th>
                <th>{t('columns.confidence')}</th>
              </tr>
            </thead>
            <tbody>
              {items.map((item) => {
                const draft = drafts[item.objectId];
                const categoryDef = schema?.categories.find(
                  (category) => category.name === draft?.category,
                );
                const metadata =
                  typeof draft?.metadata === 'object' &&
                  draft.metadata !== null &&
                  !Array.isArray(draft.metadata)
                    ? draft.metadata
                    : {};
                return (
                  <tr key={item.objectId}>
                    <td>
                      <div className="font-medium">{item.objectName}</div>
                      <div className="text-xs opacity-50">{item.currentCategory}</div>
                    </td>
                    <td>
                      <select
                        className="select select-bordered select-sm"
                        value={draft?.category ?? 'Other'}
                        disabled={phase !== 'category' || busy}
                        onChange={(event) => {
                          const category = event.target.value as StableCategory;
                          const suggestion = item.categorySuggestions.find(
                            (candidate) => candidate.category === category,
                          );
                          setDrafts((current) => ({
                            ...current,
                            [item.objectId]: {
                              category,
                              subCategory: suggestion?.subCategory ?? null,
                              metadata: suggestion?.metadata ?? {},
                              canonicalIndex: null,
                            },
                          }));
                        }}
                      >
                        {CATEGORIES.map((category) => (
                          <option key={category}>{category}</option>
                        ))}
                      </select>
                      {categoryDef?.subcategories && categoryDef.subcategories.length > 0 && (
                        <select
                          className="select select-bordered select-xs mt-1 w-full"
                          value={draft?.subCategory ?? ''}
                          disabled={phase !== 'category' || busy}
                          onChange={(event) =>
                            setDrafts((current) => ({
                              ...current,
                              [item.objectId]: {
                                ...current[item.objectId],
                                subCategory: event.target.value || null,
                              },
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
                        <label className="block mt-1" key={filter.key}>
                          <span className="text-xs opacity-60">{filter.label}</span>
                          <select
                            className="select select-bordered select-xs w-full"
                            value={String(metadata[filter.key] ?? '')}
                            disabled={phase !== 'category' || busy}
                            onChange={(event) =>
                              setDrafts((current) => ({
                                ...current,
                                [item.objectId]: {
                                  ...current[item.objectId],
                                  metadata: {
                                    ...metadata,
                                    [filter.key]: event.target.value,
                                  },
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
                    </td>
                    <td>
                      {phase === 'canonical' ? (
                        <select
                          className="select select-bordered select-sm max-w-60"
                          value={draft?.canonicalIndex ?? ''}
                          onChange={(event) =>
                            setDrafts((current) => ({
                              ...current,
                              [item.objectId]: {
                                ...current[item.objectId],
                                canonicalIndex:
                                  event.target.value === '' ? null : Number(event.target.value),
                              },
                            }))
                          }
                        >
                          <option value="">{t('no_match')}</option>
                          {item.canonicalSuggestions.map((suggestion, index) => (
                            <option key={suggestion.entryKey} value={index}>
                              {suggestion.name}
                            </option>
                          ))}
                        </select>
                      ) : (
                        t('classification_after_category')
                      )}
                    </td>
                    <td>
                      {draft?.canonicalIndex !== null &&
                        item.canonicalSuggestions[draft.canonicalIndex]?.confidencePercentage}
                      {draft?.canonicalIndex !== null ? '%' : '—'}
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
          {phase === 'category' ? (
            <button
              className="btn btn-primary"
              disabled={!categoryReady || busy}
              onClick={() => void refreshCanonical()}
            >
              {t('classification_match_objects')}
            </button>
          ) : (
            <button className="btn btn-primary" disabled={busy} onClick={() => void apply()}>
              {t('classification_apply')}
            </button>
          )}
        </div>
      </div>
    </dialog>
  );
}
