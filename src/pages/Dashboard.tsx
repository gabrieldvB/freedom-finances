import { useCallback, useEffect, useState } from "react";
import { ArrowDownCircle, ArrowUpCircle, CalendarCheck2, CircleDollarSign, Clock3, CreditCard, Landmark, Plus, Scale, TrendingUp, Wallet } from "lucide-react";
import type { Page } from "../App";
import { MonthPicker } from "../components/MonthPicker";
import { TransactionTable } from "../components/TransactionTable";
import { EmptyState, Modal, Notice, PageHeader, Spinner } from "../components/Ui";
import { api, errorMessage } from "../lib/api";
import { formatDate, formatMonth } from "../lib/dates";
import { formatMoney, moneyInput, parseMoney } from "../lib/money";
import type { DashboardData } from "../types";

export default function Dashboard({ fiscalMonth, defaultFiscalMonth, onMonthChange, onDefaultMonthChange, onNavigate }: { fiscalMonth: string; defaultFiscalMonth: string; onMonthChange: (value: string) => void; onDefaultMonthChange: (value: string) => void; onNavigate: (page: Page) => void }) {
  const [data, setData] = useState<DashboardData | null>(null);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [openingBalanceOpen, setOpeningBalanceOpen] = useState(false);

  const load = useCallback(async () => {
    setError("");
    try { setData(await api.dashboard(fiscalMonth)); }
    catch (err) { setError(errorMessage(err)); }
  }, [fiscalMonth]);

  useEffect(() => { void load(); }, [load]);

  async function saveDefaultMonth() {
    setError("");
    setNotice("");
    try {
      const saved = await api.setDefaultFiscalMonth(fiscalMonth);
      onDefaultMonthChange(saved);
      setNotice("Mês fiscal padrão atualizado.");
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  return (
    <>
      <PageHeader title="Visão geral" subtitle="Seu dinheiro, sem nuvem e sem intermediários." actions={<><MonthPicker value={fiscalMonth} onChange={(month) => { setNotice(""); onMonthChange(month); }} /><button className="secondary" onClick={() => setOpeningBalanceOpen(true)}><Landmark size={17} /> Definir saldo inicial</button><button className="secondary" disabled={fiscalMonth === defaultFiscalMonth} onClick={() => void saveDefaultMonth()}><CalendarCheck2 size={17} /> Definir como padrão</button><button className="primary" onClick={() => onNavigate("transactions")}><Plus size={17} /> Novo lançamento</button></>} />
      {error && <div className="notice notice-error">{error}</div>}
      {notice && <Notice type="success">{notice}</Notice>}
      {!data ? <Spinner /> : <>
        <section className="metric-grid">
          <Metric label="Receitas do mês" value={data.projectedIncomeCents} icon={<ArrowUpCircle />} tone="positive" />
          <Metric label="Despesas diretas" value={data.projectedDirectExpensesCents} icon={<ArrowDownCircle />} tone="negative" />
          <Metric label="Faturas vencendo no mês" value={data.projectedCardInvoicesCents} icon={<CreditCard />} tone="card" />
          <Metric label="Saldo projetado do mês" value={data.projectedBalanceCents} icon={<Scale />} tone={data.projectedBalanceCents >= 0 ? "positive" : "negative"} help="Saldo projetado do mês considera receitas, despesas e faturas previstas, independentemente de já estarem pagas." />
          <Metric label="Em aberto" value={data.openCents} icon={<Clock3 />} tone="warning" />
          <Metric label="Já pago" value={data.paidCents} icon={<Wallet />} />
          <Metric label="Já recebido" value={data.receivedCents} icon={<CircleDollarSign />} tone="positive" />
          <Metric label="Saldo inicial do mês" value={data.openingBalanceCents} icon={<Landmark />} />
          <Metric label="Saldo realizado" value={data.realizedBalanceCents} icon={<Wallet />} tone={data.realizedBalanceCents >= 0 ? "positive" : "negative"} help="Saldo realizado considera apenas o que já foi pago ou recebido." />
          <Metric label="Saldo acumulado projetado" value={data.projectedAccumulatedBalanceCents} icon={<TrendingUp />} tone={data.projectedAccumulatedBalanceCents >= 0 ? "positive" : "negative"} />
        </section>

        <div className="dashboard-grid">
          <section className="panel">
            <div className="panel-title"><h2>Gastos por categoria</h2><span>{formatMonth(fiscalMonth)}</span></div>
            {data.categoryTotals.length ? <div className="category-bars">
              {data.categoryTotals.map((item) => {
                const max = Math.max(...data.categoryTotals.map((row) => row.totalCents), 1);
                return <div key={item.category} className="category-bar">
                  <div><span><i style={{ background: item.color }} />{item.category}</span><strong>{formatMoney(item.totalCents)}</strong></div>
                  <div className="bar-track"><div style={{ width: `${Math.max(4, item.totalCents / max * 100)}%`, background: item.color }} /></div>
                </div>;
              })}
            </div> : <EmptyState>Sem despesas neste mês.</EmptyState>}
          </section>
          <section className="panel">
            <div className="panel-title"><h2>Faturas abertas</h2><button className="link-button" onClick={() => onNavigate("cards")}>Ver cartões</button></div>
            {data.openInvoices.length ? <div className="invoice-list compact">
              {data.openInvoices.map((invoice) => <button key={invoice.id} onClick={() => onNavigate("cards")}>
                <span><strong>{invoice.creditCard}</strong><small>{formatMonth(invoice.fiscalMonth)} · vence {formatDate(invoice.dueDate)}</small></span>
                <strong>{formatMoney(invoice.totalCents)}</strong>
              </button>)}
            </div> : <EmptyState>Nenhuma fatura aberta no mês.</EmptyState>}
          </section>
          <section className="panel panel-wide">
            <div className="panel-title"><h2>Próximas faturas</h2></div>
            {data.upcomingInvoices.length ? <div className="invoice-chips">{data.upcomingInvoices.map((invoice) => <div key={invoice.id}><span>{invoice.creditCard} · {formatMonth(invoice.fiscalMonth)}</span><strong>{formatMoney(invoice.totalCents)}</strong></div>)}</div> : <EmptyState>Nenhuma fatura futura.</EmptyState>}
          </section>
          <section className="panel panel-wide">
            <div className="panel-title"><h2>Últimos lançamentos</h2><button className="link-button" onClick={() => onNavigate("transactions")}>Ver todos</button></div>
            <TransactionTable rows={data.recentTransactions} compact />
          </section>
        </div>
      </>}
      {openingBalanceOpen && <OpeningBalanceForm fiscalMonth={fiscalMonth} initialValue={data?.openingBalanceCents ?? 0} onClose={() => setOpeningBalanceOpen(false)} onSaved={async () => { setOpeningBalanceOpen(false); setNotice("Saldo inicial atualizado."); await load(); }} />}
    </>
  );
}

function Metric({ label, value, icon, tone = "neutral", help }: { label: string; value: number; icon: React.ReactNode; tone?: string; help?: string }) {
  return <article className={`metric metric-${tone}`} title={help}><div className="metric-icon">{icon}</div><span>{label}</span><strong>{formatMoney(value)}</strong></article>;
}

function OpeningBalanceForm({ fiscalMonth, initialValue, onClose, onSaved }: { fiscalMonth: string; initialValue: number; onClose: () => void; onSaved: () => Promise<void> }) {
  const [amount, setAmount] = useState(moneyInput(initialValue));
  const [notes, setNotes] = useState("");
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    api.getMonthlyBalance(fiscalMonth).then((balance) => {
      setAmount(moneyInput(balance.openingBalanceCents));
      setNotes(balance.notes ?? "");
    }).catch((err) => setError(errorMessage(err)));
  }, [fiscalMonth]);

  async function submit(event: React.FormEvent) {
    event.preventDefault();
    const cents = parseMoney(amount);
    if (cents === null) return setError("Informe um saldo inicial válido.");
    setSaving(true); setError("");
    try {
      await api.setMonthlyBalance(fiscalMonth, cents, notes.trim() || null);
      await onSaved();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setSaving(false);
    }
  }

  return <Modal title="Definir saldo inicial" onClose={onClose}><form className="form-grid" onSubmit={submit}>
    {error && <div className="notice notice-error full">{error}</div>}
    <label>Mês fiscal<input type="month" value={fiscalMonth} disabled /></label>
    <label>Saldo inicial (R$)<input value={amount} onChange={(event) => setAmount(event.target.value)} inputMode="decimal" autoFocus /></label>
    <label className="full">Observação opcional<textarea rows={3} value={notes} onChange={(event) => setNotes(event.target.value)} /></label>
    <div className="form-actions full"><button type="button" className="ghost" onClick={onClose}>Cancelar</button><button className="primary" disabled={saving}>{saving ? "Salvando…" : "Salvar saldo inicial"}</button></div>
  </form></Modal>;
}
