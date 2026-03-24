import React from 'react';
import { useTranslation } from 'react-i18next';
import { useIgnoredPairs, useRemoveIgnoredPair } from '../hooks/useDedup';
import { X, RefreshCw } from 'lucide-react';

interface IgnoredPairsModalProps {
  gameId: string;
  onClose: () => void;
}

export const IgnoredPairsModal: React.FC<IgnoredPairsModalProps> = ({ gameId, onClose }) => {
  const { t, i18n } = useTranslation(['scanner']);
  const { data: ignoredPairs, isLoading } = useIgnoredPairs(gameId);
  const { mutate: recoverPair, isPending: isRecovering } = useRemoveIgnoredPair();

  const formatDate = (dateStr: string) => {
    try {
      const date = new Date(dateStr);
      return new Intl.DateTimeFormat(
        i18n.language === 'id' ? 'id-ID' : i18n.language === 'zh' ? 'zh-CN' : 'en-US',
        {
          month: 'short',
          day: '2-digit',
          year: 'numeric',
          hour: '2-digit',
          minute: '2-digit',
          hour12: false,
        },
      ).format(date);
    } catch {
      return dateStr;
    }
  };

  return (
    <div className="modal modal-open">
      <div className="modal-box max-w-4xl bg-base-300 border border-base-content/10 shadow-2xl">
        <div className="flex items-center justify-between mb-6">
          <div>
            <h3 className="text-xl font-bold flex items-center gap-2">
              <span className="text-primary">{t('scanner:ignored_pairs.title_primary')}</span>{' '}
              {t('scanner:ignored_pairs.title', { primary: '' }).trim()}
            </h3>
            <p className="text-sm text-base-content/60 mt-1">
              {t('scanner:ignored_pairs.description')}
            </p>
          </div>
          <button className="btn btn-sm btn-ghost btn-circle" onClick={onClose}>
            <X className="w-5 h-5" />
          </button>
        </div>

        <div className="overflow-x-auto min-h-75 max-h-[60vh]">
          {isLoading ? (
            <div className="flex flex-col items-center justify-center py-20 gap-4">
              <span className="loading loading-spinner loading-lg text-primary"></span>
              <p className="text-sm text-base-content/60 animate-pulse">
                {t('scanner:ignored_pairs.loading')}
              </p>
            </div>
          ) : !ignoredPairs || ignoredPairs.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-20 text-center bg-base-200/50 rounded-xl border border-dashed border-base-content/20">
              <p className="text-lg font-medium opacity-40">
                {t('scanner:ignored_pairs.no_results')}
              </p>
              <p className="text-sm opacity-30 mt-1">
                {t('scanner:ignored_pairs.no_results_desc')}
              </p>
            </div>
          ) : (
            <table className="table table-sm table-zebra w-full border-separate border-spacing-y-1">
              <thead>
                <tr className="bg-base-200/50">
                  <th className="rounded-l-lg py-3">{t('scanner:ignored_pairs.table.mod_a')}</th>
                  <th className="py-3">{t('scanner:ignored_pairs.table.mod_b')}</th>
                  <th className="py-3">{t('scanner:ignored_pairs.table.reason')}</th>
                  <th className="py-3">{t('scanner:ignored_pairs.table.ignored_at')}</th>
                  <th className="rounded-r-lg py-3 text-right">
                    {t('scanner:ignored_pairs.table.action')}
                  </th>
                </tr>
              </thead>
              <tbody>
                {ignoredPairs.map((pair) => (
                  <tr key={pair.id} className="hover:bg-primary/5 transition-colors group">
                    <td className="font-medium max-w-50 truncate">{pair.folder_a_name}</td>
                    <td className="font-medium max-w-50 truncate">{pair.folder_b_name}</td>
                    <td>
                      <span className="badge badge-ghost badge-sm border-base-content/10 opacity-70">
                        {pair.reason}
                      </span>
                    </td>
                    <td className="text-xs opacity-60">{formatDate(pair.ignored_at)}</td>
                    <td className="text-right">
                      <button
                        className="btn btn-xs btn-outline btn-primary opacity-0 group-hover:opacity-100 transition-opacity"
                        onClick={() => recoverPair(pair.id)}
                        disabled={isRecovering}
                      >
                        {isRecovering ? (
                          <span className="loading loading-spinner loading-xs"></span>
                        ) : (
                          <>
                            <RefreshCw className="w-3 h-3 mr-1" />
                            {t('scanner:ignored_pairs.action_recover')}
                          </>
                        )}
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>

        <div className="modal-action mt-8 pt-4 border-t border-base-content/5">
          <button className="btn btn-primary btn-sm px-6" onClick={onClose}>
            {t('scanner:ignored_pairs.action_close')}
          </button>
        </div>
      </div>
      <div className="modal-backdrop bg-overlay-mask backdrop-blur-sm" onClick={onClose}></div>
    </div>
  );
};
