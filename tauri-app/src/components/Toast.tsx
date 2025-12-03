import { useEffect, useState } from "react";
import "./Toast.css";

export type ToastType = "success" | "error" | "info" | "warning";

export interface ToastMessage {
  id: string;
  type: ToastType;
  message: string;
  duration?: number;
}

interface ToastProps {
  toasts: ToastMessage[];
  onDismiss: (id: string) => void;
}

const TOAST_ICONS: Record<ToastType, string> = {
  success: "✅",
  error: "❌",
  info: "ℹ️",
  warning: "⚠️",
};

export function Toast({ toasts, onDismiss }: ToastProps) {
  return (
    <div className="toast-container">
      {toasts.map((toast) => (
        <ToastItem
          key={toast.id}
          toast={toast}
          onDismiss={() => onDismiss(toast.id)}
        />
      ))}
    </div>
  );
}

function ToastItem({
  toast,
  onDismiss,
}: {
  toast: ToastMessage;
  onDismiss: () => void;
}) {
  const [isExiting, setIsExiting] = useState(false);

  useEffect(() => {
    const duration = toast.duration || 4000;
    const timer = setTimeout(() => {
      setIsExiting(true);
      setTimeout(onDismiss, 300); // Wait for exit animation
    }, duration);

    return () => clearTimeout(timer);
  }, [toast.duration, onDismiss]);

  return (
    <div
      className={`toast-item toast-${toast.type} ${isExiting ? "exiting" : ""}`}
      onClick={() => {
        setIsExiting(true);
        setTimeout(onDismiss, 300);
      }}
    >
      <div className="toast-icon">{TOAST_ICONS[toast.type]}</div>
      <div className="toast-message">{toast.message}</div>
      <button className="toast-close" onClick={onDismiss}>
        ×
      </button>
    </div>
  );
}
