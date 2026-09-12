import { useEffect, useState, type RefCallback } from 'react';
import { Check, ExternalLink, Eye, FileArchive, Folder, Pencil, RefreshCw, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type {
  DestinationSuggestion,
  DestinationMatchMethod,
  ImportBatch,
  ImportDecision,
  ImportItem,
  ImportSourcePreview,
} from '../../../shared/api/tauri/bindings.gen';
import type { ObjectSummary } from '@/entities/game-object';
import { archiveErrorKindFromStoredMessage } from '../../../shared/lib/appError';
import { destinationDecision } from '../utils/importBatchDecision';
import { ImportBatchWizardDestinationPanel } from './ImportBatchWizardDestinationPanel';
import { ImportSourcePreviewCard } from './ImportSourcePreviewCard';
import { MatchScoreBadge } from './MatchScoreBadge';

type Props = {
  batch: ImportBatch;
  item: ImportItem;
  objects: ObjectSummary[];
  busy: boolean;
  selected: boolean;
  onToggleSelected: () => void;
  onChooseDestination: (
    item: ImportItem,
    suggestion: DestinationSuggestion,
    decision: ImportDecision,
  ) => Promise<void>;
  onChooseManualTarget: (item: ImportItem, objectId: string) => Promise<void>;
  onSkip: (item: ImportItem) => Promise<void>;
  onRename: (item: ImportItem, plannedName: string) => Promise<void>;
  onRetry: (item: ImportItem) => Promise<void>;
  onRevealSource: (item: ImportItem) => Promise<void>;
  onRevealDestination?: (item: ImportItem) => Promise<void>;
  onLoadSourcePreview?: (item: ImportItem) => Promise<ImportSourcePreview>;
  virtualIndex?: number;
  measureElement?: RefCallback<HTMLTableRowElement>;
};

export function ImportBatchWizardItemRow({
  batch,
  busy,
  item,
  objects,
  onChooseDestination,
  onChooseManualTarget,
  onLoadSourcePreview,
  onRename,
  onRetry,
  onRevealDestination,
  onRevealSource,
  onSkip,
  onToggleSelected,
  selected,
  virtualIndex,
  measureElement,
}: Props) {
  const { t } = useTranslation('match_wizard');
  const sourceDisplayName = withoutDisabledPrefix(item.plannedName);
  const [plannedName, setPlannedName] = useState(sourceDisplayName);
  const [editing, setEditing] = useState(false);
  useEffect(() => setPlannedName(withoutDisabledPrefix(item.plannedName)), [item.plannedName]);
  const archiveSource = ['archive_root', 'browser_download'].includes(item.sourceKind);
  const needsRecovery = [
    'committing',
    'reconciling',
    'finalizing_metadata',
    'partial',
    'metadata_pending',
  ].includes(item.status);
  const proceed = !['pending', 'skip'].includes(item.decision) || item.status === 'ready';
  const canEdit = [
    'discovered',
    'staged',
    'awaiting_category',
    'awaiting_destination',
    'ready',
    'skipped',
    'failed',
  ].includes(item.status);
  const canRetry = [
    'discovered',
    'staged',
    'awaiting_category',
    'awaiting_destination',
    'failed',
    'partial',
    'metadata_pending',
    'finalizing_metadata',
    'reconciling',
    'committing',
  ].includes(item.status);
  const topSuggestion = item.destinationSuggestions[0] ?? null;
  const targetComparison = item.targetComparison;
  const canKeepSeparate =
    targetComparison !== null &&
    targetComparison.outcome !== 'already_installed' &&
    targetComparison.suggestedSeparateName !== null &&
    topSuggestion !== null;
  const archiveError = archiveErrorKindFromStoredMessage(item.error);
  const errorText = needsRecovery
    ? t('errors.metadata_pending')
    : item.error
      ? archiveError
        ? t(`errors.archive.${archiveError}`)
        : item.error
      : null;
  const selectedSuggestion = item.destinationSuggestions.find(
    (suggestion) =>
      (item.destinationObjectId !== null && suggestion.objectId === item.destinationObjectId) ||
      (item.destinationPath !== null && suggestion.targetPath === item.destinationPath),
  );
  const selectedTopSuggestion =
    selectedSuggestion !== undefined &&
    topSuggestion !== null &&
    selectedSuggestion.kind === topSuggestion.kind &&
    selectedSuggestion.objectId === topSuggestion.objectId &&
    selectedSuggestion.targetPath === topSuggestion.targetPath;
  const hasManualDestination =
    item.decision !== 'skip' &&
    (item.destinationObjectId !== null || item.destinationPath !== null) &&
    !selectedTopSuggestion;
  const displayedConfidence = item.confidencePercentage;
  const displayedTier = item.confidenceTier;
  const displayedMethod = effectiveMatchMethod(selectedSuggestion);
  const selectedObjectName = item.destinationObjectId
    ? objects.find((object) => object.id === item.destinationObjectId)?.name
    : undefined;
  const sourceIdentification = item.canonicalSuggestions.find(
    (suggestion) => suggestion.confidenceTier === 'high' || suggestion.confidenceTier === 'medium',
  );
  const identityNeedsReview = item.identityMatchStatus === 'needs_review';
  const duplicateSourceName = item.duplicateOfItemId
    ? batch.items.find((candidate) => candidate.id === item.duplicateOfItemId)?.plannedName
    : null;
  const selectedDestinationName =
    selectedObjectName ??
    selectedSuggestion?.folderName ??
    item.destinationPath ??
    t('no_destination');

  const saveName = async () => {
    const next = plannedName.trim();
    if (!next || next === sourceDisplayName) {
      setPlannedName(sourceDisplayName);
      setEditing(false);
      return;
    }
    await onRename(item, next);
    setEditing(false);
  };

  return (
    <tr
      ref={measureElement}
      data-index={virtualIndex}
      className={selected ? 'bg-primary/5' : undefined}
    >
      <td className="w-10 text-center align-middle">
        <input
          type="checkbox"
          className="checkbox checkbox-sm checkbox-primary"
          checked={selected}
          onChange={onToggleSelected}
          aria-label={t('selection.item', { name: item.plannedName })}
        />
      </td>
      <td className="min-w-0 align-middle">
        <ImportSourcePreviewCard item={item} loadPreview={onLoadSourcePreview}>
          <div className="group/source flex min-h-14 min-w-0 flex-col justify-center">
            {editing ? (
              <div className="min-w-0">
                <div className="flex items-center gap-1">
                  <input
                    className="input input-sm input-bordered min-w-0 flex-1 font-semibold"
                    value={plannedName}
                    onChange={(event) => setPlannedName(event.target.value)}
                    onKeyDown={(event) => {
                      if (event.key === 'Enter') void saveName();
                      if (event.key === 'Escape') {
                        setPlannedName(sourceDisplayName);
                        setEditing(false);
                      }
                    }}
                    disabled={busy}
                    autoFocus
                  />
                  <button
                    type="button"
                    className="btn btn-ghost btn-square btn-sm text-success"
                    onClick={() => void saveName()}
                    aria-label={t('actions.save_name')}
                  >
                    <Check size={15} />
                  </button>
                  <button
                    type="button"
                    className="btn btn-ghost btn-square btn-sm text-error"
                    onClick={() => {
                      setPlannedName(sourceDisplayName);
                      setEditing(false);
                    }}
                    aria-label={t('common:actions.cancel')}
                  >
                    <X size={15} />
                  </button>
                </div>
                <p
                  className="mt-1 truncate text-[11px] text-base-content/55"
                  title={t('source.source_unchanged')}
                >
                  {t('source.destination_preview', {
                    destination: selectedDestinationName,
                    name: plannedName.trim() || sourceDisplayName,
                  })}
                </p>
              </div>
            ) : (
              <div className="flex min-w-0 items-center gap-1">
                <span className="truncate font-semibold">{sourceDisplayName}</span>
                <button
                  type="button"
                  className="btn btn-ghost btn-square btn-xs opacity-0 transition-opacity group-hover/source:opacity-100 focus:opacity-100"
                  disabled={!canEdit || busy}
                  onClick={() => setEditing(true)}
                  aria-label={t('source.edit_name')}
                  title={t('source.edit_name')}
                >
                  <Pencil size={13} />
                </button>
              </div>
            )}
            {!editing && (
              <div className="mt-0.5 flex min-w-0 items-center gap-1.5 text-[11px] leading-4 text-base-content/50">
                {archiveSource ? (
                  <FileArchive size={13} className="shrink-0" aria-hidden="true" />
                ) : (
                  <Folder size={13} className="shrink-0" aria-hidden="true" />
                )}
                <button
                  type="button"
                  className="truncate text-left hover:text-primary hover:underline"
                  title={item.sourcePath}
                  onClick={() => void onRevealSource(item)}
                >
                  {shortPath(item.sourcePath)}
                </button>
                <ExternalLink
                  size={11}
                  className="shrink-0 opacity-0 group-hover/source:opacity-60"
                  aria-hidden="true"
                />
              </div>
            )}
            <p className="mt-0.5 truncate text-[11px] leading-4 text-base-content/60">
              {sourceIdentification
                ? t('source.identified', {
                    name: sourceIdentification.name,
                    score: sourceIdentification.confidencePercentage,
                  })
                : t('source.matching_by_name')}
            </p>
            {identityNeedsReview && (
              <p className="mt-0.5 text-[11px] font-medium leading-4 text-warning">
                {t('source.identity_review_required')}
              </p>
            )}
            <p className="mt-0.5 truncate text-[11px] leading-4 text-base-content/55">
              {t(`content_kind.${item.contentKind}`)} · {t(`package_shape.${item.packageShape}`)}
            </p>
            {duplicateSourceName && (
              <p className="mt-0.5 truncate text-[11px] leading-4 text-warning">
                {t('source.duplicate_of', { name: duplicateSourceName })}
              </p>
            )}
          </div>
        </ImportSourcePreviewCard>
        {errorText && (
          <p className="mt-2 line-clamp-2 text-xs leading-snug text-error">{errorText}</p>
        )}
        {targetComparison && (
          <p className="mt-1 line-clamp-2 text-xs leading-snug text-warning">
            {t(`target_comparison.${targetComparison.outcome}`, {
              additional: targetComparison.additionalFiles,
              changed: targetComparison.changedFiles,
              missing: targetComparison.missingFiles,
            })}
          </p>
        )}
        {item.reviewGate.reasons.map((reason) => (
          <p key={`${reason.code}-${reason.diagnosticCode ?? ''}`} className="mt-1 text-xs leading-snug text-warning">
            {t(`review_reasons.${reason.code}`)}
          </p>
        ))}
        {item.diagnostics.map((diagnostic) => (
          <p
            key={`${diagnostic.stage}-${diagnostic.code}`}
            className="mt-1 text-xs leading-snug text-warning"
          >
            {t(`diagnostics.${diagnostic.code}`, { defaultValue: diagnostic.recovery })}
          </p>
        ))}
      </td>
      <td className="min-w-0 align-middle">
        <div className="flex min-h-14 min-w-0 items-center gap-1.5">
          <div className="min-w-0 flex-1">
            <ImportBatchWizardDestinationPanel
              batch={batch}
              busy={busy}
              item={item}
              objects={objects}
              onChooseDestination={onChooseDestination}
              onChooseManualTarget={onChooseManualTarget}
            />
          </div>
          {item.destinationPath && onRevealDestination && (
            <button
              type="button"
              className="btn btn-ghost btn-square h-11 min-h-11 w-11 shrink-0"
              onClick={() => void onRevealDestination(item)}
              aria-label={t('actions.view_destination')}
              title={t('actions.view_destination')}
            >
              <Eye size={16} />
            </button>
          )}
        </div>
      </td>
      <td className="min-w-0 align-middle">
        <MatchScoreBadge
          score={displayedConfidence}
          tier={displayedTier}
          destinationName={selectedDestinationName}
          method={displayedMethod}
          manual={hasManualDestination}
          categoryWarning={
            selectedSuggestion?.warning !== null && selectedSuggestion?.warning !== undefined
          }
        />
      </td>
      <td className="w-32 align-middle">
        <div className="join join-vertical flex min-h-14 w-full flex-col justify-center">
          <button
            type="button"
            className={`btn btn-xs join-item justify-start ${proceed ? 'btn-success' : 'btn-ghost'}`}
            disabled={busy || (!proceed && !topSuggestion)}
            onClick={() => {
              if (!proceed && topSuggestion) {
                void onChooseDestination(
                  item,
                  topSuggestion,
                  destinationDecision(batch, topSuggestion),
                );
              }
            }}
          >
            {t('actions.proceed')}
          </button>
          {canKeepSeparate && targetComparison && (
            <button
              type="button"
              className="btn btn-xs join-item justify-start"
              disabled={busy}
              onClick={() => {
                if (topSuggestion) {
                  void onChooseDestination(item, topSuggestion, 'keep_separate');
                }
              }}
              title={targetComparison.suggestedSeparateName ?? undefined}
            >
              {t('actions.keep_separate', { name: targetComparison.suggestedSeparateName })}
            </button>
          )}
          <button
            type="button"
            className={`btn btn-xs join-item justify-start ${
              item.decision === 'skip' ? 'btn-warning' : 'btn-ghost'
            }`}
            disabled={busy}
            onClick={() => void onSkip(item)}
          >
            {t('actions.skip')}
          </button>
        </div>
        {canRetry && (
          <button
            type="button"
            className="btn btn-ghost btn-xs mt-1 gap-1"
            disabled={busy}
            onClick={() => void onRetry(item)}
          >
            <RefreshCw size={11} /> {needsRecovery ? t('actions.resume') : t('actions.retry')}
          </button>
        )}
      </td>
    </tr>
  );
}

function shortPath(path: string): string {
  const parts = path.split(/[\\/]+/).filter(Boolean);
  if (parts.length <= 2) return parts.join('/');
  return `…/${parts.slice(-2).join('/')}`;
}

function withoutDisabledPrefix(name: string): string {
  return name.replace(/^disable(?:d)?[\s_-]+/i, '').trim() || name;
}

function effectiveMatchMethod(
  suggestion: DestinationSuggestion | undefined,
): DestinationMatchMethod {
  if (!suggestion) return 'no_name_match';
  if (
    suggestion.matchMethod === 'no_name_match' &&
    (suggestion.kind === 'create_canonical' ||
      (suggestion.canonicalEntryKey !== null && suggestion.confidencePercentage >= 75))
  ) {
    return 'canonical_identity';
  }
  return suggestion.matchMethod ?? 'no_name_match';
}
