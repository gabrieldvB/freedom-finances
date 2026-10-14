import type { PropsWithChildren, ReactNode } from "react";
import { X } from "lucide-react";

interface ModalProps {
  title: string;
  description?: string;
  icon?: ReactNode;
  onClose: () => void;
  wide?: boolean;
}

export function Modal({ title, description, icon, onClose, children, wide = false }: PropsWithChildren<ModalProps>) {
  return <div className="modal-backdrop" role="presentation" onMouseDown={onClose}>
    <section className={`modal ${wide ? "modal-wide" : ""}`} role="dialog" aria-modal="true" aria-label={title} onMouseDown={(event) => event.stopPropagation()}>
      <header className="modal-header">
        <div className="modal-heading">{icon && <span className="modal-icon">{icon}</span>}<div><h2>{title}</h2>{description && <p>{description}</p>}</div></div>
        <button className="icon-button" onClick={onClose} aria-label="Fechar"><X size={19} /></button>
      </header>
      <div className="modal-body">{children}</div>
    </section>
  </div>;
}
