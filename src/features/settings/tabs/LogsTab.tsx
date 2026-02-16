import { useCallback, useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
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
  const { addToast } = useToastStore();
  const [isLoading, setIsLoading] = useState(false);
  const [level, setLevel] = useState<LogLevel>('ALL');
  const [lines, setLines] = useState<string[]>([]);

  const loadLogs = useCallback(async () => {
    setIsLoading(true);
    try {
      const next = await invoke<string[]>('get_log_lines', { lines: 300 });
      setLines(next);
    } catch (error) {
      console.error(error);
      addToast('error', `Failed to load logs: ${String(error)}`);
    } finally {
      setIsLoading(false);
    }
  }, [addToast]);

  const openLogFolder = async () => {
    try {
      await invoke('open_log_folder');
    } catch (error) {
      console.error(error);
      addToast('error', `Failed to open log folder: ${String(error)}`);
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
              <h3 className="card-title text-lg">Logs</h3>
              <p className="text-sm text-base-content/70">Recent app logs from tauri-plugin-log.</p>
            </div>
            <div className="flex items-center gap-2">
              <select
                className="select select-bordered select-sm"
                value={level}
                onChange={(event) => setLevel(event.target.value as LogLevel)}
                aria-label="Filter logs by level"
              >
                <option value="ALL">All</option>
                <option value="INFO">Info</option>
                <option value="WARN">Warn</option>
                <option value="ERROR">Error</option>
              </select>

              <button
                type="button"
                className="btn btn-sm btn-neutral gap-2"
                onClick={() => void loadLogs()}
                disabled={isLoading}
              >
                <RefreshCcw size={14} className={isLoading ? 'animate-spin' : ''} />
                Refresh
              </button>

              <button
                type="button"
                className="btn btn-sm btn-outline gap-2"
                onClick={() => void openLogFolder()}
              >
                <ExternalLink size={14} />
                Open Folder
              </button>
            </div>
          </div>

          <div className="mt-4 rounded-lg border border-base-300 bg-base-100 p-3">
            <div className="max-h-112 overflow-auto font-mono text-xs leading-5">
              {visibleLines.length === 0 ? (
                <p className="text-base-content/60">No log entries match the selected level.</p>
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
