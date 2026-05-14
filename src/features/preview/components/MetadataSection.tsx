import { useState, useEffect, useRef } from 'react';
import { Pencil } from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface MetadataSectionProps {
  activePath: string | null;
  authorDraft: string;
  versionDraft: string;
  descriptionDraft: string;
  metadataDirty: boolean;
  canEdit?: boolean;
  onAuthorChange: (value: string) => void;
  onVersionChange: (value: string) => void;
  onDescriptionChange: (value: string) => void;
  onDiscard: () => void;
}

export default function MetadataSection({
  activePath,
  authorDraft,
  versionDraft,
  descriptionDraft,
  metadataDirty,
  canEdit = true,
  onAuthorChange,
  onVersionChange,
  onDescriptionChange,
  onDiscard,
}: MetadataSectionProps) {
  const { t } = useTranslation(['preview']);
  const [showSavedStatus, setShowSavedStatus] = useState(false);
  const [isEditing, setIsEditing] = useState(false);
  const prevDirty = useRef(metadataDirty);

  useEffect(() => {
    let timer: number;
    if (metadataDirty) {
      setShowSavedStatus(false);
    } else if (!metadataDirty && prevDirty.current) {
      setShowSavedStatus(true);
      timer = window.setTimeout(() => setShowSavedStatus(false), 2000);
    }
    prevDirty.current = metadataDirty;
    return () => clearTimeout(timer);
  }, [metadataDirty]);

  useEffect(() => {
    if (!canEdit && isEditing) {
      setIsEditing(false);
    }
  }, [canEdit, isEditing]);

  if (!isEditing) {
    return (
      <div className="mb-6 flex flex-col">
        <div className="mb-2 flex items-center justify-between">
          <h3 className="text-xs font-bold uppercase tracking-widest text-base-content/40">
            {t('preview:metadata.title')}
          </h3>
          <div className="flex items-center gap-2">
            {showSavedStatus && (
              <span className="text-[10px] font-normal text-success/70 bg-success/5 px-1.5 py-0.5 rounded border border-success/10 flex items-center gap-1">
                <span className="w-1 h-1 rounded-full bg-success/50" />
                {t('preview:metadata.done_label')}
              </span>
            )}
            <button
              className="btn btn-ghost btn-xs text-base-content/50 hover:text-base-content"
              onClick={() => setIsEditing(true)}
              title={t('preview:metadata.edit_title')}
              disabled={!activePath || !canEdit}
            >
              <Pencil size={14} /> {t('preview:actions.edit')}
            </button>
          </div>
        </div>

        <div
          className="flex-1 cursor-pointer rounded-lg hover:bg-base-content/5 p-2 -mx-2 transition-colors group"
          onDoubleClick={() => {
            if (activePath && canEdit) setIsEditing(true);
          }}
          title={activePath ? t('preview:metadata.double_click_edit') : undefined}
        >
          <div className="flex items-center gap-2 text-xs text-base-content/60 mb-3">
            <span>{authorDraft || t('preview:metadata.unknown_author')}</span>
            <span className="w-1 h-1 rounded-full bg-base-content/30" />
            <span>v{versionDraft || t('preview:metadata.version_default')}</span>
          </div>

          {descriptionDraft ? (
            <p className="text-sm text-base-content/80 whitespace-pre-wrap leading-relaxed">
              {descriptionDraft}
            </p>
          ) : (
            <p className="text-sm text-base-content/40 italic">
              {t('preview:metadata.no_description')}
            </p>
          )}
        </div>
      </div>
    );
  }

  return (
    <div className="mb-6 flex flex-col">
      <div className="mb-2 flex items-center justify-between">
        <h3 className="font-bold text-sm tracking-tight flex items-center gap-2">
          {t('preview:metadata.title')}
          {metadataDirty && ( // Use metadataDirty for auto-saving status
            <span className="text-[10px] font-normal text-primary/70 animate-pulse bg-primary/5 px-1.5 py-0.5 rounded border border-primary/10">
              {t('preview:metadata.auto_saving_label')}
            </span>
          )}
          {showSavedStatus && (
            <span className="text-[10px] font-normal text-success/70 bg-success/5 px-1.5 py-0.5 rounded border border-success/10 flex items-center gap-1">
              <span className="w-1 h-1 rounded-full bg-success/50" />
              {t('preview:metadata.done_label')}
            </span>
          )}
        </h3>
        <div className="flex items-center gap-2">
          {metadataDirty ? (
            <button
              className="btn btn-ghost btn-xs text-warning"
              onClick={onDiscard}
              title={t('preview:metadata.revert_title')}
            >
              {t('preview:actions.revert')}
            </button>
          ) : (
            <button
              className="btn btn-ghost btn-xs text-primary"
              onClick={() => setIsEditing(false)}
            >
              {t('preview:actions.done')}
            </button>
          )}
        </div>
      </div>

      <div className="flex-1 overflow-y-auto">
        <div className="mb-2 grid grid-cols-2 gap-2">
          <div>
            <label className="label py-1" htmlFor="metadata-author-input">
              <span className="label-text text-xs">{t('preview:metadata.author')}</span>
            </label>
            <input
              id="metadata-author-input"
              aria-label={t('preview:metadata.author')}
              type="text"
              className="input input-bordered w-full bg-transparent text-sm"
              placeholder={t('preview:metadata.author_placeholder')}
              value={authorDraft}
              disabled={!activePath || !canEdit}
              onChange={(event) => onAuthorChange(event.target.value)}
            />
          </div>
          <div>
            <label className="label py-1" htmlFor="metadata-version-input">
              <span className="label-text text-xs">{t('preview:metadata.version')}</span>
            </label>
            <input
              id="metadata-version-input"
              aria-label={t('preview:metadata.version_aria')}
              type="text"
              className="input input-bordered w-full bg-transparent text-sm"
              placeholder={t('preview:metadata.version_placeholder')}
              value={versionDraft}
              disabled={!activePath || !canEdit}
              onChange={(event) => onVersionChange(event.target.value)}
            />
          </div>
        </div>

        <label className="label py-1" htmlFor="metadata-description-input">
          <span className="label-text text-xs">{t('preview:metadata.description')}</span>
        </label>
        <textarea
          id="metadata-description-input"
          aria-label={t('preview:metadata.description')}
          className="textarea textarea-bordered h-24 w-full resize-none bg-transparent text-sm"
          placeholder={t('preview:metadata.description_placeholder')}
          value={descriptionDraft}
          disabled={!activePath || !canEdit}
          onChange={(event) => onDescriptionChange(event.target.value)}
        />
      </div>
    </div>
  );
}
