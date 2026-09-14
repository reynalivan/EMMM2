import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { BrowserBookmark } from '@/shared/api/tauri/bindings';
import { useDialogSync } from '@/shared/lib/hooks/useDialogSync';
import { LiquidSurface } from '@/shared/ui/liquid';

interface BookmarkEditorDialogProps {
  bookmark: BrowserBookmark | null;
  isSaving: boolean;
  onClose: () => void;
  onSave: (input: { id: string; url: string; title: string }) => Promise<boolean>;
}

export function BookmarkEditorDialog({
  bookmark,
  isSaving,
  onClose,
  onSave,
}: BookmarkEditorDialogProps) {
  const { t } = useTranslation(['browser', 'common']);
  const dialogRef = useRef<HTMLDialogElement>(null);
  const [title, setTitle] = useState('');
  const [url, setUrl] = useState('');

  useDialogSync(dialogRef, bookmark !== null);

  useEffect(() => {
    setTitle(bookmark?.title ?? '');
    setUrl(bookmark?.url ?? '');
  }, [bookmark]);

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!bookmark) return;

    const didSave = await onSave({ id: bookmark.id, title: title.trim(), url: url.trim() });
    if (didSave) onClose();
  };

  return (
    <dialog
      ref={dialogRef}
      className="modal modal-bottom sm:modal-middle bg-overlay-mask"
      aria-labelledby="bookmark-editor-title"
      onCancel={onClose}
      onClose={onClose}
    >
      <LiquidSurface liquidRole="overlay" className="modal-box max-w-md shadow-2xl">
        <form className="space-y-4" onSubmit={handleSubmit}>
          <div>
            <h2 id="bookmark-editor-title" className="text-base font-semibold">
              {t('library.editor_title')}
            </h2>
          </div>

          <label className="form-control gap-1.5">
            <span className="label-text text-sm">{t('library.editor_name')}</span>
            <input
              className="input input-bordered w-full"
              value={title}
              onChange={(event) => setTitle(event.target.value)}
              autoComplete="off"
            />
          </label>

          <label className="form-control gap-1.5">
            <span className="label-text text-sm">{t('library.editor_url')}</span>
            <input
              className="input input-bordered w-full"
              type="url"
              value={url}
              onChange={(event) => setUrl(event.target.value)}
              autoComplete="url"
              required
            />
          </label>

          <div className="modal-action mt-5">
            <button type="button" className="btn btn-ghost" onClick={onClose} disabled={isSaving}>
              {t('common:actions.cancel')}
            </button>
            <button type="submit" className="btn btn-primary" disabled={isSaving || !url.trim()}>
              {t('library.save_bookmark')}
            </button>
          </div>
        </form>
      </LiquidSurface>
      <form method="dialog" className="modal-backdrop">
        <button type="button" onClick={onClose} aria-label={t('common:actions.close')} />
      </form>
    </dialog>
  );
}
