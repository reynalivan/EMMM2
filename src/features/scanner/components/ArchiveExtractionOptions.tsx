import { useTranslation } from 'react-i18next';

interface ArchiveExtractionOptionsProps {
  autoRename: boolean;
  disableByDefault: boolean;
  unpackNested: boolean;
  hasNestedArchives: boolean;
  onAutoRenameChange: (value: boolean) => void;
  onDisableByDefaultChange: (value: boolean) => void;
  onUnpackNestedChange: (value: boolean) => void;
}

export default function ArchiveExtractionOptions({
  autoRename,
  disableByDefault,
  unpackNested,
  hasNestedArchives,
  onAutoRenameChange,
  onDisableByDefaultChange,
  onUnpackNestedChange,
}: ArchiveExtractionOptionsProps) {
  const { t } = useTranslation(['scanner']);

  return (
    <div className="space-y-2">
      <div
        className={`form-control rounded-lg p-3 border ${autoRename ? 'bg-base-200/50 border-base-300' : 'bg-error/5 border-error/30'}`}
      >
        <label className="label cursor-pointer justify-start gap-3 py-0">
          <input
            type="checkbox"
            className="toggle toggle-sm toggle-primary"
            checked={autoRename}
            onChange={(event) => onAutoRenameChange(event.target.checked)}
          />
          <div className="flex flex-col">
            {autoRename ? (
              <span className="label-text text-sm">{t('extract.option_auto_rename')}</span>
            ) : (
              <>
                <span className="label-text text-sm text-error">
                  {t('extract.option_overwrite')}
                </span>
                <span className="text-[10px] text-error/70">
                  {t('extract.option_overwrite_desc')}
                  <span className="badge badge-error badge-xs ml-2 align-middle">
                    {t('extract.not_recommended')}
                  </span>
                </span>
              </>
            )}
          </div>
        </label>
      </div>

      <div className="form-control bg-base-200/50 rounded-lg p-3 border border-base-300">
        <label className="label cursor-pointer justify-start gap-3 py-0">
          <input
            type="checkbox"
            className="checkbox checkbox-sm checkbox-primary"
            checked={disableByDefault}
            onChange={(event) => onDisableByDefaultChange(event.target.checked)}
          />
          <span className="label-text text-sm">{t('extract.option_disabled')}</span>
        </label>
      </div>

      {hasNestedArchives && (
        <div className="form-control bg-primary/5 rounded-lg p-3 border border-primary/20">
          <label
            className="label cursor-pointer justify-start gap-3 py-0 tooltip tooltip-right"
            data-tip={t('extract.option_unpack_nested_tooltip')}
          >
            <input
              type="checkbox"
              className="checkbox checkbox-sm checkbox-primary"
              checked={unpackNested}
              onChange={(event) => onUnpackNestedChange(event.target.checked)}
            />
            <div className="flex flex-col text-left">
              <span className="label-text text-sm font-medium">
                {t('extract.option_unpack_nested')}
              </span>
            </div>
          </label>
        </div>
      )}
    </div>
  );
}
