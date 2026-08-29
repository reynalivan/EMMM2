import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type {
  DestinationSuggestion,
  GameSchema,
  ImportBatch,
  ImportDecision,
  ImportItem,
  JsonValue,
  StableCategory,
} from '../../lib/bindings.gen';
import type { ObjectSummary } from '../../types/object';
import { destinationDecision } from './importBatchDecision';

const CATEGORIES: StableCategory[] = ['Character', 'Weapon', 'UI', 'Other'];

type Props = {
  batch: ImportBatch;
  item: ImportItem;
  schema: GameSchema | null;
  objects: ObjectSummary[];
  busy: boolean;
  onClassify: (
    item: ImportItem,
    category: StableCategory,
    subCategory: string | null,
    metadata: JsonValue,
  ) => Promise<void>;
  onChooseDestination: (
    item: ImportItem,
    suggestion: DestinationSuggestion,
    decision: ImportDecision,
  ) => Promise<void>;
  onChooseManualTarget: (item: ImportItem, objectId: string) => Promise<void>;
  onSkip: (item: ImportItem) => Promise<void>;
  onRename: (item: ImportItem, plannedName: string) => Promise<void>;
  onRetry: (item: ImportItem) => Promise<void>;
  onOpenInExplorer: (item: ImportItem) => Promise<void>;
};

function suggestedMetadata(item: ImportItem, category: StableCategory): JsonValue {
  return (
    item.categorySuggestions.find((suggestion) => suggestion.category === category)?.metadata ?? {}
  );
}

export function ImportBatchWizardItemRow({
  batch,
  busy,
  item,
  objects,
  onChooseDestination,
  onChooseManualTarget,
  onClassify,
  onOpenInExplorer,
  onRename,
  onRetry,
  onSkip,
  schema,
}: Props) {
  const { t } = useTranslation('match_wizard');
  const [manualTarget, setManualTarget] = useState('');
  const [plannedName, setPlannedName] = useState(item.plannedName);
  const [category, setCategory] = useState<StableCategory>(
    item.matchCategory ?? item.categorySuggestions[0]?.category ?? 'Other',
  );
  const [subcategory, setSubcategory] = useState(
    item.categorySuggestions.find((suggestion) => suggestion.category === category)?.subCategory ??
      '',
  );
  const [metadata, setMetadata] = useState<Record<string, string>>({});
  const categoryDef = schema?.categories.find((candidate) => candidate.name === category);
  const suggested = suggestedMetadata(item, category);
  const suggestedObject =
    typeof suggested === 'object' && suggested !== null && !Array.isArray(suggested)
      ? suggested
      : {};
  const canChooseDestination = item.status === 'awaiting_destination' || item.status === 'skipped';
  const canEditPlan = [
    'discovered',
    'staged',
    'awaiting_category',
    'awaiting_destination',
    'ready',
    'skipped',
    'failed',
  ].includes(item.status);

  return (
    <tr>
      <td className="min-w-52">
        <div className="font-medium">{item.plannedName}</div>
        <div className="text-xs opacity-50 max-w-56 truncate" title={item.sourcePath}>
          {item.sourcePath}
        </div>
        <div className="join mt-1">
          <input
            className="input input-bordered input-xs join-item w-36"
            value={plannedName}
            onChange={(event) => setPlannedName(event.target.value)}
          />
          <button
            className="btn btn-xs join-item"
            disabled={!canEditPlan || busy || plannedName === item.plannedName}
            onClick={() => void onRename(item, plannedName)}
          >
            {t('actions.rename')}
          </button>
        </div>
      </td>
      <td className="min-w-44">
        {item.status === 'awaiting_category' ? (
          <>
            <div className="join">
              <select
                className="select select-bordered select-xs join-item"
                value={category}
                onChange={(event) => {
                  setCategory(event.target.value as StableCategory);
                  setSubcategory('');
                  setMetadata({});
                }}
              >
                {CATEGORIES.map((candidate) => (
                  <option key={candidate}>{candidate}</option>
                ))}
              </select>
              {categoryDef?.subcategories && categoryDef.subcategories.length > 0 && (
                <select
                  className="select select-bordered select-xs join-item"
                  value={subcategory}
                  onChange={(event) => setSubcategory(event.target.value)}
                >
                  <option value="" />
                  {categoryDef.subcategories.map((candidate) => (
                    <option key={candidate}>{candidate}</option>
                  ))}
                </select>
              )}
              <button
                className="btn btn-primary btn-xs join-item"
                disabled={busy}
                onClick={() =>
                  void onClassify(item, category, subcategory || null, {
                    ...suggestedObject,
                    ...metadata,
                  })
                }
              >
                {t('actions.confirm')}
              </button>
            </div>
            {categoryDef?.filters?.map((filter) => (
              <label className="block mt-1" key={filter.key}>
                <span className="text-xs opacity-60">{filter.label}</span>
                <select
                  className="select select-bordered select-xs w-full"
                  value={metadata[filter.key] ?? String(suggestedObject[filter.key] ?? '')}
                  onChange={(event) =>
                    setMetadata((current) => ({
                      ...current,
                      [filter.key]: event.target.value,
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
          </>
        ) : (
          <>
            <span className="badge badge-outline">{item.matchCategory}</span>
            {item.subCategory && <span className="badge badge-ghost ml-1">{item.subCategory}</span>}
          </>
        )}
      </td>
      <td>
        {item.canonicalSuggestions[0]?.name ?? t('no_match')}
        {item.canonicalSuggestions[0]?.matchedAlias && (
          <div className="text-xs opacity-60">{item.canonicalSuggestions[0].matchedAlias}</div>
        )}
      </td>
      <td className="min-w-64">
        {canChooseDestination ? (
          <div className="space-y-1">
            {item.destinationSuggestions.map((suggestion) => (
              <button
                key={[suggestion.kind, suggestion.objectId ?? suggestion.canonicalEntryKey].join(
                  ':',
                )}
                className="btn btn-ghost btn-xs justify-start w-full"
                disabled={busy}
                onClick={() =>
                  void onChooseDestination(item, suggestion, destinationDecision(batch, suggestion))
                }
                title={[suggestion.targetPath, suggestion.warning].filter(Boolean).join('\n')}
              >
                {suggestion.folderName}
                {suggestion.warning
                  ? ` — ${suggestion.warning}`
                  : suggestion.kind === 'specific_target' &&
                    suggestion.confidenceTier !== 'high' &&
                    ' — ' + t('specific_warning')}
              </button>
            ))}
            <div className="join w-full">
              <select
                className="select select-bordered select-xs join-item flex-1"
                value={manualTarget}
                onChange={(event) => setManualTarget(event.target.value)}
              >
                <option value="">{t('choose_existing')}</option>
                {objects.map((object) => (
                  <option key={object.id} value={object.id}>
                    {object.name}
                  </option>
                ))}
              </select>
              <button
                className="btn btn-xs join-item"
                disabled={!manualTarget || busy}
                onClick={() => void onChooseManualTarget(item, manualTarget)}
              >
                {t('actions.use_target')}
              </button>
            </div>
          </div>
        ) : (
          <span title={item.destinationPath ?? undefined}>
            {item.destinationPath ?? t('no_destination')}
          </span>
        )}
      </td>
      <td>
        <span className="badge badge-outline">
          {item.confidencePercentage}% {item.confidenceTier}
        </span>
        {item.evidence.length > 0 && (
          <details className="text-xs mt-1">
            <summary>{t('evidence')}</summary>
            {item.evidence.map((evidence, index) => (
              <div key={[evidence.source, index].join(':')}>
                {evidence.source}: {evidence.value}
              </div>
            ))}
          </details>
        )}
      </td>
      <td>
        <span className="badge badge-outline">{item.decision}</span>
        <div className="text-xs mt-1">{item.result ?? item.error}</div>
      </td>
      <td>
        <div className="flex flex-col gap-1">
          {item.status === 'awaiting_destination' && (
            <button
              className="btn btn-ghost btn-xs"
              disabled={busy}
              onClick={() => void onSkip(item)}
            >
              {t('actions.skip')}
            </button>
          )}
          {canEditPlan && (
            <button
              className="btn btn-ghost btn-xs"
              disabled={busy}
              onClick={() => void onRetry(item)}
            >
              {t('actions.retry')}
            </button>
          )}
          <button className="btn btn-ghost btn-xs" onClick={() => void onOpenInExplorer(item)}>
            {t('actions.open')}
          </button>
        </div>
      </td>
    </tr>
  );
}
