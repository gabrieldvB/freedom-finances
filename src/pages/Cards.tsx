import { useCallback, useEffect, useMemo, useState } from "react";
import { CreditCard as CardIcon, Pencil, Plus, Power } from "lucide-react";
import { Modal, Notice, PageHeader, Spinner, StatusBadge } from "../components/Ui";
import { api, errorMessage } from "../lib/api";
import { currentFiscalMonth, formatDate, formatMonth } from "../lib/dates";
import { formatMoney, moneyInput, parseMoney } from "../lib/money";
import type { CreditCard, Invoice } from "../types";
import InvoiceDetail from "./InvoiceDetail";

export default function Cards() {
  const [cards, setCards] = useState<CreditCard[]>([]); const [invoices, setInvoices] = useState<Invoice[]>([]);
  const [editing, setEditing] = useState<CreditCard | "new" | null>(null); const [selectedInvoice, setSelectedInvoice] = useState<number | null>(null);
  const [loading, setLoading] = useState(true); const [error, setError] = useState("");
  const load = useCallback(async () => { setLoading(true); setError(""); try { const [cardRows, invoiceRows] = await Promise.all([api.listCards(), api.listInvoices()]); setCards(cardRows); setInvoices(invoiceRows); } catch (err) { setError(errorMessage(err)); } finally { setLoading(false); } }, []);
  useEffect(() => { void load(); }, [load]);

  async function toggle(card: CreditCard) { try { await api.setCardActive(card.id, !card.active); await load(); } catch (err) { setError(errorMessage(err)); } }
  return <>
    <PageHeader title="Cartões e faturas" subtitle="Acompanhe o presente e os próximos meses." actions={<button className="primary" onClick={() => setEditing("new")}><Plus size={17} /> Novo cartão</button>} />
    {error && <Notice type="error">{error}</Notice>}
    {loading ? <Spinner /> : cards.length === 0 ? <section className="panel empty-state">Cadastre seu primeiro cartão para gerar faturas automaticamente.</section> : <div className="cards-stack">
      {cards.map((card) => <CardSection key={card.id} card={card} invoices={invoices.filter((row) => row.creditCardId === card.id)} onEdit={() => setEditing(card)} onToggle={() => void toggle(card)} onInvoice={setSelectedInvoice} />)}
    </div>}
    {editing && <CardForm card={editing === "new" ? null : editing} onClose={() => setEditing(null)} onSaved={async () => { setEditing(null); await load(); }} />}
    {selectedInvoice !== null && <InvoiceDetail id={selectedInvoice} onClose={() => setSelectedInvoice(null)} onChanged={load} />}
  </>;
}

function CardSection({ card, invoices, onEdit, onToggle, onInvoice }: { card: CreditCard; invoices: Invoice[]; onEdit: () => void; onToggle: () => void; onInvoice: (id: number) => void }) {
  const current = currentFiscalMonth();
  const groups = useMemo(() => ({
    current: invoices.filter((row) => row.fiscalMonth === current),
    open: invoices.filter((row) => row.fiscalMonth < current && ["open", "closed"].includes(row.status)),
    future: invoices.filter((row) => row.fiscalMonth > current && row.status !== "cancelled"),
    history: invoices.filter((row) => ["paid", "cancelled"].includes(row.status)),
  }), [invoices, current]);
  return <section className={`panel card-section ${!card.active ? "inactive" : ""}`}>
    <div className="card-heading">
      <div className="credit-card-visual"><CardIcon size={25} /><span>{card.name}</span><small>Fecha dia {card.closingDay} · vence dia {card.dueDay}</small><strong>Limite {formatMoney(card.limitCents)}</strong></div>
      <div className="row-actions"><button className="icon-button" title="Editar" onClick={onEdit}><Pencil size={17} /></button><button className="icon-button" title={card.active ? "Desativar" : "Ativar"} onClick={onToggle}><Power size={17} /></button></div>
    </div>
    <div className="invoice-groups">
      <InvoiceGroup title="Fatura atual" rows={groups.current} onOpen={onInvoice} />
      <InvoiceGroup title="Faturas abertas" rows={groups.open} onOpen={onInvoice} />
      <InvoiceGroup title="Próximas faturas" rows={groups.future} onOpen={onInvoice} />
      <InvoiceGroup title="Histórico" rows={groups.history} onOpen={onInvoice} />
    </div>
  </section>;
}

function InvoiceGroup({ title, rows, onOpen }: { title: string; rows: Invoice[]; onOpen: (id: number) => void }) {
  return <div className="invoice-group"><h3>{title}</h3>{rows.length ? rows.map((invoice) => <button key={invoice.id} className="invoice-row" onClick={() => onOpen(invoice.id)}>
    <div><strong>{formatMonth(invoice.fiscalMonth)}</strong><StatusBadge status={invoice.status} /></div>
    <span>Fecha {formatDate(invoice.closingDate)}</span><span>Vence {formatDate(invoice.dueDate)}</span><span>{invoice.transactionCount} lançamento(s)</span><strong>{formatMoney(invoice.totalCents)}</strong>
  </button>) : <p className="muted">Nenhuma.</p>}</div>;
}

function CardForm({ card, onClose, onSaved }: { card: CreditCard | null; onClose: () => void; onSaved: () => Promise<void> }) {
  const [name, setName] = useState(card?.name ?? ""); const [closingDay, setClosingDay] = useState(card?.closingDay ?? 20); const [dueDay, setDueDay] = useState(card?.dueDay ?? 10); const [limit, setLimit] = useState(card ? moneyInput(card.limitCents) : "0,00");
  const [error, setError] = useState(""); const [saving, setSaving] = useState(false);
  async function submit(event: React.FormEvent) { event.preventDefault(); const cents = parseMoney(limit); if (!name.trim()) return setError("Informe o nome."); if (cents === null || cents < 0) return setError("Informe um limite válido."); if (closingDay < 1 || closingDay > 31 || dueDay < 1 || dueDay > 31) return setError("Dias devem estar entre 1 e 31."); setSaving(true); try { await api.saveCard({ id: card?.id, name: name.trim(), closingDay, dueDay, limitCents: cents }); await onSaved(); } catch (err) { setError(errorMessage(err)); } finally { setSaving(false); } }
  return <Modal title={card ? "Editar cartão" : "Novo cartão"} onClose={onClose}><form className="form-grid" onSubmit={submit}>{error && <div className="notice notice-error full">{error}</div>}<label className="full">Nome<input value={name} onChange={(e) => setName(e.target.value)} autoFocus /></label><label>Dia de fechamento<input type="number" min="1" max="31" value={closingDay} onChange={(e) => setClosingDay(Number(e.target.value))} /></label><label>Dia de vencimento<input type="number" min="1" max="31" value={dueDay} onChange={(e) => setDueDay(Number(e.target.value))} /></label><label className="full">Limite (R$)<input value={limit} onChange={(e) => setLimit(e.target.value)} inputMode="decimal" /></label><div className="form-actions full"><button type="button" className="ghost" onClick={onClose}>Cancelar</button><button className="primary" disabled={saving}>Salvar</button></div></form></Modal>;
}
