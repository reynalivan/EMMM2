import { X, CheckCircle, AlertCircle, Info, AlertTriangle } from 'lucide-react';
import { useEffect } from 'react';
import { createPortal } from 'react-dom';
import { useToastStore, type Toast } from '../../stores/useToastStore';

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
  useEffect(() => {
    if (toast.duration && toast.duration > 0) {
      const timer = setTimeout(onDismiss, toast.duration);
      return () => clearTimeout(timer);
    }
  }, [toast, onDismiss]);

  return (
    <div
      className={`alert ${colors[toast.type]} shadow-lg min-w-[300px] flex justify-between animate-in slide-in-from-right-5 fade-in duration-300`}
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
              onDismiss();
            }}
            className="btn btn-ghost btn-xs font-bold uppercase tracking-wide opacity-90 hover:opacity-100"
          >
            {toast.action.label}
          </button>
        )}
        <button
          onClick={onDismiss}
          className="btn btn-ghost btn-xs btn-circle opacity-80 hover:opacity-100"
        >
          <X size={14} />
        </button>
      </div>
    </div>
  );
}
