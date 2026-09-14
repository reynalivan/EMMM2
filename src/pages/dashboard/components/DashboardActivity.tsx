import { Clock, Keyboard } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { GameConfig } from '@/entities/game';
import type { DashboardPayload } from '../model/dashboard';
import type { ActiveKeyBinding } from '@/entities/settings';
import { ModThumbnail } from '@/entities/mod';
import { formatRelativeDate } from '../../../shared/lib/utils/formatters';
import VirtualList from '@/shared/ui/components/ui/VirtualList';

const KEYBINDING_VIRTUALIZATION_THRESHOLD = 80;

function keybindingKey(keybinding: ActiveKeyBinding): string {
  return `${keybinding.folder_path}:${keybinding.section_name}:${keybinding.key ?? ''}:${keybinding.back ?? ''}`;
}

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
      <RecentModsCard recentMods={recentMods} />
      <ActiveKeybindingsCard
        keybindings={keybindings}
        isLoading={keybindingsLoading}
        gameId={activeGame?.id ?? ''}
      />
    </>
  );
}

function RecentModsCard({ recentMods }: { recentMods: DashboardPayload['recent_mods'] }) {
  const { t } = useTranslation(['dashboard', 'common']);

  return (
    <div className="card bg-base-200/50 border border-base-300">
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
                <div className="flex min-w-0 items-center gap-3">
                  <ModThumbnail
                    gameId={mod.game_id}
                    folderPath={mod.folder_path}
                    sizeClassName="size-9"
                  />
                  <div className="min-w-0">
                    <p className="text-sm font-medium truncate">{mod.name}</p>
                    <p className="text-xs text-base-content/50">
                      {mod.object_name
                        ? t('activity.category', { category: mod.object_name })
                        : t('activity.uncategorized')}
                    </p>
                  </div>
                </div>
                <span className="text-xs text-base-content/40 whitespace-nowrap ml-3">
                  {formatRelativeDate(mod.indexed_at, t)}
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

function ActiveKeybindingsCard({
  keybindings,
  isLoading,
  gameId,
}: {
  keybindings: ActiveKeyBinding[];
  isLoading: boolean;
  gameId: string;
}) {
  const { t } = useTranslation(['dashboard']);
  const shouldVirtualize = keybindings.length > KEYBINDING_VIRTUALIZATION_THRESHOLD;

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
          <>
            <div className="space-y-2 sm:hidden" data-testid="active-keybindings-mobile-cards">
              {shouldVirtualize ? (
                <VirtualList
                  ariaLabel={t('keys.title')}
                  className="h-[min(55vh,34rem)]"
                  contentClassName="pr-1"
                  estimateSize={() => 176}
                  getItemKey={keybindingKey}
                  items={keybindings}
                  renderItem={(keybinding) => (
                    <ActiveKeybindingMobileCard gameId={gameId} keybinding={keybinding} />
                  )}
                />
              ) : (
                keybindings.map((keybinding) => (
                  <ActiveKeybindingMobileCard
                    key={keybindingKey(keybinding)}
                    gameId={gameId}
                    keybinding={keybinding}
                  />
                ))
              )}
            </div>
            <div className="hidden sm:block">
              {shouldVirtualize ? (
                <VirtualList
                  ariaLabel={t('keys.title')}
                  className="h-[min(55vh,34rem)] rounded-lg border border-base-300"
                  estimateSize={() => 48}
                  getItemKey={keybindingKey}
                  items={keybindings}
                  renderItem={(keybinding) => (
                    <ActiveKeybindingDesktopRow gameId={gameId} keybinding={keybinding} />
                  )}
                />
              ) : (
                <div className="hidden max-h-[min(55vh,34rem)] overflow-auto sm:block">
                  <table className="table table-xs table-zebra">
                    <thead className="sticky top-0 bg-base-200">
                      <tr>
                        <th>{t('keys.table_mod')}</th>
                        <th>{t('keys.table_section')}</th>
                        <th>{t('keys.table_key')}</th>
                        <th>{t('keys.table_back')}</th>
                        <th>{t('keys.table_control')}</th>
                        <th>{t('keys.table_values')}</th>
                      </tr>
                    </thead>
                    <tbody>
                      {keybindings.map((keybinding) => (
                        <tr key={keybindingKey(keybinding)}>
                          <td className="max-w-48">
                            <div className="flex min-w-0 items-center gap-2">
                              <ModThumbnail
                                gameId={gameId}
                                folderPath={keybinding.folder_path}
                                sizeClassName="size-7"
                              />
                              <span
                                className="truncate"
                                title={String(keybinding.mod_name ?? '') || undefined}
                              >
                                {String(keybinding.mod_name ?? '')}
                              </span>
                            </div>
                          </td>
                          <td className="text-base-content/60">{keybinding.section_name}</td>
                          <td>
                            {keybinding.key && <kbd className="kbd kbd-xs">{keybinding.key}</kbd>}
                          </td>
                          <td>
                            {keybinding.back && <kbd className="kbd kbd-xs">{keybinding.back}</kbd>}
                          </td>
                          <td>
                            <span className="badge badge-ghost badge-xs">
                              {t(`keys.control_kind.${keybinding.control_kind}`)}
                            </span>
                          </td>
                          <td
                            className="max-w-40 truncate text-base-content/60"
                            title={keybinding.value_summary ?? undefined}
                          >
                            {keybinding.value_summary ?? '-'}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
            </div>
          </>
        ) : (
          <p className="text-sm text-base-content/40 py-4 text-center">{t('keys.no_bindings')}</p>
        )}
      </div>
    </div>
  );
}

function ActiveKeybindingDesktopRow({
  gameId,
  keybinding,
}: {
  gameId: string;
  keybinding: ActiveKeyBinding;
}) {
  const { t } = useTranslation(['dashboard']);

  return (
    <article className="grid min-h-12 grid-cols-[minmax(12rem,1.4fr)_minmax(8rem,1fr)_4rem_4rem_7rem_minmax(8rem,1fr)] items-center gap-2 border-b border-base-300 px-3 py-1.5 text-xs last:border-b-0">
      <div className="flex min-w-0 items-center gap-2">
        <ModThumbnail gameId={gameId} folderPath={keybinding.folder_path} sizeClassName="size-7" />
        <span className="truncate" title={keybinding.mod_name}>
          {keybinding.mod_name}
        </span>
      </div>
      <span className="truncate text-base-content/60">{keybinding.section_name}</span>
      <span>{keybinding.key && <kbd className="kbd kbd-xs">{keybinding.key}</kbd>}</span>
      <span>{keybinding.back && <kbd className="kbd kbd-xs">{keybinding.back}</kbd>}</span>
      <span className="badge badge-ghost badge-xs w-fit">
        {t(`keys.control_kind.${keybinding.control_kind}`)}
      </span>
      <span className="truncate text-base-content/60" title={keybinding.value_summary ?? undefined}>
        {keybinding.value_summary ?? '-'}
      </span>
    </article>
  );
}

function ActiveKeybindingMobileCard({
  gameId,
  keybinding,
}: {
  gameId: string;
  keybinding: ActiveKeyBinding;
}) {
  const { t } = useTranslation(['dashboard']);

  return (
    <article className="rounded-lg border border-base-300 bg-base-100/40 p-3">
      <div className="flex min-w-0 items-center gap-3">
        <ModThumbnail gameId={gameId} folderPath={keybinding.folder_path} sizeClassName="size-9" />
        <div className="min-w-0">
          <p
            className="truncate text-sm font-medium"
            title={String(keybinding.mod_name ?? '') || undefined}
          >
            {String(keybinding.mod_name ?? '')}
          </p>
          <p className="truncate text-xs text-base-content/60">{keybinding.section_name}</p>
        </div>
      </div>
      <dl className="mt-3 grid grid-cols-2 gap-x-4 gap-y-2 text-xs">
        <div>
          <dt className="text-base-content/50">{t('keys.table_key')}</dt>
          <dd className="mt-0.5">
            {keybinding.key ? <kbd className="kbd kbd-xs">{keybinding.key}</kbd> : '-'}
          </dd>
        </div>
        <div>
          <dt className="text-base-content/50">{t('keys.table_back')}</dt>
          <dd className="mt-0.5">
            {keybinding.back ? <kbd className="kbd kbd-xs">{keybinding.back}</kbd> : '-'}
          </dd>
        </div>
        <div>
          <dt className="text-base-content/50">{t('keys.table_control')}</dt>
          <dd className="mt-0.5">
            <span className="badge badge-ghost badge-xs">
              {t(`keys.control_kind.${keybinding.control_kind}`)}
            </span>
          </dd>
        </div>
        <div className="min-w-0">
          <dt className="text-base-content/50">{t('keys.table_values')}</dt>
          <dd
            className="mt-0.5 truncate text-base-content/70"
            title={keybinding.value_summary ?? undefined}
          >
            {keybinding.value_summary ?? '-'}
          </dd>
        </div>
      </dl>
    </article>
  );
}
