import { useCallback, useEffect, useState } from "react";
import { Check, LockKeyhole, RotateCcw, XCircle } from "lucide-react";
import { TransactionTable } from "../components/TransactionTable";
import { Modal, Spinner, StatusBadge } from "../components/Ui";
import { api, errorMessage } from "../lib/api";
import { formatDate, formatMonth } from "../lib/dates";
import { formatMoney } from "../lib/money";
import type { Invoice, InvoiceStatus, Transaction } from "../types";

export default function InvoiceDetail({ id, onClose, onChanged }: { id: number; onClose: () => void; onChanged: () => Promise<void> }) {
  const [invoice, setInvoice] = useState<Invoice | null>(null); const [rows, setRows] = useState<Transaction[]>([]); const [error, setError] = useState(""); const [busy, setBusy] = useState(false);
  const load = useCallback(async () => { try { const [data, transactions] = await Promise.all([api.getInvoice(id), api.getInvoiceTransactions(id)]); setInvoice(data); setRows(transactions); } catch (err) { setError(errorMessage(err)); } }, [id]);
  useEffect(() => { void load(); }, [load]);
  async function change(status: InvoiceStatus) { if (status === "paid" && !window.confirm("Pagar esta fatura e marcar todos os lançamentos abertos como pagos?")) return; setBusy(true); setError(""); try { await api.setInvoiceStatus(id, status); await load(); await onChanged(); } catch (err) { setError(errorMessage(err)); } finally { setBusy(false); } }
  return <Modal title="Detalhe da fatura" onClose={onClose} wide>
    {error && <div className="notice notice-error">{error}</div>}
    {!invoice ? <Spinner /> : <>
      <div className="invoice-summary"><div><span>Cartão</span><strong>{invoice.creditCard}</strong></div><div><span>Mês fiscal</span><strong>{formatMonth(invoice.fiscalMonth)}</strong></div><div><span>Status</span><StatusBadge status={invoice.status} /></div><div><span>Fechamento</span><strong>{formatDate(invoice.closingDate)}</strong></div><div><span>Vencimento</span><strong>{formatDate(invoice.dueDate)}</strong></div><div><span>Origem</span><strong>{invoice.declaredAt ? "Fatura declarada" : "Fatura automática"}</strong></div></div>
      <div className="invoice-breakdown"><div><span>Saldo declarado</span><strong>{formatMoney(invoice.declaredAmountCents)}</strong></div><div><span>Lançamentos no app</span><strong>{formatMoney(invoice.transactionsTotalCents)}</strong></div><div><span>Total da fatura</span><strong className="large-money">{formatMoney(invoice.totalCents)}</strong></div></div>
      {invoice.declaredNote && <p className="invoice-note"><strong>Observação:</strong> {invoice.declaredNote}</p>}
      <div className="invoice-actions"><button className="secondary" disabled={busy || invoice.status === "closed"} onClick={() => void change("closed")}><LockKeyhole size={16} /> Fechar</button><button className="primary" disabled={busy || invoice.status === "paid"} onClick={() => void change("paid")}><Check size={16} /> Pagar fatura</button><button className="ghost" disabled={busy || invoice.status === "open"} onClick={() => void change("open")}><RotateCcw size={16} /> Reabrir</button><button className="danger-button" disabled={busy || invoice.status === "cancelled"} onClick={() => void change("cancelled")}><XCircle size={16} /> Cancelar</button></div>
      <h3 className="section-title">Lançamentos ({rows.length})</h3><TransactionTable rows={rows} compact />
    </>}
  </Modal>;
}
