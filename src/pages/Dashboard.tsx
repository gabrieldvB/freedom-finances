import { useCallback, useEffect, useState } from "react";
import { ArrowDownCircle, ArrowUpCircle, CalendarCheck2, CircleDollarSign, CreditCard, Landmark, Plus, Scale, TrendingUp, Wallet } from "lucide-react";
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
  const [initialBalanceOpen, setInitialBalanceOpen] = useState(false);

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
      <PageHeader title="Visão geral" subtitle="Seu dinheiro, sem nuvem e sem intermediários." actions={<><MonthPicker value={fiscalMonth} onChange={(month) => { setNotice(""); onMonthChange(month); }} /><button className="secondary" onClick={() => setInitialBalanceOpen(true)}><Landmark size={17} /> Saldo inicial do controle</button><button className="secondary" disabled={fiscalMonth === defaultFiscalMonth} onClick={() => void saveDefaultMonth()}><CalendarCheck2 size={17} /> Definir como padrão</button><button className="primary" onClick={() => onNavigate("transactions")}><Plus size={17} /> Novo lançamento</button></>} />
      {error && <div className="notice notice-error">{error}</div>}
      {notice && <Notice type="success">{notice}</Notice>}
      {!data ? <Spinner /> : <>
        <section className="metric-grid">
          <Metric label="Receitas previstas" value={data.projectedIncomeCents} icon={<ArrowUpCircle />} tone="positive" />
          <Metric label="Despesas diretas previstas" value={data.projectedDirectExpensesCents} icon={<ArrowDownCircle />} tone="negative" />
          <Metric label="Faturas previstas" value={data.projectedCardInvoicesCents} icon={<CreditCard />} tone="card" />
          <Metric label="Resultado projetado do mês" value={data.projectedBalanceCents} icon={<Scale />} tone={data.projectedBalanceCents >= 0 ? "positive" : "negative"} help="Considera receitas, despesas e faturas previstas, independentemente de já estarem pagas." />
          <Metric label="Receitas recebidas" value={data.realizedIncomeCents} icon={<CircleDollarSign />} tone="positive" />
          <Metric label="Despesas pagas" value={data.realizedDirectExpensesCents} icon={<Wallet />} tone="negative" />
          <Metric label="Faturas pagas" value={data.realizedCardInvoicesCents} icon={<CreditCard />} tone="card" />
          <Metric label="Resultado realizado do mês" value={data.realizedMonthResultCents} icon={<Wallet />} tone={data.realizedMonthResultCents >= 0 ? "positive" : "negative"} help="Considera apenas receitas recebidas, despesas pagas e faturas pagas." />
          <Metric label="Saldo acumulado projetado" value={data.projectedAccumulatedBalanceCents} icon={<TrendingUp />} tone={data.projectedAccumulatedBalanceCents >= 0 ? "positive" : "negative"} />
          <Metric label="Saldo acumulado realizado" value={data.realizedAccumulatedBalanceCents} icon={<Landmark />} tone={data.realizedAccumulatedBalanceCents >= 0 ? "positive" : "negative"} />
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
      {initialBalanceOpen && <InitialBalanceForm initialValue={data?.initialGlobalBalanceCents ?? 0} onClose={() => setInitialBalanceOpen(false)} onSaved={async () => { setInitialBalanceOpen(false); setNotice("Saldo inicial do controle atualizado."); await load(); }} />}
    </>
  );
}

function Metric({ label, value, icon, tone = "neutral", help }: { label: string; value: number; icon: React.ReactNode; tone?: string; help?: string }) {
  return <article className={`metric metric-${tone}`} title={help}><div className="metric-icon">{icon}</div><span>{label}</span><strong>{formatMoney(value)}</strong></article>;
}

function InitialBalanceForm({ initialValue, onClose, onSaved }: { initialValue: number; onClose: () => void; onSaved: () => Promise<void> }) {
  const [amount, setAmount] = useState(moneyInput(initialValue));
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    api.getInitialGlobalBalance()
      .then((balance) => setAmount(moneyInput(balance)))
      .catch((err) => setError(errorMessage(err)));
  }, []);

  async function submit(event: React.FormEvent) {
    event.preventDefault();
    const cents = parseMoney(amount);
    if (cents === null) return setError("Informe um saldo inicial válido.");
    setSaving(true); setError("");
    try {
      await api.setInitialGlobalBalance(cents);
      await onSaved();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setSaving(false);
    }
  }

  return <Modal title="Saldo inicial do controle" onClose={onClose}><form className="form-grid" onSubmit={submit}>
    {error && <div className="notice notice-error full">{error}</div>}
    <label className="full">Saldo inicial (R$)<input value={amount} onChange={(event) => setAmount(event.target.value)} inputMode="decimal" autoFocus /></label>
    <p className="form-hint full">Esse valor é a base única dos saldos acumulados projetado e realizado.</p>
    <div className="form-actions full"><button type="button" className="ghost" onClick={onClose}>Cancelar</button><button className="primary" disabled={saving}>{saving ? "Salvando…" : "Salvar saldo inicial"}</button></div>
  </form></Modal>;
}
