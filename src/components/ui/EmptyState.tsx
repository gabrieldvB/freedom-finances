import type { PropsWithChildren, ReactNode } from "react";
import { Inbox } from "lucide-react";

interface EmptyStateProps {
  icon?: ReactNode;
  title?: string;
  description?: string;
  action?: ReactNode;
}

export function EmptyState({ icon = <Inbox size={22} />, title, description, action, children }: PropsWithChildren<EmptyStateProps>) {
  return <div className="empty-state">
    <span className="empty-state-icon">{icon}</span>
    {title && <strong>{title}</strong>}
    {description && <p>{description}</p>}
    {!title && children && <p>{children}</p>}
    {action && <div className="empty-state-action">{action}</div>}
  </div>;
}
