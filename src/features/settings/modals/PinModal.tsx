import { useState, useEffect, useRef } from 'react';
import { X, Lock } from 'lucide-react';

interface PinModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSuccess: (pin: string) => void;
  title?: string;
  description?: string;
  isSettingNew?: boolean;
}

export default function PinModal({
  isOpen,
  onClose,
  onSuccess,
  title = 'Enter PIN',
  description = 'Please enter your PIN to continue.',
  isSettingNew = false,
}: PinModalProps) {
  const [pin, setPin] = useState('');
  const [confirmPin, setConfirmPin] = useState('');
  const [error, setError] = useState('');
  const inputRef = useRef<HTMLInputElement>(null);

  // Reset state when modal opens
  const [prevIsOpen, setPrevIsOpen] = useState(isOpen);
  if (isOpen !== prevIsOpen) {
    setPrevIsOpen(isOpen);
    if (isOpen) {
      setPin('');
      setConfirmPin('');
      setError('');
    }
  }

  useEffect(() => {
    if (isOpen) {
      setTimeout(() => inputRef.current?.focus(), 100);
    }
  }, [isOpen]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    setError('');

    if (pin.length !== 6) {
      setError('PIN must be exactly 6 digits');
      return;
    }

    if (isSettingNew && pin !== confirmPin) {
      setError('PINs do not match');
      return;
    }

    onSuccess(pin);
    // Modal closing is handled by parent usually, or we can close here
    // onClose();
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm p-4">
      <div className="bg-base-100 rounded-xl shadow-2xl w-full max-w-sm overflow-hidden border border-base-300 transform transition-all scale-100">
        <div className="flex items-center justify-between p-4 border-b border-base-300 bg-base-200/50">
          <h3 className="font-bold text-lg flex items-center gap-2">
            <Lock size={18} className="text-primary" />
            {title}
          </h3>
          <button onClick={onClose} className="btn btn-ghost btn-sm btn-square">
            <X size={20} />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="p-6 space-y-4">
          <p className="text-sm opacity-70">{description}</p>

          <div className="form-control">
            <label className="label py-1">
              <span className="label-text font-medium">{isSettingNew ? 'New PIN' : 'PIN'}</span>
            </label>
            <input
              ref={inputRef}
              type="password"
              className="input input-bordered w-full text-center tracking-[0.5em] font-mono text-lg"
              value={pin}
              onChange={(e) => setPin(e.target.value.replace(/[^0-9]/g, ''))} // Numbers only usually
              maxLength={6}
              inputMode="numeric"
              autoComplete="off"
            />
          </div>

          {isSettingNew && (
            <div className="form-control">
              <label className="label py-1">
                <span className="label-text font-medium">Confirm New PIN</span>
              </label>
              <input
                type="password"
                className="input input-bordered w-full text-center tracking-[0.5em] font-mono text-lg"
                value={confirmPin}
                onChange={(e) => setConfirmPin(e.target.value.replace(/[^0-9]/g, ''))}
                maxLength={6}
                inputMode="numeric"
                autoComplete="off"
              />
            </div>
          )}

          {error && (
            <div className="alert alert-error text-xs py-2">
              <span>{error}</span>
            </div>
          )}

          <div className="modal-action justify-center mt-6">
            <button type="submit" className="btn btn-primary w-full">
              {isSettingNew ? 'Set PIN' : 'Unlock'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
