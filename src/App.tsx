import { useEffect, useState } from "react";
import { CreditCard, LayoutDashboard, ReceiptText, Repeat, Settings as SettingsIcon, Tags } from "lucide-react";
import { currentFiscalMonth } from "./lib/dates";
import Dashboard from "./pages/Dashboard";
import Transactions from "./pages/Transactions";
import Cards from "./pages/Cards";
import Categories from "./pages/Categories";
import Settings from "./pages/Settings";
import Recurring from "./pages/Recurring";
import { Spinner } from "./components/Ui";
import { api } from "./lib/api";

export type Page = "dashboard" | "transactions" | "recurring" | "cards" | "categories" | "settings";

const navigation = [
  { id: "dashboard" as const, label: "Dashboard", icon: LayoutDashboard },
  { id: "transactions" as const, label: "Lançamentos", icon: ReceiptText },
  { id: "recurring" as const, label: "Fixos", icon: Repeat },
  { id: "cards" as const, label: "Cartões e faturas", icon: CreditCard },
  { id: "categories" as const, label: "Categorias", icon: Tags },
];

export default function App() {
  const [page, setPage] = useState<Page>("dashboard");
  const [fiscalMonth, setFiscalMonth] = useState<string | null>(null);
  const [defaultFiscalMonth, setDefaultFiscalMonth] = useState<string | null>(null);

  useEffect(() => {
    api.getDefaultFiscalMonth()
      .then((month) => { setDefaultFiscalMonth(month); setFiscalMonth(month); })
      .catch(() => { const fallback = currentFiscalMonth(); setDefaultFiscalMonth(fallback); setFiscalMonth(fallback); });
  }, []);

  function navigate(nextPage: Page) {
    setPage(nextPage);
  }

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark" aria-hidden="true">💰</div>
          <div className="brand-copy"><strong>Freedom Finances</strong><span>Controle financeiro local</span></div>
        </div>
        <nav className="sidebar-nav" aria-label="Navegação principal">
          {navigation.map(({ id, label, icon: Icon }) => (
            <button key={id} className={`nav-item ${page === id ? "active" : ""}`} onClick={() => navigate(id)}>
              <span className="nav-icon"><Icon size={18} /></span><span>{label}</span>
            </button>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <button className={`nav-item ${page === "settings" ? "active" : ""}`} onClick={() => navigate("settings")}><span className="nav-icon"><SettingsIcon size={18} /></span><span>Configurações</span></button>
          <div className="sidebar-footer"><span className="offline-dot" /><span><strong>Privado e offline</strong><small>Dados neste dispositivo</small></span></div>
        </div>
      </aside>
      <main className="content">
        {!fiscalMonth || !defaultFiscalMonth ? <Spinner /> : <>
          {page === "dashboard" && <Dashboard fiscalMonth={fiscalMonth} defaultFiscalMonth={defaultFiscalMonth} onMonthChange={setFiscalMonth} onDefaultMonthChange={setDefaultFiscalMonth} onNavigate={navigate} />}
          {page === "transactions" && <Transactions fiscalMonth={fiscalMonth} defaultFiscalMonth={defaultFiscalMonth} onMonthChange={setFiscalMonth} />}
          {page === "recurring" && <Recurring fiscalMonth={fiscalMonth} defaultFiscalMonth={defaultFiscalMonth} onMonthChange={setFiscalMonth} onOpenTransactions={() => navigate("transactions")} />}
          {page === "cards" && <Cards fiscalMonth={fiscalMonth} defaultFiscalMonth={defaultFiscalMonth} onMonthChange={setFiscalMonth} />}
          {page === "categories" && <Categories />}
          {page === "settings" && <Settings defaultFiscalMonth={defaultFiscalMonth} />}
        </>}
      </main>
    </div>
  );
}
