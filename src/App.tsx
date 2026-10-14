import { useState } from "react";
import { CreditCard, LayoutDashboard, ListChecks, Settings as SettingsIcon, Shapes, WalletCards } from "lucide-react";
import { currentFiscalMonth } from "./lib/dates";
import Dashboard from "./pages/Dashboard";
import Transactions from "./pages/Transactions";
import Cards from "./pages/Cards";
import Categories from "./pages/Categories";
import Settings from "./pages/Settings";

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
  const [fiscalMonth, setFiscalMonth] = useState(currentFiscalMonth());

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark"><CreditCard size={24} /></div>
          <div><strong>Freedom</strong><span>Finances</span></div>
        </div>
        <nav>
          {navigation.map(({ id, label, icon: Icon }) => (
            <button key={id} className={page === id ? "active" : ""} onClick={() => setPage(id)}>
              <Icon size={19} /> {label}
            </button>
          ))}
        </nav>
        <div className="sidebar-footer"><span className="offline-dot" /> Dados locais · offline</div>
      </aside>
      <main className="content">
        {page === "dashboard" && <Dashboard fiscalMonth={fiscalMonth} onMonthChange={setFiscalMonth} onNavigate={setPage} />}
        {page === "transactions" && <Transactions fiscalMonth={fiscalMonth} onMonthChange={setFiscalMonth} />}
        {page === "cards" && <Cards />}
        {page === "categories" && <Categories />}
        {page === "settings" && <Settings />}
      </main>
    </div>
  );
}
