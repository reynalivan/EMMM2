import { X } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export type TrustDocument = 'privacy' | 'terms';

interface TrustInformationDialogProps {
  document: TrustDocument | null;
  onClose: () => void;
}

export function TrustInformationDialog({ document, onClose }: TrustInformationDialogProps) {
  const { t } = useTranslation('settings');

  if (!document) return null;

  const content =
    document === 'privacy'
      ? {
          title: t('general.trust.privacy.title'),
          sections: [
            {
              title: t('general.trust.privacy.section_one'),
              body: t('general.trust.privacy.body_one'),
            },
            {
              title: t('general.trust.privacy.section_two'),
              body: t('general.trust.privacy.body_two'),
            },
            {
              title: t('general.trust.privacy.section_three'),
              body: t('general.trust.privacy.body_three'),
            },
          ],
        }
      : {
          title: t('general.trust.terms.title'),
          sections: [
            {
              title: t('general.trust.terms.section_one'),
              body: t('general.trust.terms.body_one'),
            },
            {
              title: t('general.trust.terms.section_two'),
              body: t('general.trust.terms.body_two'),
            },
            {
              title: t('general.trust.terms.section_three'),
              body: t('general.trust.terms.body_three'),
            },
          ],
        };

  return (
    <dialog
      open
      className="modal modal-open bg-overlay-mask backdrop-blur-sm"
      aria-modal="true"
      aria-labelledby="trust-information-title"
      onCancel={onClose}
    >
      <section className="modal-box flex max-h-[80vh] max-w-xl flex-col overflow-hidden p-0">
        <header className="flex shrink-0 items-center justify-between border-b border-base-300 px-5 py-4">
          <h3 id="trust-information-title" className="text-base font-semibold">
            {content.title}
          </h3>
          <button
            type="button"
            className="btn btn-ghost btn-sm btn-square"
            aria-label={t('general.trust.close_dialog')}
            onClick={onClose}
          >
            <X size={18} />
          </button>
        </header>
        <div className="min-h-0 overflow-y-auto px-5 py-4 text-sm leading-6 text-base-content/75">
          <div className="space-y-4">
            {content.sections.map((section) => (
              <section key={section.title}>
                <h4 className="font-medium text-base-content">{section.title}</h4>
                <p className="mt-1">{section.body}</p>
              </section>
            ))}
          </div>
        </div>
      </section>
      <form method="dialog" className="modal-backdrop">
        <button onClick={onClose}>{t('general.trust.close')}</button>
      </form>
    </dialog>
  );
}
