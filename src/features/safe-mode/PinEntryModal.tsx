import { useState, useEffect } from 'react';
import { createPortal } from 'react-dom';
import { useSettings } from '../../hooks/useSettings';
import { commands } from '../../lib/bindings';
import { ShieldCheck, ShieldAlert, X, KeyRound, RotateCcw } from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface PinEntryModalProps {
  open: boolean;
  onClose: () => void;
  onSuccess: (pin: string) => void;
  title?: string;
  description?: string;
  cancellable?: boolean;
}

type View = 'pin' | 'recovery';

export default function PinEntryModal({
  open,
  onClose,
  onSuccess,
  title,
  description,
  cancellable = true,
}: PinEntryModalProps) {
  const { t } = useTranslation(['safe_mode', 'common']);
  const [view, setView] = useState<View>('pin');
  const [pin, setPin] = useState('');
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [lockoutTimer, setLockoutTimer] = useState<number>(0);
  const [recoveryInput, setRecoveryInput] = useState('');
  const [recoveryError, setRecoveryError] = useState<string | null>(null);
  const [recoverySuccess, setRecoverySuccess] = useState(false);

  const { verifyPin, resetPinWithRecoveryCodeAsync, settings } = useSettings();
  const hasRecoveryCode = !!settings?.safe_mode?.recovery_code_hash;

  const [prevOpen, setPrevOpen] = useState(open);
  if (open !== prevOpen) {
    setPrevOpen(open);
    if (open) {
      setPin('');
      setErrorMsg(null);
      setView('pin');
      setRecoveryInput('');
      setRecoveryError(null);
      setRecoverySuccess(false);
    }
  }

  useEffect(() => {
    let interval: number;
    if (lockoutTimer > 0) {
      interval = window.setInterval(() => {
        setLockoutTimer((prev) => prev - 1);
      }, 1000);
    }
    return () => clearInterval(interval);
  }, [lockoutTimer]);

  const handlePinSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (pin.length !== 6) {
      setErrorMsg(t('safe_mode:pin_entry.error.length'));
      return;
    }

    try {
      const isValid = await verifyPin(pin);
      if (isValid) {
        onSuccess(pin);
        onClose();
      } else {
        const status = await commands.getPinStatus();
        if (status.is_locked) {
          setLockoutTimer(status.lockout_seconds_remaining);
          setErrorMsg(
            t('safe_mode:pin_entry.error.locked', { seconds: status.lockout_seconds_remaining }),
          );
        } else {
          setErrorMsg(t('safe_mode:pin_entry.error.invalid', { count: status.attempts_remaining }));
        }
        setPin('');
      }
    } catch (err) {
      setErrorMsg(String(err));
    }
  };

  const handleRecoverySubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setRecoveryError(null);

    if (!recoveryInput.trim()) {
      setRecoveryError(t('safe_mode:recovery.error.empty'));
      return;
    }

    try {
      const valid = await resetPinWithRecoveryCodeAsync(recoveryInput.trim());
      if (valid) {
        setRecoverySuccess(true);
      } else {
        setRecoveryError(t('safe_mode:recovery.error.invalid'));
      }
    } catch (err) {
      setRecoveryError(String(err));
    }
  };

  const isLocked = lockoutTimer > 0;

  if (!open) return null;

  return createPortal(
    <div className="modal modal-open z-1000">
      <div className="modal-box bg-base-300 border border-base-content/10 shadow-2xl safe-mode-modal relative">
        {cancellable && (
          <button
            onClick={onClose}
            className="btn btn-sm btn-circle btn-ghost absolute right-2 top-2"
          >
            <X size={18} />
          </button>
        )}

        {/* ── PIN View ── */}
        {view === 'pin' && (
          <>
            <h3 className="font-bold text-lg flex items-center gap-2 text-warning mb-2">
              {isLocked ? <ShieldAlert size={20} /> : <ShieldCheck size={20} />}
              {title || t('safe_mode:pin_entry.title')}
            </h3>

            <p className="py-2 text-sm text-base-content/70">
              {description || t('safe_mode:pin_entry.desc')}
            </p>

            <form onSubmit={handlePinSubmit} className="mt-4 form-control w-full">
              <input
                type="password"
                className={`input input-bordered w-full text-center text-2xl tracking-[0.5em] transition-all bg-base-200 outline-none ${
                  errorMsg
                    ? 'border-error/50 focus:border-error'
                    : 'border-base-content/10 focus:border-primary'
                }`}
                placeholder={t('safe_mode:pin_entry.placeholder')}
                maxLength={6}
                value={pin}
                onChange={(e) => {
                  setErrorMsg(null);
                  setPin(e.target.value.replace(/[^0-9]/g, ''));
                }}
                autoFocus
                disabled={isLocked}
              />

              <div className="h-6 mt-2 flex justify-center items-center">
                {errorMsg && <span className="text-xs text-error font-medium">{errorMsg}</span>}
                {isLocked && !errorMsg && (
                  <span className="text-xs text-warning font-medium">
                    {t('safe_mode:pin_entry.lockout', { seconds: lockoutTimer })}
                  </span>
                )}
              </div>

              <div className="modal-action mt-6 gap-2">
                {cancellable && (
                  <button
                    type="button"
                    className="btn btn-ghost hover:bg-base-content/5 flex-1"
                    onClick={onClose}
                  >
                    {t('common:actions.cancel')}
                  </button>
                )}
                <button
                  type="submit"
                  className="btn btn-primary flex-1 shadow-lg shadow-primary/20"
                  disabled={pin.length !== 6 || isLocked}
                >
                  {t('safe_mode:pin_entry.verify')}
                </button>
              </div>
            </form>

            {/* Forgot PIN link */}
            {hasRecoveryCode && (
              <div className="text-center mt-3">
                <button
                  type="button"
                  className="btn btn-link btn-xs text-base-content/50 hover:text-base-content gap-1"
                  onClick={() => setView('recovery')}
                >
                  <RotateCcw size={12} />
                  {t('safe_mode:pin_entry.forgot')}
                </button>
              </div>
            )}
          </>
        )}

        {/* ── Recovery View ── */}
        {view === 'recovery' && (
          <>
            <h3 className="font-bold text-lg flex items-center gap-2 text-warning mb-2">
              <KeyRound size={20} />
              {t('safe_mode:recovery.title')}
            </h3>

            {!recoverySuccess ? (
              <>
                <p className="py-2 text-sm text-base-content/70">
                  {t('safe_mode:recovery.desc_format')}
                </p>

                <form onSubmit={handleRecoverySubmit} className="mt-4 form-control w-full">
                  <input
                    type="text"
                    className={`input input-bordered w-full font-mono text-center tracking-widest uppercase bg-base-200 outline-none ${
                      recoveryError
                        ? 'border-error/50 focus:border-error'
                        : 'border-base-content/10 focus:border-warning'
                    }`}
                    placeholder={t('safe_mode:recovery.placeholder')}
                    value={recoveryInput}
                    onChange={(e) => {
                      setRecoveryError(null);
                      setRecoveryInput(e.target.value);
                    }}
                    autoFocus
                  />

                  {recoveryError && (
                    <p className="text-xs text-error mt-2 text-center">{recoveryError}</p>
                  )}

                  <div className="modal-action mt-6 gap-2">
                    <button
                      type="button"
                      className="btn btn-ghost hover:bg-base-content/5 flex-1"
                      onClick={() => setView('pin')}
                    >
                      ← {t('common:actions.back')}
                    </button>
                    <button
                      type="submit"
                      className="btn btn-warning flex-1"
                      disabled={!recoveryInput.trim()}
                    >
                      {t('safe_mode:recovery.reset')}
                    </button>
                  </div>
                </form>
              </>
            ) : (
              <div className="flex flex-col items-center gap-4 py-4 text-center">
                <div className="w-14 h-14 rounded-2xl bg-success/10 flex items-center justify-center">
                  <ShieldCheck size={28} className="text-success" />
                </div>
                <div>
                  <p className="font-bold text-success">{t('safe_mode:recovery.success.title')}</p>
                  <p className="text-sm text-base-content/60 mt-1">
                    {t('safe_mode:recovery.success.desc')}
                  </p>
                </div>
                <button className="btn btn-success w-full" onClick={onClose}>
                  {t('common:actions.close')}
                </button>
              </div>
            )}
          </>
        )}
      </div>
      <form method="dialog" className="modal-backdrop bg-overlay-mask backdrop-blur-[2px]">
        <button onClick={cancellable ? onClose : undefined} disabled={!cancellable}>
          {t('common:actions.close')}
        </button>
      </form>
    </div>,
    document.body,
  );
}
