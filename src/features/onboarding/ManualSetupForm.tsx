import { useState } from 'react';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z } from 'zod';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { ArrowLeft, FolderOpen, Loader2, AlertCircle } from 'lucide-react';

const GAME_OPTIONS = [
  { value: 'GIMI', label: 'Genshin Impact (GIMI)' },
  { value: 'SRMI', label: 'Honkai Star Rail (SRMI)' },
  { value: 'WWMI', label: 'Wuthering Waves (WWMI)' },
  { value: 'ZZMI', label: 'Zenless Zone Zero (ZZMI)' },
  { value: 'EFMI', label: 'Arknight Endfield (EFMI)' },
] as const;

const schema = z.object({
  gameType: z.string().min(1, 'Please select a game type'),
  path: z.string().min(1, 'Please select a game folder'),
});

type FormData = z.infer<typeof schema>;

interface GameConfig {
  id: string;
  name: string;
  game_type: string;
  path: string;
  mods_path: string;
  launcher_path: string;
  launch_args: string | null;
}

interface Props {
  onBack: () => void;
  onComplete: (game: GameConfig) => void;
}

export default function ManualSetupForm({ onBack, onComplete }: Props) {
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [serverError, setServerError] = useState<string | null>(null);

  const {
    register,
    handleSubmit,
    setValue,
    watch,
    formState: { errors },
  } = useForm<FormData>({
    resolver: zodResolver(schema),
    defaultValues: { gameType: '', path: '' },
  });

  const currentPath = watch('path');

  const handleBrowse = async () => {
    const selectedPath = await open({
      directory: true,
      multiple: false,
      title: 'Select 3DMigoto game folder',
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
      const game = await invoke<GameConfig>('add_game_manual', {
        gameType: data.gameType,
        path: data.path,
      });
      onComplete(game);
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
        <div className="flex items-center gap-3">
          <button id="btn-back" className="btn btn-ghost btn-circle btn-sm" onClick={onBack}>
            <ArrowLeft className="w-5 h-5" />
          </button>
          <div>
            <h2 className="text-2xl font-bold">Manual Setup</h2>
            <p className="text-base-content/60 text-sm">Add a 3DMigoto game instance manually</p>
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
        <form onSubmit={handleSubmit(onSubmit)} className="space-y-5">
          {/* Game Type Select */}
          <fieldset className="fieldset">
            <legend className="fieldset-legend">Game Type</legend>
            <select
              id="select-game-type"
              className={`select select-primary w-full ${errors.gameType ? 'select-error' : ''}`}
              {...register('gameType')}
            >
              <option value="">Select a game...</option>
              {GAME_OPTIONS.map((opt) => (
                <option key={opt.value} value={opt.value}>
                  {opt.label}
                </option>
              ))}
            </select>
            {errors.gameType && (
              <p className="text-error text-sm mt-1">{errors.gameType.message}</p>
            )}
          </fieldset>

          {/* Folder Path */}
          <fieldset className="fieldset">
            <legend className="fieldset-legend">Game Folder</legend>
            <div className="flex gap-2">
              <input
                id="input-game-path"
                type="text"
                className={`input input-primary flex-1 ${errors.path ? 'input-error' : ''}`}
                placeholder="Click Browse to select folder..."
                readOnly
                value={currentPath}
                {...register('path')}
              />
              <button
                id="btn-browse"
                type="button"
                className="btn btn-outline btn-primary"
                onClick={handleBrowse}
              >
                <FolderOpen className="w-4 h-4" />
                Browse
              </button>
            </div>
            {errors.path && <p className="text-error text-sm mt-1">{errors.path.message}</p>}
            <p className="text-base-content/40 text-xs mt-1">
              Select the root folder containing d3dx.ini, d3d11.dll, and /Mods
            </p>
          </fieldset>

          {/* Submit */}
          <button
            id="btn-add-game"
            type="submit"
            className="btn btn-primary btn-block btn-lg"
            disabled={isSubmitting}
          >
            {isSubmitting ? (
              <>
                <Loader2 className="w-5 h-5 animate-spin" />
                Validating...
              </>
            ) : (
              'Add Game'
            )}
          </button>
        </form>
      </div>
    </div>
  );
}
