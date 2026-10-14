import type { PropsWithChildren, ReactNode } from "react";
import { AlertCircle, CheckCircle2 } from "lucide-react";

export { EmptyState } from "./ui/EmptyState";
export { Modal } from "./ui/Modal";
export { StatusBadge } from "./ui/Badge";

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

export function Spinner() {
  return <div className="loading" role="status"><span className="loader" /><span>Carregando dados…</span></div>;
}

export function Notice({ type, children }: PropsWithChildren<{ type: "success" | "error" }>) {
  return <div className={`notice notice-${type}`}>{type === "success" ? <CheckCircle2 size={17} /> : <AlertCircle size={17} />}<span>{children}</span></div>;
}
