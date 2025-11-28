import { useState, useCallback } from 'react';
import { ToastMessage, ToastType } from '../components/Toast';

export interface AddToastOptions {
  type: ToastType;
  title: string;
  message?: string;
  duration?: number;
  persistent?: boolean;
}

export interface ToastContextValue {
  toasts: ToastMessage[];
  addToast: (options: AddToastOptions) => string;
  removeToast: (id: string) => void;
  clearAllToasts: () => void;
  // Convenience methods
  success: (title: string, message?: string, duration?: number) => string;
  error: (title: string, message?: string, persistent?: boolean) => string;
  warning: (title: string, message?: string, duration?: number) => string;
  info: (title: string, message?: string, duration?: number) => string;
}

export const useToast = (): ToastContextValue => {
  const [toasts, setToasts] = useState<ToastMessage[]>([]);

  const generateId = (): string => {
    return `toast-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
  };

  const addToast = useCallback((options: AddToastOptions): string => {
    const id = generateId();
    const toast: ToastMessage = {
      id,
      type: options.type,
      title: options.title,
      message: options.message,
      duration: options.duration ?? 5000,
      persistent: options.persistent ?? false,
    };

    setToasts(prev => [...prev, toast]);
    return id;
  }, []);

  const removeToast = useCallback((id: string) => {
    setToasts(prev => prev.filter(toast => toast.id !== id));
  }, []);

  const clearAllToasts = useCallback(() => {
    setToasts([]);
  }, []);

  // Convenience methods for common toast types
  const success = useCallback((title: string, message?: string, duration?: number): string => {
    return addToast({ type: 'success', title, message, duration });
  }, [addToast]);

  const error = useCallback((title: string, message?: string, persistent?: boolean): string => {
    return addToast({ 
      type: 'error', 
      title, 
      message, 
      duration: persistent ? 0 : 7000, // Errors stay longer by default
      persistent 
    });
  }, [addToast]);

  const warning = useCallback((title: string, message?: string, duration?: number): string => {
    return addToast({ type: 'warning', title, message, duration: duration ?? 6000 });
  }, [addToast]);

  const info = useCallback((title: string, message?: string, duration?: number): string => {
    return addToast({ type: 'info', title, message, duration });
  }, [addToast]);

  return {
    toasts,
    addToast,
    removeToast,
    clearAllToasts,
    success,
    error,
    warning,
    info,
  };
};