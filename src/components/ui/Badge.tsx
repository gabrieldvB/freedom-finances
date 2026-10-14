import { Check, CircleDot, LockKeyhole, TrendingDown, TrendingUp, X } from "lucide-react";

const labels: Record<string, string> = {
  open: "Em aberto",
  paid: "Pago",
  cancelled: "Cancelado",
  closed: "Fechada",
  income: "Receita",
  expense: "Despesa",
  transfer: "Transferência",
};

const icons: Record<string, React.ReactNode> = {
  open: <CircleDot size={11} />,
  paid: <Check size={11} />,
  cancelled: <X size={11} />,
  closed: <LockKeyhole size={11} />,
  income: <TrendingUp size={11} />,
  expense: <TrendingDown size={11} />,
};

export function StatusBadge({ status }: { status: string }) {
  return <span className={`status-badge status-${status}`}>{icons[status]}{labels[status] ?? status}</span>;
}
