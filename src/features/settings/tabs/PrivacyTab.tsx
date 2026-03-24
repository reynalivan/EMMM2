import { useState } from 'react';
import { Shield, ShieldAlert, KeyRound, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useSettings } from '../../../hooks/useSettings';
import PinModal from '../modals/PinModal';
import { useToastStore } from '../../../stores/useToastStore';
import { useSafeModeToggle as useSafeModeToggle } from '../../collections/hooks';
import ModeSwitchConfirmModal from '../../safe-mode/ModeSwitchConfirmModal';
import { commands } from '../../../lib/bindings';
import PinEntryModal from '../../safe-mode/PinEntryModal';
import RecoveryCodeModal from '../../safe-mode/RecoveryCodeModal';

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
  const { t } = useTranslation(['settings', 'safe_mode', 'common']);
  const { settings, saveSettingsAsync, setPinWithRecoveryAsync, verifyPin } = useSettings();
  const {
    toggleSafeMode,
    handleConfirmSwitch,
    handlePinSuccess,
    confirmModalOpen,
    confirmTargetSafeMode,
    closeConfirmModal,
    pinModalOpen: corridorPinModalOpen,
    closePinModal: closeCorridorPinModal,
  } = useSafeModeToggle();
  const { addToast } = useToastStore();

  const [modalMode, setModalMode] = useState<'unlock' | 'set_new'>('unlock');
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [pendingAction, setPendingAction] = useState<SafeModePendingAction>(null);
  const [keywordInput, setKeywordInput] = useState('');
  const [recoveryCode, setRecoveryCode] = useState<string | null>(null);

  if (!settings) return <div>{t('common:status.loading')}</div>;

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
  };

  // Toggle Safe Mode — uses the shared hook which shows confirmation modal
  const handleToggleSafeMode = () => {
    toggleSafeMode();
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
        const isValid = await verifyPin(pin);
        if (isValid) {
          setIsModalOpen(false);
          if (pendingAction) {
            await pendingAction();
          }
          setPendingAction(null);
          return;
        }

        const status = await commands.getPinStatus();
        if (status.is_locked) {
          addToast(
            'error',
            t('safe_mode:pin_entry.error.lockout', { seconds: status.lockout_seconds_remaining }),
          );
        } else {
          addToast(
            'error',
            t('safe_mode:pin_entry.error.invalid', { count: status.attempts_remaining }),
          );
        }
      } catch (e) {
        console.error(e);
        addToast('error', t('safe_mode:pin_entry.error.system'));
      }
    } else {
      try {
        const code = await setPinWithRecoveryAsync(pin);
        setIsModalOpen(false);
        setRecoveryCode(code);
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
      addToast('warning', t('settings:privacy.keywords_exists'));
      return;
    }

    const nextKeywords = [...effectiveKeywords, nextKeyword];
    setKeywordInput('');
    try {
      await persistSafeModeSettings({ keywords: nextKeywords });
    } catch (error) {
      console.error(error);
      addToast(
        'error',
        t('settings:privacy.keywords_update_failed', { error: toErrorMessage(error) }),
      );
    }
  };

  const handleRemoveKeyword = async (keyword: string) => {
    const nextKeywords = effectiveKeywords.filter((value) => value !== keyword);
    try {
      await persistSafeModeSettings({ keywords: nextKeywords });
    } catch (error) {
      console.error(error);
      addToast(
        'error',
        t('settings:privacy.keywords_update_failed', { error: toErrorMessage(error) }),
      );
    }
  };

  const handleToggleForceExclusive = async () => {
    try {
      await persistSafeModeSettings({
        force_exclusive_mode: !settings.safe_mode.force_exclusive_mode,
      });
    } catch (error) {
      console.error(error);
      addToast(
        'error',
        t('settings:privacy.protection_update_failed', { error: toErrorMessage(error) }),
      );
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
                <h3 className="card-title text-lg">{t('settings:privacy.title')}</h3>
                <p className="text-sm opacity-70 max-w-md mt-1">{t('settings:privacy.desc')}</p>
              </div>
            </div>
            <input
              type="checkbox"
              className="toggle toggle-success toggle-lg"
              checked={settings.safe_mode.enabled}
              onChange={handleToggleSafeMode}
              aria-label={t('safe_mode:settings.toggle')}
            />
          </div>

          <div className="mt-6 pt-6 border-t border-base-300">
            <h4 className="text-sm font-bold uppercase tracking-wider opacity-50 mb-3">
              {t('settings:privacy.keywords_title')}
            </h4>
            <div className="flex items-center gap-2 mb-3">
              <input
                type="text"
                className="input input-sm input-bordered flex-1"
                placeholder={t('settings:privacy.keywords_placeholder')}
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
                {t('settings:privacy.keywords_add')}
              </button>
            </div>
            <div className="flex flex-wrap gap-2">
              {effectiveKeywords.map((keyword) => (
                <span key={keyword} className="badge badge-neutral gap-1 pl-3 pr-2 py-3">
                  {keyword}
                  <button
                    type="button"
                    className="btn btn-ghost btn-xs btn-circle"
                    aria-label={t('safe_mode:settings.keywords.remove', { keyword })}
                    onClick={() => void handleRemoveKeyword(keyword)}
                  >
                    <X size={12} />
                  </button>
                </span>
              ))}
              {effectiveKeywords.length === 0 && (
                <span className="text-xs opacity-60">{t('settings:privacy.keywords_empty')}</span>
              )}
            </div>

            <div className="form-control mt-4">
              <label className="label cursor-pointer justify-start gap-3">
                <input
                  type="checkbox"
                  className="toggle toggle-sm"
                  checked={!!settings.safe_mode.force_exclusive_mode}
                  onChange={() => void handleToggleForceExclusive()}
                />
                <span className="label-text font-medium">
                  {t('settings:privacy.protection_title')}
                </span>
              </label>
              <p className="text-xs opacity-70 pl-12">{t('settings:privacy.protection_desc')}</p>
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
                <h3 className="card-title text-lg">{t('settings:privacy.security_title')}</h3>
                <p className="text-sm opacity-70">
                  {hasPin
                    ? t('settings:privacy.security_status_set')
                    : t('settings:privacy.security_status_none')}
                </p>
                {hasPin && !!settings.safe_mode.recovery_code_hash && (
                  <p className="text-xs text-success/70 mt-0.5">
                    ✓ {t('settings:privacy.security_recovery_configured')}
                  </p>
                )}
              </div>
            </div>
            <div className="flex gap-2">
              {hasPin && (
                <button
                  className="btn btn-ghost btn-sm text-error/70 hover:text-error hover:bg-error/10"
                  onClick={async () => {
                    try {
                      await saveSettingsAsync({
                        ...settings,
                        safe_mode: {
                          ...settings.safe_mode,
                          pin_hash: null,
                          recovery_code_hash: null,
                        },
                      });
                      addToast('success', t('safe_mode:recovery.removed'));
                    } catch (e) {
                      addToast(
                        'error',
                        t('safe_mode:recovery.remove_failed', { error: String(e) }),
                      );
                    }
                  }}
                >
                  {t('settings:privacy.security_remove')}
                </button>
              )}
              <button className="btn btn-neutral" onClick={handleChangePin}>
                {hasPin
                  ? t('settings:privacy.security_change')
                  : t('settings:privacy.security_set')}
              </button>
            </div>
          </div>
        </div>
      </div>

      <PinModal
        isOpen={isModalOpen}
        onClose={() => setIsModalOpen(false)}
        onSuccess={handleModalSuccess}
        isSettingNew={modalMode === 'set_new'}
        title={
          modalMode === 'set_new'
            ? t('settings:privacy.security_modal_set_title')
            : t('settings:privacy.security_modal_unlock_title')
        }
        description={
          modalMode === 'set_new'
            ? t('settings:privacy.security_modal_set_desc')
            : t('settings:privacy.security_modal_unlock_desc')
        }
      />

      {/* Recovery Code Modal — shown once after setting a new PIN */}
      <RecoveryCodeModal
        open={!!recoveryCode}
        recoveryCode={recoveryCode ?? ''}
        onClose={() => setRecoveryCode(null)}
      />

      {/* Confirmation Modal for Corridor Switch */}
      <ModeSwitchConfirmModal
        open={confirmModalOpen}
        targetSafeMode={confirmTargetSafeMode}
        onClose={closeConfirmModal}
        onConfirm={handleConfirmSwitch}
      />

      {/* PIN Entry Modal for Safe→Unsafe corridor transition */}
      <PinEntryModal
        open={corridorPinModalOpen}
        onClose={closeCorridorPinModal}
        onSuccess={async () => {
          handlePinSuccess();
        }}
      />
    </div>
  );
}
