import { Clock, Gamepad2, Keyboard, PlayCircle } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { commands } from '../../../lib/bindings';
import type { GameConfig } from '../../../types/game';
import type { DashboardPayload } from '../../../types/dashboard';
import type { ActiveKeyBinding } from '../../../types/settings';
import { formatRelativeDate } from '../dashboardViewUtils';

interface DashboardActivityProps {
  activeGame: GameConfig | null;
  keybindings: ActiveKeyBinding[];
  keybindingsLoading: boolean;
  recentMods: DashboardPayload['recent_mods'];
}

export function DashboardActivity({
  activeGame,
  keybindings,
  keybindingsLoading,
  recentMods,
}: DashboardActivityProps) {
  return (
    <>
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <RecentModsCard recentMods={recentMods} />
        <QuickPlayCard activeGame={activeGame} />
      </div>
      <ActiveKeybindingsCard keybindings={keybindings} isLoading={keybindingsLoading} />
    </>
  );
}

function RecentModsCard({ recentMods }: { recentMods: DashboardPayload['recent_mods'] }) {
  const { t } = useTranslation(['dashboard', 'common']);

  return (
    <div className="card bg-base-200/50 border border-base-300 lg:col-span-2">
      <div className="card-body">
        <h2 className="card-title text-sm font-semibold text-base-content/70">
          <Clock size={16} className="mr-1" />
          {t('activity.recent_title')}
        </h2>
        {recentMods.length > 0 ? (
          <ul className="space-y-2">
            {recentMods.map((mod) => (
              <li
                key={mod.id}
                className="flex items-center justify-between py-2 px-3 rounded-lg hover:bg-base-300/50 transition-colors"
              >
                <div className="min-w-0">
                  <p className="text-sm font-medium truncate">{mod.name}</p>
                  <p className="text-xs text-base-content/50">
                    {mod.category
                      ? t('activity.category', { category: mod.category })
                      : t('activity.uncategorized')}
                  </p>
                </div>
                <span className="text-xs text-base-content/40 whitespace-nowrap ml-3">
                  {formatRelativeDate(mod.modified_at, t)}
                </span>
              </li>
            ))}
          </ul>
        ) : (
          <p className="text-sm text-base-content/40 py-4 text-center">{t('activity.no_mods')}</p>
        )}
      </div>
    </div>
  );
}

function QuickPlayCard({ activeGame }: { activeGame: GameConfig | null }) {
  const { t } = useTranslation(['dashboard']);

  return (
    <div className="card bg-base-200/50 border border-base-300">
      <div className="card-body items-center text-center">
        <h2 className="card-title text-sm font-semibold text-base-content/70">
          {t('actions.quick_play')}
        </h2>
        {activeGame ? (
          <>
            <div className="my-3">
              <div className="w-16 h-16 rounded-full bg-primary/10 flex items-center justify-center">
                <Gamepad2 size={28} className="text-primary" />
              </div>
            </div>
            <p className="font-semibold text-base">{activeGame.name}</p>
            <p className="text-xs text-base-content/50 mb-3">{t('activity.last_selected')}</p>
            <button
              onClick={() => {
                commands.launchGame({ gameId: activeGame.id }).catch(console.error);
              }}
              className="btn btn-primary btn-sm gap-2 w-full"
            >
              <PlayCircle size={16} />
              {t('activity.launch')}
            </button>
          </>
        ) : (
          <p className="text-sm text-base-content/40 py-4">{t('activity.no_game')}</p>
        )}
      </div>
    </div>
  );
}

function ActiveKeybindingsCard({
  keybindings,
  isLoading,
}: {
  keybindings: ActiveKeyBinding[];
  isLoading: boolean;
}) {
  const { t } = useTranslation(['dashboard']);

  return (
    <div className="card bg-base-200/50 border border-base-300">
      <div className="card-body">
        <h2 className="card-title text-sm font-semibold text-base-content/70">
          <Keyboard size={16} className="mr-1" />
          {t('keys.title')}
          {keybindings.length > 0 && (
            <span className="badge badge-sm badge-ghost ml-1">{keybindings.length}</span>
          )}
        </h2>
        {isLoading ? (
          <div className="flex justify-center py-4">
            <span className="loading loading-dots loading-sm" />
          </div>
        ) : keybindings.length > 0 ? (
          <div className="overflow-x-auto max-h-64">
            <table className="table table-xs table-zebra">
              <thead className="sticky top-0 bg-base-200">
                <tr>
                  <th>{t('keys.table_mod')}</th>
                  <th>{t('keys.table_section')}</th>
                  <th>{t('keys.table_key')}</th>
                  <th>{t('keys.table_back')}</th>
                </tr>
              </thead>
              <tbody>
                {keybindings.map((keybinding, index) => (
                  <tr key={`${keybinding.mod_name}-${keybinding.section_name}-${index}`}>
                    <td
                      className="truncate max-w-40"
                      title={String(keybinding.mod_name ?? '') || undefined}
                    >
                      {String(keybinding.mod_name ?? '')}
                    </td>
                    <td className="text-base-content/60">{keybinding.section_name}</td>
                    <td>{keybinding.key && <kbd className="kbd kbd-xs">{keybinding.key}</kbd>}</td>
                    <td>
                      {keybinding.back && <kbd className="kbd kbd-xs">{keybinding.back}</kbd>}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <p className="text-sm text-base-content/40 py-4 text-center">{t('keys.no_bindings')}</p>
        )}
      </div>
    </div>
  );
}
