import React, { createContext, useContext, ReactNode } from 'react';
import { useToast, ToastContextValue } from '../hooks/useToast';
import ToastContainer from '../components/ToastContainer';

const ToastContext = createContext<ToastContextValue | undefined>(undefined);

interface ToastProviderProps {
  children: ReactNode;
}

export const ToastProvider: React.FC<ToastProviderProps> = ({ children }) => {
  const toastValue = useToast();

  return (
    <ToastContext.Provider value={toastValue}>
      {children}
      <ToastContainer toasts={toastValue.toasts} onRemoveToast={toastValue.removeToast} />
    </ToastContext.Provider>
  );
};

export const useToastContext = (): ToastContextValue => {
  const context = useContext(ToastContext);
  if (context === undefined) {
    throw new Error('useToastContext must be used within a ToastProvider');
  }
  return context;
};