import { useState } from 'react';
import { KeyRound, Copy, CheckCheck, ShieldAlert, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface RecoveryCodeModalProps {
  open: boolean;
  recoveryCode: string;
  onClose: () => void;
}

export default function RecoveryCodeModal({ open, recoveryCode, onClose }: RecoveryCodeModalProps) {
  const { t } = useTranslation(['safe_mode', 'common']);
  const [copied, setCopied] = useState(false);

  if (!open) return null;

  const handleCopy = () => {
    void navigator.clipboard.writeText(recoveryCode).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 3000);
    });
  };

  return (
    <div className="modal modal-open z-50">
      <div className="modal-box bg-base-300 border border-warning/30 shadow-2xl max-w-md relative">
        <button
          onClick={onClose}
          className="btn btn-sm btn-circle btn-ghost absolute right-2 top-2"
          aria-label={t('common:actions.close')}
        >
          <X size={18} />
        </button>

        <div className="flex flex-col items-center text-center gap-4 pt-2">
          {/* Icon */}
          <div className="w-14 h-14 rounded-2xl bg-warning/10 flex items-center justify-center">
            <KeyRound size={28} className="text-warning" />
          </div>

          <div>
            <h3 className="font-bold text-lg">{t('safe_mode:recovery_modal.title')}</h3>
            <p className="text-sm text-base-content/60 mt-1">
              {t('safe_mode:recovery_modal.desc')}
            </p>
          </div>

          {/* Code Display */}
          <div className="w-full bg-base-200 rounded-xl p-4 border border-base-content/10">
            <p className="text-2xl font-mono font-bold tracking-[0.15em] text-center text-warning select-all">
              {recoveryCode}
            </p>
          </div>

          {/* Copy Button */}
          <button
            className={`btn w-full gap-2 transition-all ${
              copied ? 'btn-success' : 'btn-warning btn-outline'
            }`}
            onClick={handleCopy}
          >
            {copied ? (
              <>
                <CheckCheck size={18} />
                {t('safe_mode:recovery_modal.copied')}
              </>
            ) : (
              <>
                <Copy size={18} />
                {t('safe_mode:recovery_modal.copy')}
              </>
            )}
          </button>

          {/* Warning */}
          <div className="alert alert-warning text-xs py-2 text-left gap-2">
            <ShieldAlert size={16} className="shrink-0" />
            <span>{t('safe_mode:recovery_modal.warning')}</span>
          </div>

          <button className="btn btn-ghost btn-sm w-full" onClick={onClose}>
            {t('safe_mode:recovery_modal.confirm_close')}
          </button>
        </div>
      </div>
    </div>
  );
}
