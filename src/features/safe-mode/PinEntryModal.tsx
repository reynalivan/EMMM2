import { useState, useEffect } from 'react';
import { useSettings } from '../../hooks/useSettings';
import { ShieldCheck, ShieldAlert, X } from 'lucide-react';

interface PinEntryModalProps {
  open: boolean;
  onClose: () => void;
  onSuccess: () => void;
  title?: string;
  description?: string;
}

export default function PinEntryModal({
  open,
  onClose,
  onSuccess,
  title = 'Enter PIN',
  description = 'A PIN is required to disable Safe Mode.',
}: PinEntryModalProps) {
  const [pin, setPin] = useState('');
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [lockoutTimer, setLockoutTimer] = useState<number>(0);

  const { verifyPin } = useSettings();

  const [prevOpen, setPrevOpen] = useState(open);

  if (open !== prevOpen) {
    setPrevOpen(open);
    if (open) {
      setPin('');
      setErrorMsg(null);
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

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (pin.length !== 6) {
      setErrorMsg('PIN must be 6 digits');
      return;
    }

    try {
      const status = await verifyPin(pin);
      if (status.valid) {
        onSuccess();
        onClose();
      } else {
        if (status.locked_seconds_remaining > 0) {
          setLockoutTimer(status.locked_seconds_remaining);
          setErrorMsg(`Too many failed attempts. Locked for ${status.locked_seconds_remaining}s`);
        } else {
          setErrorMsg(`Invalid PIN. ${status.attempts_remaining} attempts remaining.`);
        }
        setPin('');
      }
    } catch (err) {
      setErrorMsg(String(err));
    }
  };

  const isLocked = lockoutTimer > 0;

  if (!open) return null;

  return (
    <div className="modal modal-open">
      <div className="modal-box bg-base-300 border border-white/10 shadow-2xl safe-mode-modal relative">
        <button
          onClick={onClose}
          className="btn btn-sm btn-circle btn-ghost absolute right-2 top-2"
        >
          <X size={18} />
        </button>

        <h3 className="font-bold text-lg flex items-center gap-2 text-warning mb-2">
          {isLocked ? <ShieldAlert size={20} /> : <ShieldCheck size={20} />}
          {title}
        </h3>

        <p className="py-2 text-sm text-white/70">{description}</p>

        <form onSubmit={handleSubmit} className="mt-4 form-control w-full">
          <input
            type="password"
            className={`input input-bordered w-full text-center text-2xl tracking-[0.5em] transition-all bg-base-200 outline-none ${
              errorMsg
                ? 'border-error/50 focus:border-error'
                : 'border-white/10 focus:border-primary'
            }`}
            placeholder="••••••"
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
                Try again in {lockoutTimer} seconds
              </span>
            )}
          </div>

          <div className="modal-action mt-6 gap-2">
            <button
              type="button"
              className="btn btn-ghost hover:bg-white/5 flex-1"
              onClick={onClose}
            >
              Cancel
            </button>
            <button
              type="submit"
              className="btn btn-primary flex-1 shadow-lg shadow-primary/20"
              disabled={pin.length !== 6 || isLocked}
            >
              Verify
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
