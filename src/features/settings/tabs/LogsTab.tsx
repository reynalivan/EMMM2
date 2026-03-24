import { useCallback, useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { commands } from '../../../lib/bindings';
import { ExternalLink, RefreshCcw } from 'lucide-react';
import { useToastStore } from '../../../stores/useToastStore';

type LogLevel = 'ALL' | 'INFO' | 'WARN' | 'ERROR';

function detectLevel(line: string): Exclude<LogLevel, 'ALL'> | null {
  const upper = line.toUpperCase();
  if (upper.includes('[ERROR]') || upper.includes(' ERROR ')) {
    return 'ERROR';
  }
  if (upper.includes('[WARN]') || upper.includes(' WARN ')) {
    return 'WARN';
  }
  if (upper.includes('[INFO]') || upper.includes(' INFO ')) {
    return 'INFO';
  }
  return null;
}

export default function LogsTab() {
  const { t } = useTranslation(['settings', 'common']);
  const { addToast } = useToastStore();
  const [isLoading, setIsLoading] = useState(false);
  const [level, setLevel] = useState<LogLevel>('ALL');
  const [lines, setLines] = useState<string[]>([]);

  const loadLogs = useCallback(async () => {
    setIsLoading(true);
    try {
      const next = await commands.getLogLines({ limit: 300 });
      setLines(next);
    } catch (error) {
      console.error(error);
      addToast('error', t('settings:logs.load_failed', { error: String(error) }));
    } finally {
      setIsLoading(false);
    }
  }, [addToast, t]);

  const openLogFolder = async () => {
    try {
      await commands.openLogFolder();
    } catch (error) {
      console.error(error);
      addToast('error', t('settings:logs.folder_failed', { error: String(error) }));
    }
  };

  useEffect(() => {
    void loadLogs();
  }, [loadLogs]);

  const visibleLines = useMemo(() => {
    if (level === 'ALL') {
      return lines;
    }
    return lines.filter((line) => detectLevel(line) === level);
  }, [level, lines]);

  return (
    <div className="space-y-4">
      <div className="card border border-base-300 bg-base-200 shadow-sm">
        <div className="card-body">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div>
              <h3 className="card-title text-lg">{t('settings:logs.title')}</h3>
              <p className="text-sm text-base-content/70">{t('settings:logs.desc')}</p>
            </div>
            <div className="flex items-center gap-2">
              <select
                className="select select-bordered select-sm"
                value={level}
                onChange={(event) => setLevel(event.target.value as LogLevel)}
                aria-label={t('settings:logs.filter_label')}
              >
                <option value="ALL">{t('settings:logs.levels.all')}</option>
                <option value="INFO">{t('settings:logs.levels.info')}</option>
                <option value="WARN">{t('settings:logs.levels.warn')}</option>
                <option value="ERROR">{t('settings:logs.levels.error')}</option>
              </select>

              <button
                type="button"
                className="btn btn-sm btn-neutral gap-2"
                onClick={() => void loadLogs()}
                disabled={isLoading}
              >
                <RefreshCcw size={14} className={isLoading ? 'animate-spin' : ''} />
                {t('settings:logs.refresh')}
              </button>

              <button
                type="button"
                className="btn btn-sm btn-outline gap-2"
                onClick={() => void openLogFolder()}
              >
                <ExternalLink size={14} />
                {t('settings:logs.open_folder')}
              </button>
            </div>
          </div>

          <div className="mt-4 rounded-lg border border-base-300 bg-base-100 p-3">
            <div className="max-h-112 overflow-auto font-mono text-xs leading-5">
              {visibleLines.length === 0 ? (
                <p className="text-base-content/60">{t('settings:logs.empty')}</p>
              ) : (
                visibleLines.map((line, index) => (
                  <div
                    key={`${index}-${line.slice(0, 16)}`}
                    className="whitespace-pre-wrap break-all"
                  >
                    {line}
                  </div>
                ))
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
