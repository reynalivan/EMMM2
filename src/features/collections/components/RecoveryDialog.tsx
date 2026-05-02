import { useState } from 'react';
import { commands } from '../../../lib/bindings';
import { AlertTriangle, Loader2, RotateCcw } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { PipelineTask, RecoveryAction } from '../../../types/task';
import { toast } from '../../../stores/useToastStore';
import { formatAppError } from '../../../lib/appError';

interface RecoveryDialogProps {
  tasks: PipelineTask[];
  onResolved: (remainingTasks: PipelineTask[]) => void;
}

export function RecoveryDialog({ tasks, onResolved }: RecoveryDialogProps) {
  const { t } = useTranslation('collections');
  const [pendingAction, setPendingAction] = useState<RecoveryAction | null>(null);
  const primaryTask = tasks[0] ?? null;

  const resolveTasks = async (): Promise<void> => {
    const remainingTasks = await commands.appStartupCheck();
    onResolved(remainingTasks);
  };

  const handleResolve = async (action: RecoveryAction) => {
    if (!primaryTask) {
      return;
    }

    setPendingAction(action);
    try {
      await commands.resolveRecoveryTask({
        taskId: primaryTask.id,
        action,
      });
      const successKey =
        action === 'RETRY'
          ? 'recovery.toast.resumed'
          : action === 'ROLLBACK'
            ? 'recovery.toast.rolled_back'
            : 'recovery.toast.ignored';
      toast.success(t(successKey), 5000);
      await resolveTasks();
    } catch (e) {
      const failureKey =
        action === 'RETRY'
          ? 'recovery.toast.resume_failed'
          : action === 'ROLLBACK'
            ? 'recovery.toast.rollback_failed'
            : 'recovery.toast.ignore_failed';
      toast.error(t(failureKey, { error: formatAppError(e) }), 5000);
    } finally {
      setPendingAction(null);
    }
  };

  if (tasks.length === 0) return null;

  return (
    <div className="fixed inset-0 z-9999 bg-base-300/90 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="bg-base-100 rounded-xl shadow-2xl max-w-lg w-full overflow-hidden border border-error/20">
        <div className="bg-error/10 p-6 flex items-start gap-4 border-b border-error/10">
          <div className="bg-error/20 p-3 rounded-full text-error shrink-0">
            <AlertTriangle size={32} />
          </div>
          <div>
            <h2 className="text-xl font-bold text-error mb-1">{t('recovery.title')}</h2>
            <p className="text-base-content/70 text-sm">{t('recovery.desc')}</p>
          </div>
        </div>

        <div className="p-6">
          <h3 className="font-semibold mb-3">{t('recovery.pending', { count: tasks.length })}</h3>
          <div className="bg-base-200 rounded-lg p-3 max-h-40 overflow-y-auto custom-scrollbar mb-6 border border-base-300 space-y-2">
            {tasks.map((task) => (
              <div
                key={task.id}
                className="flex flex-col text-sm bg-base-100 p-2 rounded border border-base-300"
              >
                <div className="flex justify-between">
                  <span className="font-mono text-xs opacity-70">
                    {t('recovery.labels.id')} {task.id.slice(0, 8)}...
                  </span>
                  <span className="badge badge-warning badge-sm">{task.status}</span>
                </div>
                <div className="font-medium mt-1">
                  {t('recovery.labels.type')} {task.task_type}
                </div>
                {task.target_id && (
                  <div className="text-xs opacity-60 mt-1">
                    {t('recovery.labels.target')} {task.target_id}
                  </div>
                )}
              </div>
            ))}
          </div>

          <p className="text-sm mb-6 p-3 bg-warning/10 text-warning-content rounded-lg border border-warning/20">
            <span className="font-bold flex items-center gap-2 mb-1">
              <RotateCcw size={16} /> {t('recovery.important')}
            </span>
            {t('recovery.warning')}
          </p>

          <div className="flex justify-end gap-2">
            <button
              className="btn btn-ghost w-full sm:w-auto"
              onClick={() => void handleResolve('IGNORE')}
              disabled={pendingAction !== null}
            >
              {pendingAction === 'IGNORE' ? (
                <>
                  <Loader2 className="animate-spin" size={18} />
                  {t('recovery.actions.ignoring')}
                </>
              ) : (
                t('recovery.actions.ignore')
              )}
            </button>
            <button
              className="btn btn-warning w-full sm:w-auto"
              onClick={() => void handleResolve('ROLLBACK')}
              disabled={pendingAction !== null}
            >
              {pendingAction === 'ROLLBACK' ? (
                <>
                  <Loader2 className="animate-spin" size={18} />
                  {t('recovery.actions.rolling_back')}
                </>
              ) : (
                t('recovery.actions.rollback')
              )}
            </button>
            <button
              className="btn btn-primary w-full sm:w-auto"
              onClick={() => void handleResolve('RETRY')}
              disabled={pendingAction !== null}
            >
              {pendingAction === 'RETRY' ? (
                <>
                  <Loader2 className="animate-spin" size={18} />
                  {t('recovery.actions.resuming')}
                </>
              ) : (
                t('recovery.actions.resume')
              )}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
