import { useEffect, useState } from "react";
import { CreditCard, LayoutDashboard, ListChecks, Settings as SettingsIcon, Shapes, WalletCards } from "lucide-react";
import { currentFiscalMonth } from "./lib/dates";
import Dashboard from "./pages/Dashboard";
import Transactions from "./pages/Transactions";
import Cards from "./pages/Cards";
import Categories from "./pages/Categories";
import Settings from "./pages/Settings";
import { Spinner } from "./components/Ui";
import { api } from "./lib/api";

export type Page = "dashboard" | "transactions" | "cards" | "categories" | "settings";

const navigation = [
  { id: "dashboard" as const, label: "Dashboard", icon: LayoutDashboard },
  { id: "transactions" as const, label: "Lançamentos", icon: ListChecks },
  { id: "cards" as const, label: "Cartões e faturas", icon: WalletCards },
  { id: "categories" as const, label: "Categorias", icon: Shapes },
  { id: "settings" as const, label: "Configurações", icon: SettingsIcon },
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
    if (defaultFiscalMonth && (nextPage === "dashboard" || nextPage === "transactions")) {
      setFiscalMonth(defaultFiscalMonth);
    }
    setPage(nextPage);
  }

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark"><CreditCard size={24} /></div>
          <div><strong>Freedom</strong><span>Finances</span></div>
        </div>
        <nav>
          {navigation.map(({ id, label, icon: Icon }) => (
            <button key={id} className={page === id ? "active" : ""} onClick={() => navigate(id)}>
              <Icon size={19} /> {label}
            </button>
          ))}
        </nav>
        <div className="sidebar-footer"><span className="offline-dot" /> Dados locais · offline</div>
      </aside>
      <main className="content">
        {!fiscalMonth || !defaultFiscalMonth ? <Spinner /> : <>
          {page === "dashboard" && <Dashboard fiscalMonth={fiscalMonth} defaultFiscalMonth={defaultFiscalMonth} onMonthChange={setFiscalMonth} onDefaultMonthChange={setDefaultFiscalMonth} onNavigate={navigate} />}
          {page === "transactions" && <Transactions fiscalMonth={fiscalMonth} defaultFiscalMonth={defaultFiscalMonth} onMonthChange={setFiscalMonth} />}
          {page === "cards" && <Cards defaultFiscalMonth={defaultFiscalMonth} />}
          {page === "categories" && <Categories />}
          {page === "settings" && <Settings defaultFiscalMonth={defaultFiscalMonth} />}
        </>}
      </main>
    </div>
  );
}
