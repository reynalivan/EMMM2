import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z } from 'zod';
import { commands } from '../../lib/bindings';
import { open } from '@tauri-apps/plugin-dialog';
import { ArrowLeft, FolderOpen, Loader2, AlertCircle } from 'lucide-react';
import type { GameConfig } from '../../types/game';
import { GAME_OPTIONS } from '../../types/game';

interface ManualSetupFormProps {
  onBack: () => void;
  onSuccess: (game: GameConfig) => void;
}

type FormData = {
  gameType: string;
  path: string;
};

export function ManualSetupForm({ onBack, onSuccess }: ManualSetupFormProps) {
  const { t } = useTranslation('onboarding');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [serverError, setServerError] = useState<string | null>(null);

  const {
    register,
    handleSubmit,
    setValue,
    formState: { errors },
  } = useForm<FormData>({
    resolver: zodResolver(
      z.object({
        gameType: z.string().min(1, t('manual_setup.validation.game_type_required')),
        path: z.string().min(1, t('manual_setup.validation.path_required')),
      }),
    ),
    defaultValues: { gameType: '', path: '' },
  });

  const handleBrowse = async () => {
    const selectedPath = await open({
      directory: true,
      multiple: false,
      title: t('manual_setup.validation.select_folder_title'),
    });
    if (selectedPath) {
      setValue('path', selectedPath, { shouldValidate: true });
      setServerError(null);
    }
  };

  const onSubmit = async (data: FormData) => {
    setIsSubmitting(true);
    setServerError(null);
    try {
      const game = await commands.addGameManual({
        gameType: data.gameType,
        path: data.path,
      });
      onSuccess(game);
    } catch (err) {
      setServerError(String(err));
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="min-h-screen bg-base-100 flex items-center justify-center p-6">
      <div className="max-w-md w-full space-y-6">
        {/* Header */}
        <div className="flex items-center gap-4">
          <button onClick={onBack} className="btn btn-ghost btn-sm gap-2">
            <ArrowLeft className="w-5 h-5" />
            {t('common:actions.back')}
          </button>
          <div className="flex-1">
            <h2 className="text-2xl font-bold">{t('manual_setup.title')}</h2>
            <p className="text-base-content/60 text-sm">{t('manual_setup.subtitle')}</p>
          </div>
        </div>

        {/* Server Error */}
        {serverError && (
          <div role="alert" className="alert alert-error alert-soft">
            <AlertCircle className="w-5 h-5" />
            <span>{serverError}</span>
          </div>
        )}

        {/* Form */}
        <form onSubmit={handleSubmit(onSubmit)} className="space-y-6">
          {/* Game Type */}
          <div className="form-control w-full">
            <label className="label">
              <span className="label-text font-semibold">{t('manual_setup.game_type')}</span>
            </label>
            <select
              className={`select select-bordered w-full ${errors.gameType ? 'select-error' : ''}`}
              {...register('gameType')}
            >
              <option value="">{t('manual_setup.game_type_placeholder')}</option>
              {GAME_OPTIONS.map((opt) => (
                <option key={opt.value} value={opt.value}>
                  {opt.label}
                </option>
              ))}
            </select>
            {errors.gameType && (
              <label className="label">
                <span className="label-text-alt text-error">{errors.gameType.message}</span>
              </label>
            )}
          </div>

          {/* Path */}
          <div className="form-control w-full">
            <label className="label">
              <span className="label-text font-semibold">{t('manual_setup.game_folder')}</span>
            </label>
            <div className="flex gap-2">
              <input
                type="text"
                className={`input input-bordered flex-1 truncate ${errors.path ? 'input-error' : ''}`}
                placeholder={t('manual_setup.game_folder_placeholder')}
                readOnly
                {...register('path')}
              />
              <button type="button" className="btn btn-primary" onClick={handleBrowse}>
                <FolderOpen className="w-4 h-4" />
                {t('manual_setup.browse')}
              </button>
            </div>
            {errors.path && (
              <label className="label">
                <span className="label-text-alt text-error">{errors.path.message}</span>
              </label>
            )}
            <label className="label">
              <span className="label-text-alt text-base-content/50">{t('manual_setup.hint')}</span>
            </label>
          </div>

          {/* Actions */}
          <button type="submit" className="btn btn-primary btn-block gap-2" disabled={isSubmitting}>
            {isSubmitting ? (
              <Loader2 className="w-5 h-5 animate-spin" />
            ) : (
              t('manual_setup.submit_button')
            )}
          </button>
        </form>
      </div>
    </div>
  );
}
