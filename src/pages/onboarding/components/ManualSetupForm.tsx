import { formatAppError } from '../../../shared/lib/appError';
import { useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useForm, useWatch } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z } from 'zod';
import { commands } from '../../../shared/api/tauri/bindings';
import { open } from '@tauri-apps/plugin-dialog';
import { ArrowLeft, FolderOpen, Loader2, AlertCircle } from 'lucide-react';
import type { GameConfig } from '@/entities/game';
import { GAME_OPTIONS, GameType } from '@/entities/game';
import {
  GameExecutableField,
  useGameExecutableDiscovery,
  validateSelectedGameExecutables,
} from '@/features/game-executable-detection';

interface ManualSetupFormProps {
  onBack: () => void;
  onSuccess: (game: GameConfig, selectedExecutable?: string) => void;
}

type FormData = {
  gameType: string;
  path: string;
};

const GAME_TYPES: Record<(typeof GAME_OPTIONS)[number]['value'], GameType> = {
  GIMI: GameType.GIMI,
  SRMI: GameType.SRMI,
  ZZMI: GameType.ZZMI,
  WWMI: GameType.WWMI,
  EFMI: GameType.EFMI,
};

function numericGameType(value: string): GameType | undefined {
  const option = GAME_OPTIONS.find((entry) => entry.value === value);
  return option ? GAME_TYPES[option.value] : undefined;
}

export function ManualSetupForm({ onBack, onSuccess }: ManualSetupFormProps) {
  const { t } = useTranslation('onboarding');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [serverError, setServerError] = useState<string | null>(null);
  const [executables, setExecutables] = useState<
    Record<string, { value: string; detected: boolean }>
  >({});
  const revisionRef = useRef(0);
  const submittingRef = useRef(false);

  useEffect(
    () => () => {
      revisionRef.current += 1;
    },
    [],
  );

  const {
    register,
    handleSubmit,
    setValue,
    control,
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
  const gameType = useWatch({ control, name: 'gameType' });
  const importerPath = useWatch({ control, name: 'path' });
  const numericType = numericGameType(gameType);
  const roots = useMemo(() => (importerPath ? [importerPath] : []), [importerPath]);
  const discovery = useGameExecutableDiscovery(roots);

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
    if (submittingRef.current) return;
    submittingRef.current = true;
    const revision = ++revisionRef.current;
    setIsSubmitting(true);
    setServerError(null);
    discovery.cancel();
    try {
      const game = await commands.addGameManual(data.gameType, data.path);
      const executable = executables[data.gameType];
      const configuredGame = executable
        ? { ...game, game_exe: executable.value.trim() || null }
        : game;
      if (executable?.detected) {
        await validateSelectedGameExecutables(
          [configuredGame],
          new Map([[game.id, executable.value]]),
        );
      }
      if (revision !== revisionRef.current) return;
      if (executable?.detected) onSuccess(configuredGame, executable.value);
      else onSuccess(configuredGame);
    } catch (err) {
      if (revision === revisionRef.current) setServerError(formatAppError(err));
    } finally {
      submittingRef.current = false;
      if (revision === revisionRef.current) setIsSubmitting(false);
    }
  };

  return (
    <div className="min-h-screen bg-base-100 flex items-center justify-center p-6">
      <div className="w-full max-w-lg space-y-7">
        {/* Header */}
        <div className="flex items-center gap-4">
          <button
            onClick={() => {
              revisionRef.current += 1;
              discovery.cancel();
              onBack();
            }}
            className="btn btn-ghost min-h-11 gap-2 px-3 whitespace-nowrap"
          >
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
        <form
          onSubmit={(event) => {
            void handleSubmit(onSubmit)(event);
          }}
          className="space-y-6"
        >
          {/* Game Type */}
          <div className="form-control w-full">
            <label className="label">
              <span className="label-text font-semibold">{t('manual_setup.game_type')}</span>
            </label>
            <select
              disabled={isSubmitting}
              className={`select h-11 w-full ${errors.gameType ? 'select-error' : ''}`}
              aria-invalid={errors.gameType ? 'true' : 'false'}
              aria-describedby="game-type-feedback"
              {...register('gameType')}
            >
              <option value="">{t('manual_setup.game_type_placeholder')}</option>
              {GAME_OPTIONS.map((opt) => (
                <option key={opt.value} value={opt.value}>
                  {opt.label}
                </option>
              ))}
            </select>
            <div id="game-type-feedback" className="label min-h-7 py-1">
              {errors.gameType && (
                <span className="label-text-alt text-error">{errors.gameType.message}</span>
              )}
            </div>
          </div>

          {/* Path */}
          <div className="form-control w-full">
            <label className="label">
              <span className="label-text font-semibold">{t('manual_setup.game_folder')}</span>
            </label>
            <div className="flex flex-col gap-2 sm:flex-row">
              <input
                type="text"
                className={`input h-11 min-w-0 flex-1 truncate ${errors.path ? 'input-error' : ''}`}
                placeholder={t('manual_setup.game_folder_placeholder')}
                readOnly
                aria-invalid={errors.path ? 'true' : 'false'}
                aria-describedby="game-folder-feedback"
                {...register('path')}
              />
              <button
                type="button"
                disabled={isSubmitting}
                className="btn btn-primary min-h-11 shrink-0 whitespace-nowrap"
                onClick={handleBrowse}
              >
                <FolderOpen className="w-4 h-4" />
                {t('manual_setup.browse')}
              </button>
            </div>
            <div id="game-folder-feedback" className="label min-h-7 py-1">
              {errors.path ? (
                <span className="label-text-alt text-error">{errors.path.message}</span>
              ) : (
                <span className="label-text-alt text-muted block w-full whitespace-normal break-words">
                  {t('manual_setup.hint')}
                </span>
              )}
            </div>
          </div>

          {numericType !== undefined && (
            <GameExecutableField
              key={gameType}
              gameType={numericType}
              value={executables[gameType]?.value ?? ''}
              inputId="manual-game-executable"
              xxmiRoots={roots}
              discovery={discovery}
              disabled={isSubmitting}
              onChange={(value, detected) => {
                setExecutables((current) => ({ ...current, [gameType]: { value, detected } }));
                setServerError(null);
              }}
            />
          )}
          <p className="text-sm text-base-content/60">
            {t('settings:games.executable_detection.setup_later')}
          </p>

          {/* Actions */}
          <button
            type="submit"
            className="btn btn-primary btn-block min-h-11 gap-2 whitespace-nowrap"
            disabled={isSubmitting}
          >
            {isSubmitting && <Loader2 className="w-5 h-5 animate-spin" />}
            {t('manual_setup.submit_button')}
          </button>
        </form>
      </div>
    </div>
  );
}
