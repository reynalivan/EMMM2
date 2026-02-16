import { useEffect } from 'react';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import * as z from 'zod';
import { open } from '@tauri-apps/plugin-dialog';
import { X, FolderOpen } from 'lucide-react';
import type { GameConfig } from '../../../hooks/useSettings';

const gameSchema = z.object({
  id: z.string().optional(),
  name: z.string().min(1, 'Name is required'),
  game_type: z.enum(['Genshin', 'StarRail', 'ZZZ', 'Wuthering']),
  mod_path: z
    .string()
    .min(1, 'Mod path is required')
    .refine((value) => !/[?*<>|]/.test(value), 'Path contains invalid characters'),
  game_exe: z.string().min(1, 'Game executable is required'),
  loader_exe: z.string().nullable().optional(), // Can be empty
  launch_args: z.string().nullable().optional(),
});

type GameFormData = z.infer<typeof gameSchema>;

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
  const {
    register,
    handleSubmit,
    setValue,
    reset,
    formState: { errors, isSubmitting, isValid },
  } = useForm<GameFormData>({
    resolver: zodResolver(gameSchema),
    mode: 'onChange',
    defaultValues: {
      game_type: 'Genshin',
      loader_exe: '',
      launch_args: '',
    },
  });

  const modPathField = register('mod_path', {
    validate: (value) => {
      const current = value.trim().replace(/\\/g, '/').toLowerCase();
      const isDuplicate = existingModPaths.some((path) => {
        const next = path.trim().replace(/\\/g, '/').toLowerCase();
        return next === current;
      });
      return isDuplicate ? 'Already registered' : true;
    },
  });

  useEffect(() => {
    if (isOpen) {
      if (initialData) {
        reset({
          id: initialData.id,
          name: initialData.name,
          game_type: initialData.game_type as 'Genshin' | 'StarRail' | 'ZZZ' | 'Wuthering',
          mod_path: initialData.mod_path,
          game_exe: initialData.game_exe,
          loader_exe: initialData.loader_exe || '',
          launch_args: initialData.launch_args || '',
        });
      } else {
        reset({
          id: undefined,
          name: '',
          game_type: 'Genshin',
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
        title: 'Select Mod Folder',
      });
      if (selected && typeof selected === 'string') {
        setValue('mod_path', selected, { shouldValidate: true });
      }
    } catch (err) {
      console.error('Failed to pick folder', err);
    }
  };

  const pickExe = async (field: 'game_exe' | 'loader_exe') => {
    try {
      const selected = await open({
        directory: false,
        multiple: false,
        title: `Select ${field === 'game_exe' ? 'Game' : 'Loader'} Executable`,
        filters: [{ name: 'Executables', extensions: ['exe'] }],
      });
      if (selected && typeof selected === 'string') {
        setValue(field, selected, { shouldValidate: true });
      }
    } catch (err) {
      console.error('Failed to pick file', err);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm p-4">
      <div className="bg-base-100 rounded-xl shadow-2xl w-full max-w-lg overflow-hidden border border-base-300">
        <div className="flex items-center justify-between p-4 border-b border-base-300 bg-base-200/50">
          <h3 className="font-bold text-lg">{initialData ? 'Edit Game' : 'Add New Game'}</h3>
          <button onClick={onClose} className="btn btn-ghost btn-sm btn-square">
            <X size={20} />
          </button>
        </div>

        <form onSubmit={handleSubmit(onSubmit)} className="p-6 space-y-4">
          {/* Name & Type */}
          <div className="grid grid-cols-2 gap-4">
            <div className="form-control">
              <label className="label">
                <span className="label-text font-medium">Display Name</span>
              </label>
              <input
                type="text"
                className={`input input-bordered w-full ${errors.name ? 'input-error' : ''}`}
                placeholder="e.g. Genshin Impact"
                {...register('name')}
              />
              {errors.name && (
                <span className="text-error text-xs mt-1">{errors.name.message}</span>
              )}
            </div>

            <div className="form-control">
              <label className="label">
                <span className="label-text font-medium">Game Type</span>
              </label>
              <select className="select select-bordered w-full" {...register('game_type')}>
                <option value="Genshin">Genshin Impact</option>
                <option value="StarRail">Star Rail</option>
                <option value="ZZZ">ZZZ</option>
                <option value="Wuthering">Wuthering Waves</option>
              </select>
            </div>
          </div>

          {/* Paths */}
          <div className="form-control">
            <label className="label">
              <span className="label-text font-medium">Mods Folder Path</span>
            </label>
            <div className="join w-full">
              <input
                type="text"
                className={`input input-bordered join-item w-full ${errors.mod_path ? 'input-error' : ''}`}
                placeholder="C:/Games/Genshin Impact/Mods"
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
              Select the folder where you store your Mods.
            </div>
          </div>

          <div className="form-control">
            <label className="label">
              <span className="label-text font-medium">Game Executable (.exe)</span>
            </label>
            <div className="join w-full">
              <input
                type="text"
                className={`input input-bordered join-item w-full ${errors.game_exe ? 'input-error' : ''}`}
                placeholder="C:/Games/Genshin Impact/GenshinImpact.exe"
                {...register('game_exe')}
              />
              <button
                type="button"
                onClick={() => pickExe('game_exe')}
                className="btn btn-neutral join-item"
              >
                Browse
              </button>
            </div>
            {errors.game_exe && (
              <span className="text-error text-xs mt-1">{errors.game_exe.message}</span>
            )}
          </div>

          {/* Advanced Info */}
          <div className="collapse collapse-arrow bg-base-200/30 border border-base-200 rounded-lg">
            <input type="checkbox" />
            <div className="collapse-title font-medium text-sm">Advanced Options</div>
            <div className="collapse-content space-y-3 pt-2">
              <div className="form-control">
                <label className="label py-1">
                  <span className="label-text text-sm">3DMigoto Loader (Optional)</span>
                </label>
                <div className="join w-full">
                  <input
                    type="text"
                    className="input input-bordered input-sm join-item w-full"
                    placeholder="3DMigoto Loader.exe"
                    {...register('loader_exe')}
                  />
                  <button
                    type="button"
                    onClick={() => pickExe('loader_exe')}
                    className="btn btn-neutral btn-sm join-item"
                  >
                    Browse
                  </button>
                </div>
              </div>

              <div className="form-control">
                <label className="label py-1">
                  <span className="label-text text-sm">Launch Arguments</span>
                </label>
                <input
                  type="text"
                  className="input input-bordered input-sm w-full"
                  placeholder="-popupwindow"
                  {...register('launch_args')}
                />
              </div>
            </div>
          </div>

          <div className="modal-action">
            <button type="button" className="btn" onClick={onClose} disabled={isSubmitting}>
              Cancel
            </button>
            <button type="submit" className="btn btn-primary" disabled={isSubmitting || !isValid}>
              {initialData ? 'Save Changes' : 'Add Game'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
