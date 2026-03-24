import { useEffect, useRef } from 'react';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import * as z from 'zod';
import { open } from '@tauri-apps/plugin-dialog';
import { X, FolderOpen } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { TFunction } from 'i18next';
import type { GameConfig } from '../../../hooks/useSettings';
import { GameType } from '../../../types/game';
import { pathsEqual } from '../../../lib/pathKey';

function getGameSchema(t: TFunction) {
  return z.object({
    id: z.string().optional(),
    name: z.string().min(1, t('games.form.validation.name_required')),
    game_type: z.nativeEnum(GameType),
    mod_path: z
      .string()
      .min(1, t('games.form.validation.path_required'))
      .refine((value) => !/[?*<>|]/.test(value), t('games.form.validation.path_invalid')),
    game_exe: z.string().min(1, t('games.form.validation.exe_required')),
    loader_exe: z.string().nullable().optional(), // Can be empty
    launch_args: z.string().nullable().optional(),
  });
}

type GameFormData = z.infer<ReturnType<typeof getGameSchema>>;

interface GameFormModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSave: (game: GameConfig) => void;
  initialData?: GameConfig | null;
  existingModPaths: string[];
}

export default function GameFormModal({
  isOpen,
  onClose,
  onSave,
  initialData,
  existingModPaths,
}: GameFormModalProps) {
  const { t } = useTranslation('settings');
  const schema = getGameSchema(t);
  const {
    register,
    handleSubmit,
    setValue,
    reset,
    formState: { errors, isSubmitting, isValid },
  } = useForm<GameFormData>({
    resolver: zodResolver(schema),
    mode: 'onChange',
    defaultValues: {
      game_type: GameType.GIMI,
      loader_exe: '',
      launch_args: '',
    },
  });

  const dialogRef = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    if (isOpen) {
      dialogRef.current?.showModal();
    } else {
      dialogRef.current?.close();
    }
  }, [isOpen]);

  const modPathField = register('mod_path', {
    validate: (value) => {
      const isDuplicate = existingModPaths.some((path) => pathsEqual(path, value));
      return isDuplicate ? t('games.form.validation.path_duplicate') : true;
    },
  });

  useEffect(() => {
    if (isOpen) {
      if (initialData) {
        reset({
          id: initialData.id,
          name: initialData.name,
          game_type: initialData.game_type as GameType,
          mod_path: initialData.mod_path,
          game_exe: initialData.game_exe,
          loader_exe: initialData.loader_exe || '',
          launch_args: initialData.launch_args || '',
        });
      } else {
        reset({
          id: undefined,
          name: '',
          game_type: GameType.GIMI,
          mod_path: '',
          game_exe: '',
          loader_exe: '',
          launch_args: '',
        });
      }
    }
  }, [isOpen, initialData, reset]);

  const onSubmit = (data: GameFormData) => {
    const gameConfig: GameConfig = {
      id: data.id || crypto.randomUUID(),
      name: data.name,
      game_type: data.game_type,
      mod_path: data.mod_path,
      game_exe: data.game_exe,
      loader_exe: data.loader_exe || null,
      launch_args: data.launch_args || null,
    };
    onSave(gameConfig);
    onClose();
  };

  const pickFolder = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: t('games.form.validation.pick_folder_title'),
      });
      if (selected && typeof selected === 'string') {
        setValue('mod_path', selected, { shouldValidate: true });
      }
    } catch (err) {
      console.error(t('games.form.validation.pick_folder_error'), err);
    }
  };

  const pickExe = async (field: 'game_exe' | 'loader_exe') => {
    try {
      const selected = await open({
        directory: false,
        multiple: false,
        title: t(
          field === 'game_exe'
            ? 'games.form.validation.pick_exe_title'
            : 'games.form.validation.pick_loader_title',
        ),
        filters: [{ name: 'Executables', extensions: ['exe'] }],
      });
      if (selected && typeof selected === 'string') {
        setValue(field, selected, { shouldValidate: true });
      }
    } catch (err) {
      console.error(t('games.form.validation.pick_file_error'), err);
    }
  };

  return (
    <dialog ref={dialogRef} className="modal bg-overlay-mask backdrop-blur-sm" onClose={onClose}>
      <div className="bg-base-100 rounded-xl shadow-2xl w-full max-w-lg overflow-hidden border border-base-300">
        <div className="flex items-center justify-between p-4 border-b border-base-300 bg-base-200/50">
          <h3 className="font-bold text-lg">
            {initialData ? t('games.form.title_edit') : t('games.form.title_add')}
          </h3>
          <button onClick={onClose} className="btn btn-ghost btn-sm btn-square">
            <X size={20} />
          </button>
        </div>

        <form onSubmit={handleSubmit(onSubmit)} className="p-6 space-y-4">
          {/* Name & Type */}
          <div className="grid grid-cols-2 gap-4">
            <div className="form-control">
              <label className="label">
                <span className="label-text font-medium">{t('games.form.name_label')}</span>
              </label>
              <input
                type="text"
                className={`input input-bordered w-full ${errors.name ? 'input-error' : ''}`}
                placeholder={t('games.form.name_placeholder')}
                {...register('name')}
              />
              {errors.name && (
                <span className="text-error text-xs mt-1">{errors.name.message}</span>
              )}
            </div>

            <div className="form-control">
              <label className="label">
                <span className="label-text font-medium">{t('games.form.type_label')}</span>
              </label>
              <select
                className="select select-bordered w-full"
                {...register('game_type', { valueAsNumber: true })}
              >
                <option value={GameType.GIMI}>{t('games.types.gimi')}</option>
                <option value={GameType.SRMI}>{t('games.types.srmi')}</option>
                <option value={GameType.ZZMI}>{t('games.types.zzmi')}</option>
                <option value={GameType.WWMI}>{t('games.types.wwmi')}</option>
                <option value={GameType.EFMI}>{t('games.types.efmi')}</option>
              </select>
            </div>
          </div>

          {/* Paths */}
          <div className="form-control">
            <label className="label">
              <span className="label-text font-medium">{t('games.form.path_label')}</span>
            </label>
            <div className="join w-full">
              <input
                type="text"
                className={`input input-bordered join-item w-full ${errors.mod_path ? 'input-error' : ''}`}
                placeholder={t('games.form.path_placeholder')}
                {...modPathField}
              />
              <button type="button" onClick={pickFolder} className="btn btn-primary join-item">
                <FolderOpen size={18} />
              </button>
            </div>
            {errors.mod_path && (
              <span className="text-error text-xs mt-1">{errors.mod_path.message}</span>
            )}
            <div className="text-xs text-base-content/50 mt-1 ml-1">
              {t('games.form.path_help')}
            </div>
          </div>

          <div className="form-control">
            <label className="label">
              <span className="label-text font-medium">{t('games.form.exe_label')}</span>
            </label>
            <div className="join w-full">
              <input
                type="text"
                className={`input input-bordered join-item w-full ${errors.game_exe ? 'input-error' : ''}`}
                placeholder={t('games.form.exe_placeholder')}
                {...register('game_exe')}
              />
              <button
                type="button"
                onClick={() => pickExe('game_exe')}
                className="btn btn-neutral join-item"
              >
                {t('games.form.browse')}
              </button>
            </div>
            {errors.game_exe && (
              <span className="text-error text-xs mt-1">{errors.game_exe.message}</span>
            )}
          </div>

          {/* Advanced Info */}
          <div className="collapse collapse-arrow bg-base-200/30 border border-base-200 rounded-lg">
            <input type="checkbox" />
            <div className="collapse-title font-medium text-sm">
              {t('games.form.advanced_title')}
            </div>
            <div className="collapse-content space-y-3 pt-2">
              <div className="form-control">
                <label className="label py-1">
                  <span className="label-text text-sm">{t('games.form.loader_label')}</span>
                </label>
                <div className="join w-full">
                  <input
                    type="text"
                    className="input input-bordered input-sm join-item w-full"
                    placeholder={t('games.form.loader_placeholder')}
                    {...register('loader_exe')}
                  />
                  <button
                    type="button"
                    onClick={() => pickExe('loader_exe')}
                    className="btn btn-neutral btn-sm join-item"
                  >
                    {t('games.form.browse')}
                  </button>
                </div>
              </div>

              <div className="form-control">
                <label className="label py-1">
                  <span className="label-text text-sm">{t('games.form.args_label')}</span>
                </label>
                <input
                  type="text"
                  className="input input-bordered input-sm w-full"
                  placeholder={t('games.form.args_placeholder')}
                  {...register('launch_args')}
                />
              </div>
            </div>
          </div>

          <div className="modal-action">
            <button type="button" className="btn" onClick={onClose} disabled={isSubmitting}>
              {t('page.back')}
            </button>
            <button type="submit" className="btn btn-primary" disabled={isSubmitting || !isValid}>
              {initialData ? t('games.form.save') : t('games.form.add_btn')}
            </button>
          </div>
        </form>
      </div>
    </dialog>
  );
}
