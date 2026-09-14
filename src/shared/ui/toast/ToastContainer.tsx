import { X, CheckCircle, AlertCircle, Info, AlertTriangle } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { createPortal } from 'react-dom';
import { useToastStore, type Toast } from './store';

const icons = {
  success: <CheckCircle size={18} />,
  error: <AlertCircle size={18} />,
  info: <Info size={18} />,
  warning: <AlertTriangle size={18} />,
};

const colors = {
  success: 'alert-success text-success-content',
  error: 'alert-error text-error-content',
  info: 'alert-info text-info-content',
  warning: 'alert-warning text-warning-content',
};

export function ToastContainer() {
  const { toasts, removeToast } = useToastStore();

  if (toasts.length === 0) return null;

  return createPortal(
    <div className="toast toast-end toast-bottom z-50 flex flex-col gap-2 p-4">
      {toasts.map((t) => (
        <ToastItem key={t.id} toast={t} onDismiss={() => removeToast(t.id)} />
      ))}
    </div>,
    document.body,
  );
}

function ToastItem({ toast, onDismiss }: { toast: Toast; onDismiss: () => void }) {
  const [isExiting, setIsExiting] = useState(false);
  const beginDismiss = useCallback(() => setIsExiting(true), []);

  useEffect(() => {
    if (isExiting) {
      const timer = setTimeout(onDismiss, 160);
      return () => clearTimeout(timer);
    }
  }, [isExiting, onDismiss]);

  useEffect(() => {
    if (!isExiting && toast.duration && toast.duration > 0 && toast.duration !== Infinity) {
      const timer = setTimeout(beginDismiss, toast.duration);
      return () => clearTimeout(timer);
    }
  }, [beginDismiss, isExiting, toast.duration]);

  return (
    <div
      className={`alert ${colors[toast.type]} min-w-75 shadow-lg ${
        isExiting ? 'workspace-toast-exit' : 'workspace-toast-enter'
      } flex justify-between`}
      role={toast.type === 'error' ? 'alert' : 'status'}
    >
      <div className="flex items-center gap-2">
        {icons[toast.type]}
        <span className="text-sm font-medium">{toast.message}</span>
      </div>
      <div className="flex items-center gap-1">
        {toast.action && (
          <button
            onClick={() => {
              toast.action!.onClick();
              beginDismiss();
            }}
            className="btn btn-ghost btn-xs font-bold uppercase tracking-wide opacity-90 hover:opacity-100"
          >
            {toast.action.label}
          </button>
        )}
        <button
          onClick={beginDismiss}
          className="btn btn-ghost btn-xs btn-circle opacity-80 hover:opacity-100"
        >
          <X size={14} />
        </button>
      </div>
    </div>
  );
}
