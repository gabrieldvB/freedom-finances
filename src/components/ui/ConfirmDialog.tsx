import { AlertTriangle, HelpCircle } from "lucide-react";
import { Button, type ButtonVariant } from "./Button";
import { Modal } from "./Modal";

interface ConfirmDialogProps {
  title: string;
  description: string;
  confirmLabel?: string;
  variant?: Extract<ButtonVariant, "danger" | "warning" | "primary" | "success">;
  busy?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

export function ConfirmDialog({ title, description, confirmLabel = "Confirmar", variant = "danger", busy = false, onConfirm, onCancel }: ConfirmDialogProps) {
  const dangerous = variant === "danger" || variant === "warning";
  return <Modal title={title} description={description} icon={dangerous ? <AlertTriangle size={20} /> : <HelpCircle size={20} />} onClose={() => { if (!busy) onCancel(); }}>
    <div className="confirm-actions">
      <Button variant="ghost" onClick={onCancel} disabled={busy}>Cancelar</Button>
      <Button variant={variant} onClick={onConfirm} loading={busy}>{confirmLabel}</Button>
    </div>
  </Modal>;
}
