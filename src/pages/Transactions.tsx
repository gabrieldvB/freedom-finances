import { useCallback, useEffect, useMemo, useState } from "react";
import { AlertTriangle, CreditCard, Plus, ReceiptText } from "lucide-react";
import { MonthPicker } from "../components/MonthPicker";
import { TransactionTable } from "../components/TransactionTable";
import { Modal, Notice, PageHeader, Spinner } from "../components/Ui";
import { ConfirmDialog } from "../components/ui/ConfirmDialog";
import { api, errorMessage } from "../lib/api";
import { today } from "../lib/dates";
import { moneyInput, parseMoney } from "../lib/money";
import type { Account, Category, CreditCard as Card, InstallmentInput, Transaction, TransactionInput, TransactionStatus } from "../types";

export default function Transactions({ fiscalMonth, defaultFiscalMonth, onMonthChange }: { fiscalMonth: string; defaultFiscalMonth: string; onMonthChange: (value: string) => void }) {
  const [rows, setRows] = useState<Transaction[]>([]);
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [categories, setCategories] = useState<Category[]>([]);
  const [cards, setCards] = useState<Card[]>([]);
  const [editing, setEditing] = useState<Transaction | "new" | null>(null);
  const [installmentOpen, setInstallmentOpen] = useState(false);
  const [installmentToDelete, setInstallmentToDelete] = useState<Transaction | null>(null);
  const [transactionToDelete, setTransactionToDelete] = useState<Transaction | null>(null);
  const [deletingTransaction, setDeletingTransaction] = useState(false);
  const [statusFilter, setStatusFilter] = useState("active");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true); setError("");
    try {
      const [transactions, accountRows, categoryRows, cardRows] = await Promise.all([
        api.listTransactions(fiscalMonth), api.listAccounts(), api.listCategories(), api.listCards(),
      ]);
      setRows(transactions); setAccounts(accountRows); setCategories(categoryRows); setCards(cardRows);
    } catch (err) { setError(errorMessage(err)); } finally { setLoading(false); }
  }, [fiscalMonth]);

  useEffect(() => { void load(); }, [load]);
  const filtered = useMemo(() => {
    if (statusFilter === "active") return rows.filter((row) => row.status !== "cancelled");
    if (statusFilter === "all") return rows;
    return rows.filter((row) => row.status === statusFilter);
  }, [rows, statusFilter]);

  async function changeStatus(row: Transaction, status: TransactionStatus) {
    try {
      if (row.recurringOccurrenceId) await api.setRecurringOccurrenceStatus(row.recurringOccurrenceId, status);
      else await api.setTransactionStatus(row.id, status);
      await load();
    } catch (err) { setError(errorMessage(err)); }
  }

  async function remove(row: Transaction) {
    if (row.recurringOccurrenceId) {
      setTransactionToDelete(row);
      return;
    }
    if (row.installmentGroupId) {
      setInstallmentToDelete(row);
      return;
    }
    setTransactionToDelete(row);
  }

  async function confirmTransactionRemoval() {
    if (!transactionToDelete) return;
    setDeletingTransaction(true); setError("");
    try {
      if (transactionToDelete.recurringOccurrenceId) await api.cancelRecurringOccurrence(transactionToDelete.recurringOccurrenceId);
      else await api.deleteTransaction(transactionToDelete.id);
      setTransactionToDelete(null);
      await load();
    }
    catch (err) { setError(errorMessage(err)); }
    finally { setDeletingTransaction(false); }
  }

  return <>
    <PageHeader title="Lançamentos" subtitle="Receitas, despesas e compras parceladas." actions={<><MonthPicker value={fiscalMonth} onChange={onMonthChange} /><button className="secondary" onClick={() => setInstallmentOpen(true)}><CreditCard size={17} /> Compra parcelada</button><button className="primary" onClick={() => setEditing("new")}><Plus size={17} /> Novo</button></>} />
    {error && <Notice type="error">{error}</Notice>}
    <div className="toolbar"><label>Status<select value={statusFilter} onChange={(event) => setStatusFilter(event.target.value)}><option value="active">Ativos</option><option value="all">Todos</option><option value="open">Em aberto</option><option value="paid">Pago</option><option value="cancelled">Cancelados</option></select></label><span>{filtered.length} lançamento(s)</span></div>
    <section className="panel">{loading ? <Spinner /> : <TransactionTable rows={filtered} onEdit={setEditing} onStatus={changeStatus} onDelete={remove} />}</section>
    {editing && <TransactionForm transaction={editing === "new" ? null : editing} fiscalMonth={defaultFiscalMonth} accounts={accounts} categories={categories} cards={cards} onClose={() => setEditing(null)} onSaved={async () => { setEditing(null); await load(); }} />}
    {installmentOpen && <InstallmentForm fiscalMonth={defaultFiscalMonth} categories={categories} cards={cards} onClose={() => setInstallmentOpen(false)} onSaved={async () => { setInstallmentOpen(false); await load(); }} />}
    {installmentToDelete && <InstallmentDeleteModal transaction={installmentToDelete} onClose={() => setInstallmentToDelete(null)} onDeleted={async () => { setInstallmentToDelete(null); await load(); }} />}
    {transactionToDelete && <ConfirmDialog title={transactionToDelete.recurringOccurrenceId ? "Remover ocorrência deste mês?" : "Excluir lançamento?"} description={transactionToDelete.recurringOccurrenceId ? "Este lançamento foi gerado por um fixo. Somente esta ocorrência mensal será removida; o fixo continuará ativo nos próximos meses." : `O lançamento “${transactionToDelete.description}” será excluído definitivamente.`} confirmLabel={transactionToDelete.recurringOccurrenceId ? "Remover deste mês" : "Excluir lançamento"} variant="danger" busy={deletingTransaction} onConfirm={() => void confirmTransactionRemoval()} onCancel={() => setTransactionToDelete(null)} />}
  </>;
}

function InstallmentDeleteModal({ transaction, onClose, onDeleted }: { transaction: Transaction; onClose: () => void; onDeleted: () => Promise<void> }) {
  const [deleting, setDeleting] = useState(false);
  const [error, setError] = useState("");

  async function remove(scope: "single" | "from_current" | "all") {
    setDeleting(true); setError("");
    try {
      await api.deleteInstallmentTransactions(transaction.id, scope);
      await onDeleted();
    }
    catch (err) { setError(errorMessage(err)); setDeleting(false); }
  }

  return <Modal title="Excluir compra parcelada" description="Escolha o alcance da exclusão antes de continuar." icon={<AlertTriangle size={20} />} onClose={() => { if (!deleting) onClose(); }}>
    {error && <Notice type="error">{error}</Notice>}
    <p>Esta compra é parcelada. O que deseja excluir?</p>
    <p className="form-hint">{transaction.description} · parcela {transaction.installmentNumber}/{transaction.installmentTotal}</p>
    <div className="installment-delete-options">
      <button className="secondary" disabled={deleting} onClick={() => void remove("single")}>Somente esta parcela</button>
      <button className="secondary" disabled={deleting} onClick={() => void remove("from_current")}>Esta e as próximas</button>
      <button className="danger-button" disabled={deleting} onClick={() => void remove("all")}>Toda a compra parcelada</button>
      <button className="ghost" disabled={deleting} onClick={onClose}>Cancelar</button>
    </div>
  </Modal>;
}

function TransactionForm({ transaction, fiscalMonth, accounts, categories, cards, onClose, onSaved }: { transaction: Transaction | null; fiscalMonth: string; accounts: Account[]; categories: Category[]; cards: Card[]; onClose: () => void; onSaved: () => Promise<void> }) {
  const [description, setDescription] = useState(transaction?.description ?? "");
  const [amount, setAmount] = useState(transaction ? moneyInput(transaction.amountCents) : "");
  const [type, setType] = useState<TransactionInput["type"]>(transaction?.type ?? "expense");
  const [status, setStatus] = useState<TransactionInput["status"]>(transaction?.status ?? "open");
  const [date, setDate] = useState(transaction?.transactionDate ?? today());
  const [month, setMonth] = useState(transaction?.fiscalMonth ?? fiscalMonth);
  const [source, setSource] = useState(transaction?.creditCardId ? `card:${transaction.creditCardId}` : `account:${transaction?.accountId ?? accounts.find((row) => row.active)?.id ?? ""}`);
  const [categoryId, setCategoryId] = useState(String(transaction?.categoryId ?? ""));
  const [notes, setNotes] = useState(transaction?.notes ?? "");
  const [error, setError] = useState(""); const [saving, setSaving] = useState(false);

  async function submit(event: React.FormEvent) {
    event.preventDefault(); const cents = parseMoney(amount);
    if (!description.trim()) return setError("Informe a descrição.");
    if (cents === null || cents <= 0) return setError("Informe um valor válido maior que zero.");
    const [sourceType, sourceId] = source.split(":");
    if (!sourceId) return setError("Escolha uma conta ou cartão.");
    if (type === "income" && sourceType === "card") return setError("Receitas devem entrar em uma conta.");
    const input: TransactionInput = { description: description.trim(), amountCents: cents, type, status, transactionDate: date, fiscalMonth: month, accountId: sourceType === "account" ? Number(sourceId) : null, creditCardId: sourceType === "card" ? Number(sourceId) : null, categoryId: categoryId ? Number(categoryId) : null, notes: notes.trim() || null };
    setSaving(true); setError("");
    try { transaction ? await api.updateTransaction(transaction.id, input) : await api.createTransaction(input); await onSaved(); }
    catch (err) { setError(errorMessage(err)); } finally { setSaving(false); }
  }

  const categoryOptions = categories.filter((row) => row.active && row.type === (type === "income" ? "income" : "expense"));
  return <Modal title={transaction ? "Editar lançamento" : "Novo lançamento"} description="Registre receitas, despesas e transferências do mês." icon={<ReceiptText size={20} />} onClose={onClose}>
    <form onSubmit={submit} className="form-grid">
      {error && <div className="notice notice-error full">{error}</div>}
      <label className="full">Descrição<input value={description} onChange={(e) => setDescription(e.target.value)} autoFocus /></label>
      <label>Valor (R$)<input value={amount} onChange={(e) => setAmount(e.target.value)} inputMode="decimal" placeholder="0,00" /></label>
      <label>Tipo<select value={type} onChange={(e) => { setType(e.target.value as TransactionInput["type"]); setCategoryId(""); }}><option value="expense">Despesa</option><option value="income">Receita</option><option value="transfer">Transferência</option></select></label>
      <label>Status<select value={status} onChange={(e) => setStatus(e.target.value as TransactionInput["status"])}><option value="open">Em aberto</option><option value="paid">Pago</option><option value="cancelled">Cancelado</option></select></label>
      <label>Data<input type="date" value={date} onChange={(e) => setDate(e.target.value)} /></label>
      <label>Mês fiscal<input type="month" value={month} onChange={(e) => setMonth(e.target.value)} /></label>
      <label>Conta ou cartão<select value={source} onChange={(e) => setSource(e.target.value)}><optgroup label="Contas">{accounts.filter((row) => row.active).map((row) => <option key={`a${row.id}`} value={`account:${row.id}`}>{row.name}</option>)}</optgroup>{type !== "income" && <optgroup label="Cartões">{cards.filter((row) => row.active).map((row) => <option key={`c${row.id}`} value={`card:${row.id}`}>{row.name}</option>)}</optgroup>}</select></label>
      <label>Categoria<select value={categoryId} onChange={(e) => setCategoryId(e.target.value)}><option value="">Sem categoria</option>{categoryOptions.map((row) => <option key={row.id} value={row.id}>{row.name}</option>)}</select></label>
      <label className="full">Observação<textarea value={notes} onChange={(e) => setNotes(e.target.value)} rows={3} /></label>
      <div className="form-actions full"><button type="button" className="ghost" onClick={onClose}>Cancelar</button><button className="primary" disabled={saving}>{saving ? "Salvando…" : "Salvar"}</button></div>
    </form>
  </Modal>;
}

function InstallmentForm({ fiscalMonth, categories, cards, onClose, onSaved }: { fiscalMonth: string; categories: Category[]; cards: Card[]; onClose: () => void; onSaved: () => Promise<void> }) {
  const [description, setDescription] = useState(""); const [amount, setAmount] = useState(""); const [installments, setInstallments] = useState(2);
  const [purchaseDate, setPurchaseDate] = useState(today()); const [firstMonth, setFirstMonth] = useState(fiscalMonth);
  const [cardId, setCardId] = useState(String(cards.find((row) => row.active)?.id ?? "")); const [categoryId, setCategoryId] = useState(""); const [notes, setNotes] = useState("");
  const [error, setError] = useState(""); const [saving, setSaving] = useState(false);
  async function submit(event: React.FormEvent) {
    event.preventDefault(); const cents = parseMoney(amount);
    if (!description.trim()) return setError("Informe a descrição.");
    if (cents === null || cents <= 0) return setError("Informe um valor válido maior que zero.");
    if (!cardId) return setError("Cadastre e selecione um cartão.");
    if (installments < 2 || installments > 120) return setError("Use entre 2 e 120 parcelas.");
    const input: InstallmentInput = { description: description.trim(), totalAmountCents: cents, installments, purchaseDate, firstFiscalMonth: firstMonth, creditCardId: Number(cardId), categoryId: categoryId ? Number(categoryId) : null, notes: notes.trim() || null };
    setSaving(true); setError("");
    try { await api.createInstallments(input); await onSaved(); } catch (err) { setError(errorMessage(err)); } finally { setSaving(false); }
  }
  return <Modal title="Nova compra parcelada" description="As parcelas serão distribuídas automaticamente nos próximos meses." icon={<CreditCard size={20} />} onClose={onClose}>
    <form onSubmit={submit} className="form-grid">
      {error && <div className="notice notice-error full">{error}</div>}
      <label className="full">Descrição<input value={description} onChange={(e) => setDescription(e.target.value)} autoFocus /></label>
      <label>Valor total (R$)<input value={amount} onChange={(e) => setAmount(e.target.value)} inputMode="decimal" placeholder="100,00" /></label>
      <label>Parcelas<input type="number" min="2" max="120" value={installments} onChange={(e) => setInstallments(Number(e.target.value))} /></label>
      <label>Data da compra<input type="date" value={purchaseDate} onChange={(e) => setPurchaseDate(e.target.value)} /></label>
      <label>Primeiro mês fiscal<input type="month" value={firstMonth} onChange={(e) => setFirstMonth(e.target.value)} /></label>
      <label>Cartão<select value={cardId} onChange={(e) => setCardId(e.target.value)}><option value="">Selecione</option>{cards.filter((row) => row.active).map((row) => <option key={row.id} value={row.id}>{row.name}</option>)}</select></label>
      <label>Categoria<select value={categoryId} onChange={(e) => setCategoryId(e.target.value)}><option value="">Sem categoria</option>{categories.filter((row) => row.active && row.type === "expense").map((row) => <option key={row.id} value={row.id}>{row.name}</option>)}</select></label>
      <label className="full">Observação<textarea value={notes} onChange={(e) => setNotes(e.target.value)} rows={3} /></label>
      <p className="form-hint full">Os centavos restantes são distribuídos nas primeiras parcelas. R$ 100,00 em 3x vira R$ 33,34 + R$ 33,33 + R$ 33,33.</p>
      <div className="form-actions full"><button type="button" className="ghost" onClick={onClose}>Cancelar</button><button className="primary" disabled={saving}>{saving ? "Criando…" : "Criar parcelas"}</button></div>
    </form>
  </Modal>;
}
