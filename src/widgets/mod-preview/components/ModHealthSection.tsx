import { useMemo, useState } from 'react';
import { AlertTriangle, RefreshCw, ShieldAlert, ShieldCheck } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { ModViewerExternalReview } from '@/entities/mod';

export type ModHealthSupportLevel = 'full' | 'experimental' | 'baseline';
export type ModHealthSeverity = 'error' | 'warning' | 'info';

export interface ModHealthIssue {
  severity: ModHealthSeverity;
  code: string;
  message: string;
  filePath: string | null;
  line: number | null;
}

export interface ModHealthControl {
  kind: 'key_toggle' | 'menu_toggle' | 'present' | 'shape';
  name: string;
  key: string | null;
  back: string | null;
  values: string[];
  defaultValue: string | null;
}

export type ModHealthAssetCategory =
  'referenced' | 'inactive_only' | 'orphan' | 'external_reference';

export interface ModHealthAssetEntry {
  category: ModHealthAssetCategory;
  relativePath: string;
  sizeBytes: number;
  sourceFiles: string[];
}

export interface ModHealthPanelData {
  supportLevel: ModHealthSupportLevel;
  issues: ModHealthIssue[];
  assets: {
    referenced: number;
    inactiveOnly: number;
    orphan: number;
    externalReference: number;
  };
  assetEntries: ModHealthAssetEntry[];
  controls: ModHealthControl[];
}

export type { ModViewerExternalReview } from '@/entities/mod';

interface ModHealthSectionProps {
  report: ModHealthPanelData | null | undefined;
  isLoading: boolean;
  errorMessage?: string | null;
  onRecheck: () => void;
  externalReview?: ModViewerExternalReview | null;
  onDismissReview?: () => void;
  onOpenIssueFile?: (filePath: string) => void;
}

type DetailTab = 'issues' | 'assets' | 'controls';

const EXTERNAL_CHANGE_CATEGORIES: ModViewerExternalReview['changes'][number]['category'][] = [
  'ini',
  'dds',
  'backup',
  'metadata',
  'other',
];

function issueClass(severity: ModHealthSeverity): string {
  if (severity === 'error') return 'text-error';
  if (severity === 'warning') return 'text-warning';
  return 'text-info';
}

export default function ModHealthSection({
  report,
  isLoading,
  errorMessage,
  onRecheck,
  externalReview,
  onDismissReview,
  onOpenIssueFile,
}: ModHealthSectionProps) {
  const { t } = useTranslation('preview');
  const [detailsOpen, setDetailsOpen] = useState(false);
  const [activeTab, setActiveTab] = useState<DetailTab>('issues');
  const errorCount = useMemo(
    () => report?.issues.filter((issue) => issue.severity === 'error').length ?? 0,
    [report],
  );
  const warningCount = useMemo(
    () => report?.issues.filter((issue) => issue.severity === 'warning').length ?? 0,
    [report],
  );
  const externalChangesByCategory = useMemo(
    () =>
      EXTERNAL_CHANGE_CATEGORIES.map((category) => ({
        category,
        changes: externalReview?.changes.filter((change) => change.category === category) ?? [],
      })).filter((group) => group.changes.length > 0),
    [externalReview],
  );
  const externalChangeCount = externalReview?.changes.length ?? 0;
  const hasIssues = errorCount > 0 || warningCount > 0;
  const HealthIcon = hasIssues ? ShieldAlert : ShieldCheck;

  if (!report && !isLoading && !errorMessage) return null;

  return (
    <section className="mt-5 border-t border-base-content/8 pt-3">
      <div className="flex items-center gap-2">
        <HealthIcon
          size={15}
          className={
            errorCount > 0 ? 'text-error' : warningCount > 0 ? 'text-warning' : 'text-success'
          }
          aria-hidden="true"
        />
        <h3 className="text-xs font-semibold text-base-content/80">{t('mod_health.title')}</h3>
        {report && (
          <span className="text-[11px] text-base-content/45">
            {t(`mod_health.support.${report.supportLevel}`)}
          </span>
        )}
        <div className="ml-auto flex shrink-0 items-center gap-1">
          <button
            className="btn btn-ghost btn-xs btn-square"
            type="button"
            onClick={onRecheck}
            disabled={isLoading}
            aria-label={t('mod_health.recheck')}
            title={t('mod_health.recheck')}
          >
            <RefreshCw size={14} />
          </button>
          <button
            className="btn btn-ghost btn-xs"
            type="button"
            onClick={() => setDetailsOpen(true)}
            disabled={!report}
          >
            {t('mod_health.view_details')}
          </button>
        </div>
      </div>

      {isLoading ? (
        <p className="mt-1.5 text-xs text-base-content/55">{t('mod_health.loading')}</p>
      ) : errorMessage ? (
        <p className="mt-1.5 text-xs text-error">
          {t('mod_health.error', { error: errorMessage })}
        </p>
      ) : (
        <p
          className={
            hasIssues ? 'mt-1.5 text-xs text-base-content/65' : 'mt-1.5 text-xs text-success'
          }
        >
          {errorCount > 0 && (
            <span className="text-error">{t('mod_health.error_count', { count: errorCount })}</span>
          )}
          {errorCount > 0 && warningCount > 0 && <span className="text-base-content/35"> · </span>}
          {warningCount > 0 && (
            <span className="text-warning">
              {t('mod_health.warning_count', { count: warningCount })}
            </span>
          )}
          {!hasIssues && t('mod_health.no_issues')}
        </p>
      )}

      {externalReview && (
        <div className="mt-3 border-l-2 border-warning/60 pl-2.5">
          <div className="flex items-center justify-between gap-3">
            <div className="min-w-0">
              <h4 className="flex items-center gap-1.5 text-xs font-medium text-warning">
                <AlertTriangle size={13} />
                <span className="truncate">{t('mod_health.external_review.title')}</span>
                <span className="text-base-content/50">{externalChangeCount}</span>
              </h4>
            </div>
            <button
              className="btn btn-ghost btn-xs"
              type="button"
              onClick={onDismissReview}
              aria-label={t('mod_health.external_review.dismiss')}
            >
              {t('mod_health.external_review.dismiss')}
            </button>
          </div>
          <div className="mt-1.5 max-h-32 space-y-1.5 overflow-y-auto text-xs text-base-content/75">
            {externalChangesByCategory.map((group) => (
              <div key={group.category}>
                <p className="font-medium text-base-content/60">
                  {t(`mod_health.external_review.category.${group.category}`)}
                </p>
                <ul className="mt-1 space-y-1">
                  {group.changes.map((change) => (
                    <li key={`${change.kind}:${change.relativePath}`}>
                      {t(`mod_health.external_review.change.${change.kind}`)} ·{' '}
                      {change.relativePath}
                    </li>
                  ))}
                </ul>
              </div>
            ))}
          </div>
          {externalReview.collectionImpact && (
            <p className="mt-1.5 text-xs text-warning">{externalReview.collectionImpact}</p>
          )}
        </div>
      )}

      {detailsOpen && report && (
        <dialog
          open
          className="modal modal-bottom sm:modal-middle"
          onClose={() => setDetailsOpen(false)}
        >
          <div className="modal-box max-w-2xl">
            <div className="mb-4 flex items-center justify-between gap-3">
              <h4 className="text-lg font-semibold">{t('mod_health.details_title')}</h4>
              <button
                className="btn btn-ghost btn-sm"
                type="button"
                onClick={() => setDetailsOpen(false)}
              >
                {t('actions.close')}
              </button>
            </div>
            <div role="tablist" className="tabs tabs-bordered">
              {(['issues', 'assets', 'controls'] as const).map((tab) => (
                <button
                  key={tab}
                  role="tab"
                  type="button"
                  className={`tab ${activeTab === tab ? 'tab-active' : ''}`}
                  aria-selected={activeTab === tab}
                  onClick={() => setActiveTab(tab)}
                >
                  {t(`mod_health.tabs.${tab}`)}
                </button>
              ))}
            </div>

            {activeTab === 'issues' && (
              <div className="mt-4 space-y-3">
                {report.issues.length === 0 ? (
                  <p className="text-sm text-base-content/60">{t('mod_health.no_issues')}</p>
                ) : (
                  report.issues.map((issue) => (
                    <article
                      key={`${issue.code}:${issue.filePath}:${issue.line}`}
                      className="rounded-lg bg-base-200/60 p-3"
                    >
                      <p className={`text-sm font-medium ${issueClass(issue.severity)}`}>
                        {issue.message}
                      </p>
                      {issue.filePath && (
                        <div className="mt-1 flex items-center justify-between gap-2 text-xs text-base-content/60">
                          <span>
                            {issue.filePath}
                            {issue.line === null ? '' : `:${issue.line}`}
                          </span>
                          {issue.filePath.toLowerCase().endsWith('.ini') && onOpenIssueFile && (
                            <button
                              type="button"
                              className="link link-primary shrink-0"
                              onClick={() => onOpenIssueFile(issue.filePath!)}
                            >
                              {t('mod_health.open_ini')}
                            </button>
                          )}
                        </div>
                      )}
                    </article>
                  ))
                )}
              </div>
            )}

            {activeTab === 'assets' && (
              <div className="mt-4 space-y-4">
                <dl className="grid grid-cols-2 gap-3 text-sm">
                  <div>
                    <dt className="text-base-content/60">{t('mod_health.assets.referenced')}</dt>
                    <dd>{report.assets.referenced}</dd>
                  </div>
                  <div>
                    <dt className="text-base-content/60">{t('mod_health.assets.inactive_only')}</dt>
                    <dd>{report.assets.inactiveOnly}</dd>
                  </div>
                  <div>
                    <dt className="text-base-content/60">{t('mod_health.assets.orphan')}</dt>
                    <dd>{report.assets.orphan}</dd>
                  </div>
                  <div>
                    <dt className="text-base-content/60">
                      {t('mod_health.assets.external_reference')}
                    </dt>
                    <dd>{report.assets.externalReference}</dd>
                  </div>
                </dl>
                {report.assetEntries.length > 0 && (
                  <ul className="max-h-64 space-y-2 overflow-y-auto text-sm">
                    {report.assetEntries.map((asset) => (
                      <li
                        key={`${asset.category}:${asset.relativePath}`}
                        className="rounded-lg bg-base-200/60 p-3"
                      >
                        <div className="flex items-center justify-between gap-3">
                          <span className="truncate">{asset.relativePath}</span>
                          <span className="badge badge-ghost badge-sm">
                            {t(`mod_health.assets.${asset.category}`)}
                          </span>
                        </div>
                        {asset.sourceFiles.length > 0 && (
                          <p className="mt-1 truncate text-xs text-base-content/60">
                            {asset.sourceFiles.join(', ')}
                          </p>
                        )}
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            )}

            {activeTab === 'controls' && (
              <div className="mt-4 space-y-3">
                {report.controls.length === 0 ? (
                  <p className="text-sm text-base-content/60">{t('mod_health.no_controls')}</p>
                ) : (
                  report.controls.map((control) => (
                    <article
                      key={`${control.kind}:${control.name}`}
                      className="rounded-lg bg-base-200/60 p-3"
                    >
                      <div className="flex items-center justify-between gap-3">
                        <p className="font-medium">{control.name}</p>
                        <span className="badge badge-ghost badge-sm">
                          {t(`mod_health.control_kind.${control.kind}`)}
                        </span>
                      </div>
                      {(control.key || control.back) && (
                        <p className="mt-1 text-xs text-base-content/65">
                          {[control.key, control.back].filter(Boolean).join(' / ')}
                        </p>
                      )}
                      {control.values.length > 0 && (
                        <p className="mt-1 text-xs text-base-content/65">
                          {control.values.join(', ')}
                        </p>
                      )}
                      {control.defaultValue && (
                        <p className="mt-1 text-xs text-base-content/65">
                          {t('mod_health.default_value', { value: control.defaultValue })}
                        </p>
                      )}
                    </article>
                  ))
                )}
              </div>
            )}
          </div>
          <form method="dialog" className="modal-backdrop">
            <button type="submit" onClick={() => setDetailsOpen(false)}>
              {t('actions.close')}
            </button>
          </form>
        </dialog>
      )}
    </section>
  );
}
