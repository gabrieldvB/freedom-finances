import { Check, Pencil, RotateCcw, Trash2, XCircle } from "lucide-react";
import { formatDate } from "../lib/dates";
import { formatMoney } from "../lib/money";
import type { Transaction } from "../types";
import { EmptyState, StatusBadge } from "./Ui";

interface Props {
  rows: Transaction[];
  compact?: boolean;
  onEdit?: (row: Transaction) => void;
  onStatus?: (row: Transaction, status: "open" | "paid" | "cancelled") => void;
  onDelete?: (row: Transaction) => void;
}

export function TransactionTable({ rows, compact = false, onEdit, onStatus, onDelete }: Props) {
  if (!rows.length) return <EmptyState>Nenhum lançamento encontrado.</EmptyState>;
  return (
    <div className="table-wrap">
      <table>
        <thead><tr>
          <th>Data</th><th>Descrição</th>{!compact && <th>Categoria</th>}<th>Origem</th><th>Status</th><th className="number">Valor</th>{!compact && <th />}
        </tr></thead>
        <tbody>
          {rows.map((row) => (
            <tr key={row.id} className={row.status === "cancelled" ? "muted-row" : ""}>
              <td>{formatDate(row.transactionDate)}</td>
              <td><strong>{row.description}</strong>{row.installmentNumber && <small className="block">Parcela {row.installmentNumber}/{row.installmentTotal}</small>}</td>
              {!compact && <td>{row.categoryName ?? "—"}</td>}
              <td>{row.creditCardName ?? row.accountName ?? "—"}</td>
              <td><StatusBadge status={row.status} /></td>
              <td className={`number amount-${row.type}`}>{row.type === "expense" ? "−" : row.type === "income" ? "+" : ""}{formatMoney(row.amountCents)}</td>
              {!compact && <td className="row-actions">
                {onEdit && <button className="icon-button" title="Editar" onClick={() => onEdit(row)}><Pencil size={16} /></button>}
                {onStatus && row.status !== "paid" && <button className="icon-button success" title="Marcar como pago" onClick={() => onStatus(row, "paid")}><Check size={16} /></button>}
                {onStatus && row.status !== "open" && <button className="icon-button" title="Reabrir" onClick={() => onStatus(row, "open")}><RotateCcw size={16} /></button>}
                {onStatus && row.status !== "cancelled" && <button className="icon-button warning" title={row.recurringOccurrenceId ? "Cancelar ocorrência" : "Cancelar"} onClick={() => onStatus(row, "cancelled")}><XCircle size={16} /></button>}
                {onDelete && !row.recurringOccurrenceId && <button className="icon-button danger" title="Excluir" onClick={() => onDelete(row)}><Trash2 size={16} /></button>}
              </td>}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
