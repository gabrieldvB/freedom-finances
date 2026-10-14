import type { PropsWithChildren, ReactNode } from "react";
import { X } from "lucide-react";

export function PageHeader({ title, subtitle, actions }: { title: string; subtitle?: string; actions?: ReactNode }) {
  return (
    <header className="page-header">
      <div>
        <h1>{title}</h1>
        {subtitle && <p>{subtitle}</p>}
      </div>
      {actions && <div className="header-actions">{actions}</div>}
    </header>
  );
}

export function EmptyState({ children }: PropsWithChildren) {
  return <div className="empty-state">{children}</div>;
}

export function Modal({ title, onClose, children, wide = false }: PropsWithChildren<{ title: string; onClose: () => void; wide?: boolean }>) {
  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={onClose}>
      <section className={`modal ${wide ? "modal-wide" : ""}`} role="dialog" aria-modal="true" onMouseDown={(event) => event.stopPropagation()}>
        <header className="modal-header">
          <h2>{title}</h2>
          <button className="icon-button" onClick={onClose} aria-label="Fechar"><X size={20} /></button>
        </header>
        {children}
      </section>
    </div>
  );
}

export function StatusBadge({ status }: { status: string }) {
  const labels: Record<string, string> = {
    open: "Em aberto",
    paid: "Pago",
    cancelled: "Cancelado",
    closed: "Fechada",
    income: "Receita",
    expense: "Despesa",
    transfer: "Transferência",
  };
  return <span className={`badge badge-${status}`}>{labels[status] ?? status}</span>;
}

export function Spinner() {
  return <div className="loading">Carregando…</div>;
}

export function Notice({ type, children }: PropsWithChildren<{ type: "success" | "error" }>) {
  return <div className={`notice notice-${type}`}>{children}</div>;
}
