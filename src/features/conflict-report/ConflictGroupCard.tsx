import { Folder, FolderOpen } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { ConflictModSet, ConflictDecisions } from './conflictResolution';
import { buildConflictKey } from './conflictResolution';

interface ConflictGroupCardProps {
  conflictSet: ConflictModSet;
  decisions: ConflictDecisions;
  pathErrors: ReadonlyMap<string, string>;
  disabled: boolean;
  ignored: boolean;
  selected: boolean;
  onKeep: (path: string) => void;
  onDisable: (path: string) => void;
  onOpenFolder: (path: string) => void;
  onIgnoreGroup: () => void;
  onRestoreGroup: () => void;
  onSelectedChange: (selected: boolean) => void;
}

function pathName(path: string): string {
  return path.split(/[\\/]/).pop() || path;
}

export default function ConflictGroupCard({
  conflictSet,
  decisions,
  pathErrors,
  disabled,
  ignored,
  selected,
  onKeep,
  onDisable,
  onOpenFolder,
  onIgnoreGroup,
  onRestoreGroup,
  onSelectedChange,
}: ConflictGroupCardProps) {
  const { t } = useTranslation(['scanner']);
  const { conflicts, modPaths } = conflictSet;
  const remainingEnabled = modPaths.filter((path) => decisions.get(path) !== 'disable');
  const resolved = remainingEnabled.length <= 1;
  const headingId = `conflict-${encodeURIComponent(conflictSet.key)}`;
  const hasPotentialConflict = conflicts.some((conflict) => conflict.certainty === 'potential');
  const runtimeKeyCount = new Set(conflicts.map((conflict) => `${conflict.kind}:${conflict.hash}`))
    .size;

  return (
    <section
      className={`rounded-xl border p-3 ${
        ignored ? 'border-base-content/10 bg-base-200/30' : 'border-base-content/10 bg-base-200/50'
      }`}
      aria-labelledby={headingId}
    >
      <div className="mb-3 flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
        <div className="flex min-w-0 items-start gap-2">
          <label
            className="flex min-h-11 shrink-0 cursor-pointer items-center gap-2"
            title={t('scanner:conflict_modal.select_group')}
          >
            <input
              type="checkbox"
              className="checkbox checkbox-warning checkbox-sm"
              checked={selected}
              onChange={(event) => onSelectedChange(event.target.checked)}
              disabled={disabled}
              aria-label={t('scanner:conflict_modal.select_group')}
            />
          </label>
          <div className="flex min-w-0 flex-wrap items-center gap-2">
            <span id={headingId} className="badge badge-sm badge-neutral font-mono opacity-70">
              {t('scanner:conflict_modal.runtime_keys', { count: runtimeKeyCount })}
            </span>
            <span
              className={`badge badge-sm ${
                ignored ? 'badge-ghost' : resolved ? 'badge-success' : 'badge-warning'
              }`}
            >
              {ignored
                ? t('scanner:conflict_modal.ignored')
                : t(`scanner:conflict_modal.${resolved ? 'resolved' : 'unresolved'}`)}
            </span>
          </div>
        </div>
        <div className="flex items-center gap-2 self-end sm:self-auto">
          <span className="text-xs text-base-content/60">
            {t('scanner:conflict_modal.mod_locations', { count: modPaths.length })}
          </span>
          <button
            type="button"
            className="btn btn-sm min-h-11 border-0 bg-base-content/10 px-3 text-base-content hover:bg-base-content/15 focus-visible:outline focus-visible:outline-2 focus-visible:outline-warning focus-visible:outline-offset-2"
            onClick={ignored ? onRestoreGroup : onIgnoreGroup}
            disabled={disabled}
          >
            {t(`scanner:conflict_modal.${ignored ? 'restore_group' : 'ignore_group'}`)}
          </button>
        </div>
      </div>

      {hasPotentialConflict && (
        <p className="text-xs text-info mb-3">{t('scanner:conflict_modal.potential_guidance')}</p>
      )}

      <div className="flex flex-col gap-2">
        {modPaths.map((path) => {
          const name = pathName(path);
          const decision = decisions.get(path);
          return (
            <div key={path} className="rounded-md border border-base-content/10 bg-base-100/60 p-2">
              <div className="flex flex-wrap items-center gap-2">
                <div className="inline-flex min-w-0 flex-1 items-center gap-1 text-sm">
                  <Folder size={14} className="shrink-0" />
                  <span className="font-medium truncate" title={path}>
                    {name}
                  </span>
                </div>
                {!ignored && (
                  <>
                    <button
                      type="button"
                      className={`btn btn-sm min-h-11 ${
                        decision === 'keep' ? 'btn-success' : 'btn-ghost'
                      }`}
                      aria-label={t('scanner:conflict_modal.keep_enabled', { name })}
                      aria-pressed={decision === 'keep'}
                      onClick={() => onKeep(path)}
                      disabled={disabled}
                    >
                      {t('scanner:conflict_modal.keep')}
                    </button>
                    <button
                      type="button"
                      className={`btn btn-sm min-h-11 ${
                        decision === 'disable' ? 'btn-error' : 'btn-ghost'
                      }`}
                      aria-label={t('scanner:conflict_modal.disable_mod', { name })}
                      aria-pressed={decision === 'disable'}
                      onClick={() => onDisable(path)}
                      disabled={disabled}
                    >
                      {t('scanner:conflict_modal.disable')}
                    </button>
                    <button
                      type="button"
                      className="btn btn-sm min-h-11 btn-ghost"
                      aria-label={t('scanner:conflict_modal.ignore_mod', { name })}
                      onClick={onIgnoreGroup}
                      disabled={disabled}
                    >
                      {t('scanner:conflict_modal.ignore')}
                    </button>
                  </>
                )}
                <button
                  type="button"
                  className="btn btn-sm min-h-11 btn-ghost"
                  aria-label={t('scanner:conflict_modal.open_folder', { name })}
                  onClick={() => onOpenFolder(path)}
                  disabled={disabled}
                >
                  <FolderOpen size={13} />
                  {t('scanner:conflict_modal.open')}
                </button>
              </div>
              <code className="mt-1 ml-5 block break-all text-xs text-base-content/60">{path}</code>

              {pathErrors.get(path) && (
                <p className="mt-2 text-xs text-error" role="alert">
                  {pathErrors.get(path)}
                </p>
              )}
            </div>
          );
        })}
      </div>

      <details className="mt-3 rounded-md border border-base-content/10 bg-base-100/40 p-2">
        <summary className="cursor-pointer text-xs font-medium text-base-content/70">
          {t('scanner:conflict_modal.show_runtime_keys', { count: runtimeKeyCount })}
        </summary>
        <div className="mt-2 flex flex-col gap-2">
          {conflicts.map((conflict) => (
            <article
              key={buildConflictKey(conflict)}
              className="border-t border-base-content/10 pt-2 first:border-0 first:pt-0"
            >
              <div className="flex flex-wrap items-center gap-2">
                <span className="badge badge-sm badge-neutral font-mono opacity-70">
                  {conflict.hash}
                </span>
                <span className="badge badge-sm badge-outline">
                  {t(`scanner:conflict_modal.kind.${conflict.kind}`)}
                </span>
                <span
                  className={`badge badge-sm ${
                    conflict.certainty === 'potential' ? 'badge-info' : 'badge-warning'
                  }`}
                >
                  {t(`scanner:conflict_modal.${conflict.certainty}`)}
                </span>
                <span className="text-xs font-mono text-base-content/50">
                  [{conflict.section_name}]
                </span>
              </div>

              {conflict.evidence.map((item, index) => (
                <div
                  key={`${item.mod_path}:${item.source_path}:${item.section_name}:${index}`}
                  className="mt-1 text-xs text-base-content/60"
                >
                  <code className="block break-all">{item.source_path}</code>
                  <div className="flex flex-wrap gap-x-2 gap-y-1">
                    <span>[{item.section_name}]</span>
                    {item.namespace && (
                      <span>
                        {t('scanner:conflict_modal.namespace')}: {item.namespace}
                      </span>
                    )}
                    {item.condition && <span>{item.condition}</span>}
                    {item.priority !== null && (
                      <span>
                        {t('scanner:conflict_modal.match_priority')}: {item.priority}
                      </span>
                    )}
                    {item.match_first_index !== null && (
                      <span>
                        {t('scanner:conflict_modal.first_index')}: {item.match_first_index}
                      </span>
                    )}
                    {item.shader_stage && (
                      <span>
                        {t('scanner:conflict_modal.shader_stage')}: {item.shader_stage}
                      </span>
                    )}
                  </div>
                </div>
              ))}
            </article>
          ))}
        </div>
      </details>
    </section>
  );
}
