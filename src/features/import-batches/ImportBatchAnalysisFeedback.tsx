import { useState, type FormEvent } from 'react';
import { useTranslation } from 'react-i18next';
import type { ExtractionEvent } from '../../shared/api/tauri/bindings.gen';

export type ExtractionProgress = ExtractionEvent['data'];

type Props = {
  progress: ExtractionProgress | null;
  passwordError: string | null;
  loading: boolean;
  onPasswordRetry: (password: string) => Promise<void>;
  onCancel: () => Promise<void>;
};

function ExtractionProgressStatus({ progress }: { progress: ExtractionProgress }) {
  const { t } = useTranslation('match_wizard');

  return (
    <div
      className="fixed bottom-6 left-1/2 z-[120] w-[min(32rem,calc(100vw-2rem))] -translate-x-1/2 rounded-box border border-base-300 bg-base-100 p-4 shadow-2xl"
      role="status"
      aria-live="polite"
    >
      <div className="flex items-center justify-between gap-4 text-sm font-medium">
        <span>
          {t('extraction.progress', { current: progress.fileIndex, total: progress.totalFiles })}
        </span>
        <span className="truncate text-xs opacity-60" title={progress.fileName}>
          {progress.fileName}
        </span>
      </div>
      <progress
        className="progress progress-primary mt-2 w-full"
        value={progress.fileIndex}
        max={Math.max(progress.totalFiles, 1)}
      />
    </div>
  );
}

export function ImportBatchAnalysisFeedback({
  progress,
  passwordError,
  loading,
  onPasswordRetry,
  onCancel,
}: Props) {
  const { t } = useTranslation('match_wizard');
  const [password, setPassword] = useState('');

  const submitPassword = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!password || loading) return;
    const submittedPassword = password;
    setPassword('');
    void onPasswordRetry(submittedPassword);
  };

  return (
    <>
      {progress ? <ExtractionProgressStatus progress={progress} /> : null}
      {passwordError ? (
        <dialog open className="modal modal-open z-[130]" aria-labelledby="import-password-title">
          <div className="modal-box max-w-md">
            <h2 id="import-password-title" className="text-lg font-bold">
              {t('archive_password.title')}
            </h2>
            <p className="mt-2 text-sm opacity-70">{t('archive_password.description')}</p>
            <p className="alert alert-warning mt-4 text-sm" role="alert">
              {passwordError}
            </p>
            <form className="mt-4 space-y-4" onSubmit={submitPassword}>
              <label className="form-control w-full">
                <span className="label-text mb-1">{t('archive_password.label')}</span>
                <input
                  type="password"
                  className="input input-bordered w-full"
                  value={password}
                  onChange={(event) => setPassword(event.target.value)}
                  autoComplete="new-password"
                  spellCheck={false}
                  required
                  disabled={loading}
                  autoFocus
                />
              </label>
              <div className="modal-action">
                <button
                  type="button"
                  className="btn btn-ghost"
                  onClick={() => void onCancel()}
                  disabled={loading}
                >
                  {t('archive_password.cancel')}
                </button>
                <button type="submit" className="btn btn-primary" disabled={loading || !password}>
                  {loading ? <span className="loading loading-spinner loading-sm" /> : null}
                  {t('archive_password.retry')}
                </button>
              </div>
            </form>
          </div>
        </dialog>
      ) : null}
    </>
  );
}
