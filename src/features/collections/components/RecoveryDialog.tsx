import { useState } from 'react';
import { commands } from '../../../lib/bindings';
import { AlertTriangle, Loader2, RotateCcw } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { PipelineTask } from '../../../types/task';
import { toast } from '../../../stores/useToastStore';

interface RecoveryDialogProps {
  tasks: PipelineTask[];
  onResolved: () => void;
}

export function RecoveryDialog({ tasks, onResolved }: RecoveryDialogProps) {
  const { t } = useTranslation('collections');
  const [isClearing, setIsClearing] = useState(false);
  const [isResuming, setIsResuming] = useState(false);

  const handleClear = async () => {
    setIsClearing(true);
    try {
      await commands.clearPendingTasks();
      toast.success(t('recovery.toast.cleared'), 5000);
      onResolved();
    } catch (e) {
      console.error('Failed to clear pending tasks:', e);
      toast.error(String(e), 5000);
      setIsClearing(false);
    }
  };

  const handleResume = async (task: PipelineTask) => {
    setIsResuming(true);
    try {
      if (task.task_type === 'apply_collection') {
        if (!task.target_id) throw new Error('Missing target collection ID');
        await commands.applyCollection({
          gameId: task.game_id,
          collectionId: task.target_id,
          ignoreMissing: false,
        });
      } else if (task.task_type === 'switch_corridor') {
        if (!task.target_id) throw new Error('Missing target corridor state');
        await commands.switchCorridor({
          gameId: task.game_id,
          targetSafe: task.target_id === 'true',
        });
      } else {
        throw new Error(`Unknown task type: ${task.task_type}`);
      }

      // If we succeed, the task is effectively handled. Clear pending tracking.
      await commands.clearPendingTasks();
      toast.success(t('recovery.toast.resumed'), 5000);
      onResolved();
    } catch (e) {
      console.error('Failed to resume task:', e);
      toast.error(t('recovery.toast.resume_failed', { error: String(e) }), 5000);
      setIsResuming(false);
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
                  <span className="font-mono text-xs opacity-70">ID: {task.id.slice(0, 8)}...</span>
                  <span className="badge badge-warning badge-sm">{task.status}</span>
                </div>
                <div className="font-medium mt-1">Type: {task.task_type}</div>
                {task.target_id && (
                  <div className="text-xs opacity-60 mt-1">Target: {task.target_id}</div>
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
              onClick={handleClear}
              disabled={isClearing || isResuming}
            >
              {isClearing ? (
                <>
                  <Loader2 className="animate-spin" size={18} />
                  {t('recovery.actions.clearing')}
                </>
              ) : (
                t('recovery.actions.clear')
              )}
            </button>
            <button
              className="btn btn-primary w-full sm:w-auto"
              onClick={() => handleResume(tasks[0])}
              disabled={isClearing || isResuming}
            >
              {isResuming ? (
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
