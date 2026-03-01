import { create } from 'zustand';

export type ToastType = 'success' | 'error' | 'info' | 'warning';

/** Optional action button shown inside the toast (e.g. "Undo"). */
export interface ToastAction {
  label: string;
  onClick: () => void;
}

export interface Toast {
  id: string;
  type: ToastType;
  message: string;
  duration?: number;
  action?: ToastAction;
}

interface ToastState {
  toasts: Toast[];
  addToast: (type: ToastType, message: string, duration?: number) => string;
  addToastWithAction: (
    type: ToastType,
    message: string,
    action: ToastAction,
    duration?: number,
  ) => string;
  removeToast: (id: string) => void;
}

export const useToastStore = create<ToastState>((set) => ({
  toasts: [],
  addToast: (type, message, duration = 3000) => {
    const id = Math.random().toString(36).substring(2, 9);
    set((state) => {
      if (state.toasts.length >= 5) {
        console.warn('Toast cap reached. Dropped toast:', type, message);
        return state;
      }
      return { toasts: [...state.toasts, { id, type, message, duration }] };
    });
    return id;
  },
  addToastWithAction: (type, message, action, duration = 5000) => {
    const id = Math.random().toString(36).substring(2, 9);
    set((state) => {
      if (state.toasts.length >= 5) {
        console.warn('Toast cap reached. Dropped toast:', type, message);
        return state;
      }
      return { toasts: [...state.toasts, { id, type, message, duration, action }] };
    });
    return id;
  },
  removeToast: (id) => set((state) => ({ toasts: state.toasts.filter((t) => t.id !== id) })),
}));

// Helper utility for simple usage
export const toast = {
  success: (msg: string, duration?: number) =>
    useToastStore.getState().addToast('success', msg, duration),
  error: (msg: string, duration?: number) =>
    useToastStore.getState().addToast('error', msg, duration),
  info: (msg: string, duration?: number) =>
    useToastStore.getState().addToast('info', msg, duration),
  warning: (msg: string, duration?: number) =>
    useToastStore.getState().addToast('warning', msg, duration),
  /** Show a toast with an action button (e.g. undo). Default 5s duration. */
  withAction: (type: ToastType, msg: string, action: ToastAction, duration?: number) =>
    useToastStore.getState().addToastWithAction(type, msg, action, duration),
};
