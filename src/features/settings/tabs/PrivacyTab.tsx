import { useState } from 'react';
import { Shield, ShieldAlert, KeyRound, X } from 'lucide-react';
import { useSettings } from '../../../hooks/useSettings';
import PinModal from '../modals/PinModal';
import { useToastStore } from '../../../stores/useToastStore';
import { useAppStore } from '../../../stores/useAppStore';

type SafeModePendingAction = (() => Promise<void>) | null;

function normalizeKeyword(value: string): string {
  return value.trim().toLowerCase();
}

function normalizeKeywordList(values: string[]): string[] {
  const next: string[] = [];
  for (const value of values) {
    const normalized = normalizeKeyword(value);
    if (!normalized || next.includes(normalized)) {
      continue;
    }
    next.push(normalized);
  }
  return next;
}

function toErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}

export default function PrivacyTab() {
  const { settings, saveSettingsAsync, setPinAsync, verifyPin } = useSettings();
  const { setSafeMode } = useAppStore();
  const { addToast } = useToastStore();

  const [modalMode, setModalMode] = useState<'unlock' | 'set_new'>('unlock');
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [pendingAction, setPendingAction] = useState<SafeModePendingAction>(null);
  const [keywordInput, setKeywordInput] = useState('');

  if (!settings) return <div>Loading...</div>;

  const hasPin = !!settings.safe_mode.pin_hash;
  const effectiveKeywords = normalizeKeywordList(settings.safe_mode.keywords);

  const persistSafeModeSettings = async (patch: Partial<typeof settings.safe_mode>) => {
    const nextSafeMode = {
      ...settings.safe_mode,
      ...patch,
      keywords: normalizeKeywordList(patch.keywords ?? effectiveKeywords),
    };

    await saveSettingsAsync({
      ...settings,
      safe_mode: nextSafeMode,
    });

    if (typeof patch.enabled === 'boolean') {
      await setSafeMode(patch.enabled);
    }
  };

  // Toggle Safe Mode
  const handleToggleSafeMode = async () => {
    const newState = !settings.safe_mode.enabled;

    try {
      if (!newState && hasPin) {
        setModalMode('unlock');
        setPendingAction(() => async () => {
          await persistSafeModeSettings({ enabled: newState });
        });
        setIsModalOpen(true);
      } else {
        await persistSafeModeSettings({ enabled: newState });
      }
    } catch (error) {
      console.error(error);
      addToast('error', `Safe Mode update failed: ${toErrorMessage(error)}`);
    }
  };

  // Change PIN Flow
  const handleChangePin = () => {
    if (hasPin) {
      setModalMode('unlock');
      setPendingAction(() => async () => {
        setTimeout(() => {
          setModalMode('set_new');
          setPendingAction(null);
          setIsModalOpen(true);
        }, 200);
      });
      setIsModalOpen(true);
    } else {
      setModalMode('set_new');
      setPendingAction(null);
      setIsModalOpen(true);
    }
  };

  const handleModalSuccess = async (pin: string) => {
    if (modalMode === 'unlock') {
      try {
        const result = await verifyPin(pin);
        if (result.valid) {
          setIsModalOpen(false);
          if (pendingAction) {
            await pendingAction();
          }
          setPendingAction(null);
          return;
        }

        if (result.locked_seconds_remaining > 0) {
          addToast('error', `PIN locked. Try again in ${result.locked_seconds_remaining}s.`);
        } else {
          addToast('error', `Incorrect PIN. ${result.attempts_remaining} attempt(s) remaining.`);
        }
      } catch (e) {
        console.error(e);
        addToast('error', 'Verification Failed: System error.');
      }
    } else {
      try {
        await setPinAsync(pin);
        setIsModalOpen(false);
      } catch (e) {
        console.error(e);
      }
    }
  };

  const handleAddKeyword = async () => {
    const nextKeyword = normalizeKeyword(keywordInput);
    if (!nextKeyword) {
      return;
    }
    if (effectiveKeywords.includes(nextKeyword)) {
      addToast('warning', 'Keyword already exists.');
      return;
    }

    const nextKeywords = [...effectiveKeywords, nextKeyword];
    setKeywordInput('');
    try {
      await persistSafeModeSettings({ keywords: nextKeywords });
    } catch (error) {
      console.error(error);
      addToast('error', `Keyword update failed: ${toErrorMessage(error)}`);
    }
  };

  const handleRemoveKeyword = async (keyword: string) => {
    const nextKeywords = effectiveKeywords.filter((value) => value !== keyword);
    try {
      await persistSafeModeSettings({ keywords: nextKeywords });
    } catch (error) {
      console.error(error);
      addToast('error', `Keyword update failed: ${toErrorMessage(error)}`);
    }
  };

  const handleToggleForceExclusive = async () => {
    try {
      await persistSafeModeSettings({
        force_exclusive_mode: !settings.safe_mode.force_exclusive_mode,
      });
    } catch (error) {
      console.error(error);
      addToast('error', `Safe Mode filter update failed: ${toErrorMessage(error)}`);
    }
  };

  return (
    <div className="space-y-8">
      {/* Safe Mode Section */}
      <div className="card bg-base-200 shadow-sm border border-base-300">
        <div className="card-body">
          <div className="flex items-start justify-between gap-4">
            <div className="flex gap-4">
              <div
                className={`p-3 rounded-xl ${settings.safe_mode.enabled ? 'bg-success/10 text-success' : 'bg-warning/10 text-warning'}`}
              >
                {settings.safe_mode.enabled ? <Shield size={24} /> : <ShieldAlert size={24} />}
              </div>
              <div>
                <h3 className="card-title text-lg">Safe Mode</h3>
                <p className="text-sm opacity-70 max-w-md mt-1">
                  When enabled, mods tagged with sensitive keywords are hidden from the library.
                  Disabling this mode may require authentication if a PIN is set.
                </p>
              </div>
            </div>
            <input
              type="checkbox"
              className="toggle toggle-success toggle-lg"
              checked={settings.safe_mode.enabled}
              onChange={handleToggleSafeMode}
              aria-label="Toggle safe mode"
            />
          </div>

          <div className="mt-6 pt-6 border-t border-base-300">
            <h4 className="text-sm font-bold uppercase tracking-wider opacity-50 mb-3">
              Filtered Keywords
            </h4>
            <div className="flex items-center gap-2 mb-3">
              <input
                type="text"
                className="input input-sm input-bordered flex-1"
                placeholder="Add keyword (e.g. nsfw)"
                value={keywordInput}
                onChange={(event) => setKeywordInput(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === 'Enter') {
                    event.preventDefault();
                    void handleAddKeyword();
                  }
                }}
              />
              <button
                type="button"
                className="btn btn-sm btn-primary"
                onClick={() => void handleAddKeyword()}
              >
                Add
              </button>
            </div>
            <div className="flex flex-wrap gap-2">
              {effectiveKeywords.map((keyword) => (
                <span key={keyword} className="badge badge-neutral gap-1 pl-3 pr-2 py-3">
                  {keyword}
                  <button
                    type="button"
                    className="btn btn-ghost btn-xs btn-circle"
                    aria-label={`Remove ${keyword}`}
                    onClick={() => void handleRemoveKeyword(keyword)}
                  >
                    <X size={12} />
                  </button>
                </span>
              ))}
              {effectiveKeywords.length === 0 && (
                <span className="text-xs opacity-60">No filtered keywords configured.</span>
              )}
            </div>

            <div className="form-control mt-4">
              <label className="label cursor-pointer justify-start gap-3">
                <input
                  type="checkbox"
                  className="toggle toggle-sm"
                  checked={settings.safe_mode.force_exclusive_mode}
                  onChange={() => void handleToggleForceExclusive()}
                />
                <span className="label-text font-medium">Extra Keyword Protection</span>
              </label>
              <p className="text-xs opacity-70 pl-12">
                When this is ON, mods that match filtered keywords are hidden even if they are
                marked as safe.
              </p>
            </div>
          </div>
        </div>
      </div>

      {/* PIN Management */}
      <div className="card bg-base-200 shadow-sm border border-base-300">
        <div className="card-body">
          <div className="flex items-center justify-between">
            <div className="flex gap-4 items-center">
              <div className="p-3 rounded-xl bg-primary/10 text-primary">
                <KeyRound size={24} />
              </div>
              <div>
                <h3 className="card-title text-lg">Security PIN</h3>
                <p className="text-sm opacity-70">
                  {hasPin
                    ? 'A PIN is currently set. It is required to disable Safe Mode.'
                    : 'No PIN set. Anyone can toggle Safe Mode freely.'}
                </p>
              </div>
            </div>
            <button className="btn btn-neutral" onClick={handleChangePin}>
              {hasPin ? 'Change PIN' : 'Set PIN'}
            </button>
          </div>
        </div>
      </div>

      <PinModal
        isOpen={isModalOpen}
        onClose={() => setIsModalOpen(false)}
        onSuccess={handleModalSuccess}
        isSettingNew={modalMode === 'set_new'}
        title={modalMode === 'set_new' ? 'Set Security PIN' : 'Unlock Settings'}
        description={
          modalMode === 'set_new'
            ? 'Create a secure PIN to protect Safe Mode settings.'
            : 'Enter your current PIN to proceed.'
        }
      />
    </div>
  );
}
