import { useEffect, useState } from "react";
import { ArrowDownCircle, ArrowUpCircle, Clock3, CreditCard, Plus, Scale, Wallet } from "lucide-react";
import type { Page } from "../App";
import { MonthPicker } from "../components/MonthPicker";
import { TransactionTable } from "../components/TransactionTable";
import { EmptyState, PageHeader, Spinner } from "../components/Ui";
import { api, errorMessage } from "../lib/api";
import { formatDate, formatMonth } from "../lib/dates";
import { formatMoney } from "../lib/money";
import type { DashboardData } from "../types";

export default function Dashboard({ fiscalMonth, onMonthChange, onNavigate }: { fiscalMonth: string; onMonthChange: (value: string) => void; onNavigate: (page: Page) => void }) {
  const [data, setData] = useState<DashboardData | null>(null);
  const [error, setError] = useState("");

  useEffect(() => {
    setError("");
    api.dashboard(fiscalMonth).then(setData).catch((err) => setError(errorMessage(err)));
  }, [fiscalMonth]);

  return (
    <>
      <PageHeader title="Visão geral" subtitle="Seu dinheiro, sem nuvem e sem intermediários." actions={<><MonthPicker value={fiscalMonth} onChange={onMonthChange} /><button className="primary" onClick={() => onNavigate("transactions")}><Plus size={17} /> Novo lançamento</button></>} />
      {error && <div className="notice notice-error">{error}</div>}
      {!data ? <Spinner /> : <>
        <section className="metric-grid">
          <Metric label="Receitas" value={data.incomeCents} icon={<ArrowUpCircle />} tone="positive" />
          <Metric label="Despesas" value={data.expenseCents} icon={<ArrowDownCircle />} tone="negative" />
          <Metric label="Saldo do mês" value={data.balanceCents} icon={<Scale />} tone={data.balanceCents >= 0 ? "positive" : "negative"} />
          <Metric label="Em aberto" value={data.openCents} icon={<Clock3 />} tone="warning" />
          <Metric label="Pago" value={data.paidCents} icon={<Wallet />} />
          <Metric label="Faturas abertas" value={data.openInvoicesTotalCents} icon={<CreditCard />} tone="card" />
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
    </>
  );
}

function Metric({ label, value, icon, tone = "neutral" }: { label: string; value: number; icon: React.ReactNode; tone?: string }) {
  return <article className={`metric metric-${tone}`}><div className="metric-icon">{icon}</div><span>{label}</span><strong>{formatMoney(value)}</strong></article>;
}
