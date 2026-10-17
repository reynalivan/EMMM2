import { useEffect, useId, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { open } from '@tauri-apps/plugin-dialog';
import { LoaderCircle, Search } from 'lucide-react';
import { commands } from '@/shared/api/tauri/bindings';
import { formatAppError } from '@/shared/lib/appError';
import {
  useGameExecutableDiscovery,
  type ExecutableDiscoveryController,
} from './useGameExecutableDiscovery';

interface GameExecutableFieldProps {
  gameType: number;
  value: string;
  onChange: (value: string, detected: boolean) => void;
  xxmiRoots?: string[];
  disabled?: boolean;
  placeholder?: string;
  inputId?: string;
  discovery?: ExecutableDiscoveryController;
}

const CANDIDATE_PAGE_SIZE = 50;

export function GameExecutableField({
  gameType,
  value,
  onChange,
  xxmiRoots,
  disabled = false,
  placeholder,
  inputId,
  discovery,
}: GameExecutableFieldProps) {
  const { t } = useTranslation('settings');
  const generatedId = useId();
  const id = inputId ?? generatedId;
  const localDiscovery = useGameExecutableDiscovery(xxmiRoots);
  const detector = discovery ?? localDiscovery;
  const [selectionError, setSelectionError] = useState<string | null>(null);
  const [isValidating, setIsValidating] = useState(false);
  const [dismissed, setDismissed] = useState(false);
  const [candidatePage, setCandidatePage] = useState(0);
  const revision = useRef(0);
  const rootsKey = JSON.stringify(xxmiRoots ?? []);

  useEffect(() => {
    revision.current += 1;
    setSelectionError(null);
    setIsValidating(false);
    setDismissed(false);
    setCandidatePage(0);
    return () => {
      revision.current += 1;
    };
  }, [gameType, rootsKey, disabled]);

  useEffect(() => {
    revision.current += 1;
    setIsValidating(false);
  }, [value]);

  const change = (path: string, detected: boolean) => {
    revision.current += 1;
    setIsValidating(false);
    setSelectionError(null);
    setDismissed(true);
    onChange(path, detected);
  };

  const browse = async () => {
    const request = ++revision.current;
    setIsValidating(false);
    setSelectionError(null);
    try {
      const selected = await open({
        directory: false,
        multiple: false,
        title: t('games.form.validation.pick_exe_title'),
        filters: [{ name: t('games.executable_detection.file_filter'), extensions: ['exe'] }],
      });
      if (request === revision.current && typeof selected === 'string') change(selected, false);
    } catch (error) {
      if (request === revision.current) setSelectionError(formatAppError(error));
    }
  };

  const select = async (path: string) => {
    const request = ++revision.current;
    setIsValidating(true);
    setSelectionError(null);
    try {
      const candidate = await commands.validateGameExecutable({
        game_type: gameType,
        executable_path: path,
      });
      if (request === revision.current) change(candidate.executable_path, true);
    } catch (error) {
      if (request === revision.current) setSelectionError(formatAppError(error));
    } finally {
      if (request === revision.current) setIsValidating(false);
    }
  };

  const scanning = detector.pendingTypes.includes(gameType);
  useEffect(() => {
    revision.current += 1;
    setIsValidating(false);
    if (scanning) {
      setDismissed(false);
      setCandidatePage(0);
    }
  }, [scanning]);
  const completed = detector.completedTypes.includes(gameType);
  const candidates = detector.candidates.filter((candidate) => candidate.game_type === gameType);
  const pageCount = Math.ceil(candidates.length / CANDIDATE_PAGE_SIZE);
  const page = Math.min(candidatePage, Math.max(0, pageCount - 1));
  const visibleCandidates = candidates.slice(
    page * CANDIDATE_PAGE_SIZE,
    (page + 1) * CANDIDATE_PAGE_SIZE,
  );
  const showResults = !dismissed && !scanning;

  return (
    <div className="form-control min-w-0" data-testid="game-executable-field">
      <label className="label" htmlFor={id}>
        <span className="label-text font-medium">{t('games.form.exe_label')}</span>
      </label>
      <div className="flex flex-wrap items-center gap-2">
        <input
          id={id}
          name="game_exe"
          type="text"
          className="input input-bordered min-w-0 flex-1"
          value={value}
          placeholder={placeholder ?? t('games.form.exe_placeholder')}
          disabled={disabled}
          aria-describedby={`${id}-help`}
          onChange={(event) => change(event.target.value, false)}
        />
        <button
          type="button"
          className="btn btn-neutral"
          disabled={disabled}
          onClick={() => void browse()}
        >
          {t('games.form.browse')}
        </button>
        <button
          type="button"
          className="btn btn-outline"
          disabled={disabled || detector.pendingTypes.length > 0}
          onClick={() => {
            setDismissed(false);
            void detector.scan([gameType]);
          }}
        >
          {scanning ? (
            <LoaderCircle className="size-4 animate-spin" aria-hidden="true" />
          ) : (
            <Search className="size-4" aria-hidden="true" />
          )}
          {t('games.executable_detection.detect')}
        </button>
      </div>
      <p id={`${id}-help`} className="mt-1 text-xs text-base-content/70">
        {t('games.form.exe_help')}
      </p>
      {scanning && (
        <div className="mt-2 flex items-center gap-2 text-sm" role="status">
          <span>{t('games.executable_detection.scanning')}</span>
          <button
            type="button"
            className="btn btn-ghost btn-xs"
            onClick={() => {
              setDismissed(true);
              detector.cancel();
            }}
          >
            {t('games.executable_detection.cancel')}
          </button>
        </div>
      )}
      {showResults && completed && candidates.length === 0 && (
        <p className="mt-2 text-sm" role="status">
          {t('games.executable_detection.empty')}
        </p>
      )}
      {showResults && candidates.length > 0 && (
        <fieldset className="mt-3 space-y-2">
          <legend className="text-sm font-medium">{t('games.executable_detection.results')}</legend>
          {visibleCandidates.map((candidate) => (
            <div
              key={candidate.executable_path}
              className="rounded-lg border border-base-300 p-3 space-y-2"
            >
              <p className="text-sm font-medium">
                {t(`games.executable_detection.games.${candidate.game_type}`)}
              </p>
              <code className="block break-all text-xs">{candidate.executable_path}</code>
              <p className="text-xs text-base-content/70">
                {candidate.sources
                  .map((source) => t(`games.executable_detection.sources.${source}`))
                  .join(', ')}
                {candidate.region && <span> · {candidate.region}</span>}
                {candidate.channel && <span> · {candidate.channel}</span>}
              </p>
              <button
                type="button"
                className="btn btn-primary btn-sm"
                disabled={disabled || isValidating}
                onClick={() => void select(candidate.executable_path)}
              >
                {t('games.executable_detection.use')}
              </button>
            </div>
          ))}
          {pageCount > 1 && (
            <div className="flex items-center gap-2">
              <button
                type="button"
                className="btn btn-ghost btn-sm"
                disabled={page === 0}
                onClick={() => setCandidatePage(page - 1)}
              >
                {t('games.executable_detection.previous')}
              </button>
              <span className="text-xs">
                {t('games.executable_detection.page', { current: page + 1, total: pageCount })}
              </span>
              <button
                type="button"
                className="btn btn-ghost btn-sm"
                disabled={page + 1 === pageCount}
                onClick={() => setCandidatePage(page + 1)}
              >
                {t('games.executable_detection.next')}
              </button>
            </div>
          )}
          <button
            type="button"
            className="btn btn-ghost btn-sm"
            onClick={() => {
              revision.current += 1;
              setIsValidating(false);
              setDismissed(true);
            }}
          >
            {t('games.executable_detection.dismiss')}
          </button>
        </fieldset>
      )}
      {!dismissed && detector.issues.length > 0 && (
        <div className="mt-2 text-sm text-warning" role="status">
          <p>{t('games.executable_detection.partial')}</p>
          <ul className="mt-1 space-y-1">
            {detector.issues.map((issue, index) => (
              <li key={`${issue.source}:${issue.code}:${index}`}>
                {t('games.executable_detection.issue', {
                  source: t(`games.executable_detection.sources.${issue.source}`),
                  reason: t(`games.executable_detection.issues.${issue.code}`),
                })}
              </li>
            ))}
          </ul>
        </div>
      )}
      {(selectionError || (!dismissed && detector.error)) && (
        <p className="mt-2 text-sm text-error" role="alert">
          {selectionError || detector.error}
        </p>
      )}
    </div>
  );
}
