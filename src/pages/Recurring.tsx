import { useCallback, useEffect, useMemo, useState } from "react";
import { AlertTriangle, CalendarOff, Check, ExternalLink, Pencil, Plus, Repeat, RotateCcw, Trash2, WandSparkles, XCircle } from "lucide-react";
import { MonthPicker } from "../components/MonthPicker";
import { EmptyState, Modal, Notice, PageHeader, Spinner, StatusBadge } from "../components/Ui";
import { ConfirmDialog } from "../components/ui/ConfirmDialog";
import { api, errorMessage } from "../lib/api";
import { formatDate, formatMonth } from "../lib/dates";
import { formatMoney, moneyInput, parseMoney } from "../lib/money";
import type { Account, Category, RecurringItem, RecurringItemInput, RecurringOccurrence, RecurringType, TransactionStatus } from "../types";

const FISCAL_MONTH_PATTERN = /^\d{4}-(0[1-9]|1[0-2])$/;
type RecurringConfirmAction = { kind: "end"; item: RecurringItem } | { kind: "delete"; item: RecurringItem } | { kind: "cancel"; occurrence: RecurringOccurrence };

export default function Recurring({ fiscalMonth, defaultFiscalMonth, onMonthChange, onOpenTransactions }: { fiscalMonth: string; defaultFiscalMonth: string; onMonthChange: (value: string) => void; onOpenTransactions: () => void }) {
  const [items, setItems] = useState<RecurringItem[]>([]);
  const [occurrences, setOccurrences] = useState<RecurringOccurrence[]>([]);
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [categories, setCategories] = useState<Category[]>([]);
  const [editingItem, setEditingItem] = useState<RecurringItem | "new" | null>(null);
  const [editingOccurrence, setEditingOccurrence] = useState<RecurringOccurrence | null>(null);
  const [deletingWithHistory, setDeletingWithHistory] = useState<RecurringItem | null>(null);
  const [confirmAction, setConfirmAction] = useState<RecurringConfirmAction | null>(null);
  const [confirmBusy, setConfirmBusy] = useState(false);
  const [showCancelled, setShowCancelled] = useState(false);
  const [showInactive, setShowInactive] = useState(false);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<{ type: "success" | "error"; text: string } | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [itemRows, occurrenceRows, accountRows, categoryRows] = await Promise.all([
        api.listRecurringItems(), api.ensureRecurringOccurrences(fiscalMonth), api.listAccounts(), api.listCategories(),
      ]);
      setItems(itemRows); setOccurrences(occurrenceRows); setAccounts(accountRows); setCategories(categoryRows);
    } catch (err) {
      setNotice({ type: "error", text: errorMessage(err) });
    } finally {
      setLoading(false);
    }
  }, [fiscalMonth]);

  useEffect(() => { void load(); }, [load]);

  const visibleOccurrences = useMemo(() => showCancelled ? occurrences : occurrences.filter((row) => row.status !== "cancelled"), [occurrences, showCancelled]);
  const visibleItems = useMemo(() => showInactive ? items : items.filter((item) => item.active), [items, showInactive]);
  const incomeRows = useMemo(() => visibleOccurrences.filter((row) => row.type === "income"), [visibleOccurrences]);
  const expenseRows = useMemo(() => visibleOccurrences.filter((row) => row.type === "expense"), [visibleOccurrences]);

  async function generate() {
    setBusy(true); setNotice(null);
    try {
      setOccurrences(await api.ensureRecurringOccurrences(fiscalMonth));
      setNotice({ type: "success", text: `Fixos de ${formatMonth(fiscalMonth)} gerados sem duplicação.` });
    } catch (err) {
      setNotice({ type: "error", text: errorMessage(err) });
    } finally {
      setBusy(false);
    }
  }

  async function confirmRecurringAction() {
    if (!confirmAction) return;
    setConfirmBusy(true);
    try {
      if (confirmAction.kind === "end") {
        await api.endRecurringItemFromMonth(confirmAction.item.id, fiscalMonth);
        setNotice({ type: "success", text: `Fixo encerrado a partir de ${formatMonth(fiscalMonth)}.` });
      } else if (confirmAction.kind === "delete") {
        await api.deleteRecurringItem(confirmAction.item.id);
        setNotice({ type: "success", text: "Fixo excluído definitivamente." });
      } else {
        await api.cancelRecurringOccurrence(confirmAction.occurrence.id);
      }
      setConfirmAction(null);
      await load();
    }
    catch (err) { setNotice({ type: "error", text: errorMessage(err) }); }
    finally { setConfirmBusy(false); }
  }

  async function changeStatus(row: RecurringOccurrence, status: TransactionStatus) {
    if (status === "cancelled") {
      setConfirmAction({ kind: "cancel", occurrence: row });
      return;
    }
    try {
      await api.setRecurringOccurrenceStatus(row.id, status);
      await load();
    }
    catch (err) { setNotice({ type: "error", text: errorMessage(err) }); }
  }

  return <>
    <PageHeader title="Fixos" subtitle="Receitas e contas recorrentes com valores definidos mês a mês." actions={<><MonthPicker value={fiscalMonth} onChange={onMonthChange} /><button className="secondary" disabled={busy} onClick={() => void generate()}><WandSparkles size={17} /> {busy ? "Gerando…" : "Gerar fixos do mês"}</button><button className="primary" onClick={() => setEditingItem("new")}><Plus size={17} /> Novo fixo</button></>} />
    {notice && <Notice type={notice.type}>{notice.text}</Notice>}
    {loading ? <Spinner /> : <>
      <div className="recurring-view-options">
        <label className="visibility-toggle"><input type="checkbox" checked={showCancelled} onChange={(event) => setShowCancelled(event.target.checked)} /> Mostrar canceladas</label>
      </div>
      <div className="recurring-grid">
        <OccurrenceSection title="Receitas" rows={incomeRows} onEdit={setEditingOccurrence} onStatus={changeStatus} onOpenTransactions={onOpenTransactions} />
        <OccurrenceSection title="Contas" rows={expenseRows} onEdit={setEditingOccurrence} onStatus={changeStatus} onOpenTransactions={onOpenTransactions} />
      </div>
      <section className="panel recurring-models">
        <div className="panel-title"><div><h2>Modelos cadastrados</h2><p>Alterar o valor padrão não modifica ocorrências já geradas.</p></div><label className="visibility-toggle"><input type="checkbox" checked={showInactive} onChange={(event) => setShowInactive(event.target.checked)} /> Mostrar desativados</label></div>
        {visibleItems.length ? <div className="recurring-model-list">{visibleItems.map((item) => {
          const ended = !item.active || Boolean(item.endMonth && item.endMonth < fiscalMonth);
          return <div key={item.id} className={ended ? "inactive" : ""}><span><strong>{item.name}</strong><small>{item.type === "income" ? "Receita" : "Despesa"} · dia {item.dueDay} · desde {formatMonth(item.startMonth)}{item.endMonth ? ` até ${formatMonth(item.endMonth)}` : ""}{!item.active ? " · desativado" : ""}</small></span><strong>{formatMoney(item.defaultAmountCents)}</strong><div className="row-actions"><button className="icon-button" title="Editar modelo" onClick={() => setEditingItem(item)}><Pencil size={16} /></button><button className="ghost compact-action" disabled={ended} onClick={() => setConfirmAction({ kind: "end", item })}><CalendarOff size={15} /> Encerrar a partir deste mês</button><button className="ghost compact-action" onClick={() => setConfirmAction({ kind: "delete", item })}><Trash2 size={15} /> Excluir se não tiver histórico</button><button className="danger-button compact-action" onClick={() => setDeletingWithHistory(item)}><Trash2 size={15} /> Excluir fixo e histórico</button></div></div>;
        })}</div> : <EmptyState icon={<Repeat size={24} />} title={showInactive ? "Nenhum fixo cadastrado" : "Nenhum modelo ativo"} description="Crie um fixo para projetar receitas ou contas recorrentes nos próximos meses." action={<button className="primary" onClick={() => setEditingItem("new")}><Plus size={16} /> Novo fixo</button>} />}
      </section>
    </>}
    {editingItem && <RecurringItemForm item={editingItem === "new" ? null : editingItem} defaultFiscalMonth={defaultFiscalMonth} accounts={accounts} categories={categories} onClose={() => setEditingItem(null)} onSaved={async () => { setEditingItem(null); await load(); }} />}
    {editingOccurrence && <OccurrenceForm occurrence={editingOccurrence} onClose={() => setEditingOccurrence(null)} onSaved={async () => { setEditingOccurrence(null); await load(); }} />}
    {deletingWithHistory && <DeleteRecurringHistoryModal item={deletingWithHistory} onClose={() => setDeletingWithHistory(null)} onDeleted={async () => { setDeletingWithHistory(null); setNotice({ type: "success", text: "Fixo e histórico excluídos definitivamente." }); await load(); }} />}
    {confirmAction && <ConfirmDialog title={confirmAction.kind === "end" ? `Encerrar a partir de ${formatMonth(fiscalMonth)}?` : confirmAction.kind === "delete" ? "Excluir fixo sem histórico?" : "Cancelar ocorrência deste mês?"} description={confirmAction.kind === "end" ? "O histórico anterior será mantido e ocorrências futuras abertas serão canceladas." : confirmAction.kind === "delete" ? "Esta ação só funciona se o modelo ainda não possuir ocorrências vinculadas." : "O lançamento vinculado também será cancelado, sem alterar o modelo fixo."} confirmLabel={confirmAction.kind === "end" ? "Encerrar fixo" : confirmAction.kind === "delete" ? "Excluir fixo" : "Cancelar ocorrência"} variant={confirmAction.kind === "end" ? "warning" : "danger"} busy={confirmBusy} onConfirm={() => void confirmRecurringAction()} onCancel={() => setConfirmAction(null)} />}
  </>;
}

function DeleteRecurringHistoryModal({ item, onClose, onDeleted }: { item: RecurringItem; onClose: () => void; onDeleted: () => Promise<void> }) {
  const [confirmation, setConfirmation] = useState("");
  const [error, setError] = useState("");
  const [deleting, setDeleting] = useState(false);
  const nameMatches = confirmation === item.name;

  async function submit(event: React.FormEvent) {
    event.preventDefault();
    if (!nameMatches) return setError(`Digite exatamente “${item.name}” para confirmar.`);
    setDeleting(true); setError("");
    try {
      await api.deleteRecurringItemWithHistory(item.id);
      await onDeleted();
    }
    catch (err) { setError(errorMessage(err)); setDeleting(false); }
  }

  return <Modal title="Excluir fixo e histórico" description="Confirmação avançada para remoção permanente." icon={<AlertTriangle size={20} />} onClose={() => { if (!deleting) onClose(); }}>
    <form className="form-grid" onSubmit={submit}>
      {error && <div className="notice notice-error full">{error}</div>}
      <div className="destructive-warning full">Isso apagará o fixo, todas as ocorrências geradas e todos os lançamentos vinculados. Essa ação não pode ser desfeita.</div>
      <label className="full">Digite <strong>“{item.name}”</strong> para confirmar<input value={confirmation} onChange={(event) => setConfirmation(event.target.value)} autoFocus autoComplete="off" /></label>
      <div className="form-actions full"><button type="button" className="ghost" onClick={onClose} disabled={deleting}>Cancelar</button><button type="submit" className="danger-button" disabled={!nameMatches || deleting}>{deleting ? "Excluindo…" : "Excluir fixo e histórico"}</button></div>
    </form>
  </Modal>;
}

function OccurrenceSection({ title, rows, onEdit, onStatus, onOpenTransactions }: { title: string; rows: RecurringOccurrence[]; onEdit: (row: RecurringOccurrence) => void; onStatus: (row: RecurringOccurrence, status: TransactionStatus) => Promise<void>; onOpenTransactions: () => void }) {
  return <section className="panel"><div className="panel-title"><h2>{title}</h2><span>{rows.length} item(ns)</span></div>{rows.length ? <div className="recurring-occurrences">{rows.map((row) => <div key={row.id} className={row.status === "cancelled" ? "inactive" : ""}>
    <span><strong>{row.recurringItem}</strong><small>{row.categoryName ?? "Sem categoria"} · {row.accountName}</small></span>
    <span><small>Previsão</small><strong>{formatDate(row.dueDate)}</strong></span>
    <strong className={row.type === "income" ? "amount-income" : "amount-expense"}>{formatMoney(row.amountCents)}</strong>
    <StatusBadge status={row.status} />
    <div className="row-actions"><button className="icon-button" title="Editar valor do mês" onClick={() => onEdit(row)}><Pencil size={16} /></button>{row.status !== "paid" && <button className="icon-button success" title={row.type === "income" ? "Marcar como recebido" : "Marcar como pago"} onClick={() => void onStatus(row, "paid")}><Check size={16} /></button>}{row.status !== "open" && <button className="icon-button warning" title="Reabrir" onClick={() => void onStatus(row, "open")}><RotateCcw size={16} /></button>}{row.status !== "cancelled" && <button className="icon-button danger" title="Cancelar ocorrência" onClick={() => void onStatus(row, "cancelled")}><XCircle size={16} /></button>}<button className="icon-button" title="Abrir lançamentos" onClick={onOpenTransactions}><ExternalLink size={16} /></button></div>
  </div>)}</div> : <EmptyState icon={<Repeat size={22} />} title={`Nenhum fixo em ${title.toLowerCase()}`} description="Crie um modelo para projetar valores recorrentes neste mês." />}</section>;
}

function RecurringItemForm({ item, defaultFiscalMonth, accounts, categories, onClose, onSaved }: { item: RecurringItem | null; defaultFiscalMonth: string; accounts: Account[]; categories: Category[]; onClose: () => void; onSaved: () => Promise<void> }) {
  const [name, setName] = useState(item?.name ?? "");
  const [type, setType] = useState<RecurringType>(item?.type ?? "expense");
  const [categoryId, setCategoryId] = useState(String(item?.categoryId ?? ""));
  const [accountId, setAccountId] = useState(String(item?.accountId ?? accounts.find((row) => row.active)?.id ?? ""));
  const [amount, setAmount] = useState(item ? moneyInput(item.defaultAmountCents) : "");
  const [dueDay, setDueDay] = useState(item?.dueDay ?? 10);
  const [startMonth, setStartMonth] = useState(item?.startMonth ?? defaultFiscalMonth);
  const [endMonth, setEndMonth] = useState(item?.endMonth ?? "");
  const [active, setActive] = useState(item?.active ?? true);
  const [notes, setNotes] = useState(item?.notes ?? "");
  const [error, setError] = useState(""); const [saving, setSaving] = useState(false);

  async function submit(event: React.FormEvent) {
    event.preventDefault(); const cents = parseMoney(amount);
    if (!name.trim()) return setError("Informe o nome.");
    if (cents === null || cents <= 0) return setError("Informe um valor padrão maior que zero.");
    if (!accountId) return setError("Selecione uma conta.");
    if (dueDay < 1 || dueDay > 31) return setError("O dia previsto deve ficar entre 1 e 31.");
    if (!startMonth) return setError("Informe o mês inicial.");
    if (!FISCAL_MONTH_PATTERN.test(startMonth)) return setError("Mês inicial inválido; use AAAA-MM.");
    if (endMonth && !FISCAL_MONTH_PATTERN.test(endMonth)) return setError("Mês final inválido; use AAAA-MM.");
    if (endMonth && endMonth < startMonth) return setError("O mês final não pode ser anterior ao mês inicial.");
    const input: RecurringItemInput & { id?: number } = { id: item?.id, name: name.trim(), type, categoryId: categoryId ? Number(categoryId) : null, accountId: Number(accountId), defaultAmountCents: cents, dueDay, startMonth, endMonth: endMonth || null, active, notes: notes.trim() || null };
    setSaving(true); setError("");
    try { await api.saveRecurringItem(input); await onSaved(); }
    catch (err) { setError(errorMessage(err)); } finally { setSaving(false); }
  }

  const categoryOptions = categories.filter((row) => row.type === type && (row.active || row.id === item?.categoryId));
  return <Modal title={item ? "Editar fixo" : "Novo fixo"} description="Configure uma receita ou despesa que se repete mensalmente." icon={<Repeat size={20} />} onClose={onClose}><form className="form-grid" onSubmit={submit} noValidate>
    {error && <div className="notice notice-error full">{error}</div>}
    <label className="full">Nome<input value={name} onChange={(event) => setName(event.target.value)} autoFocus /></label>
    <label>Tipo<select value={type} onChange={(event) => { setType(event.target.value as RecurringType); setCategoryId(""); }}><option value="income">Receita</option><option value="expense">Despesa</option></select></label>
    <label>Valor padrão (R$)<input value={amount} onChange={(event) => setAmount(event.target.value)} inputMode="decimal" placeholder="0,00" /></label>
    <label>Conta<select value={accountId} onChange={(event) => setAccountId(event.target.value)}><option value="">Selecione</option>{accounts.filter((row) => row.active || row.id === item?.accountId).map((row) => <option key={row.id} value={row.id}>{row.name}</option>)}</select></label>
    <label>Categoria<select value={categoryId} onChange={(event) => setCategoryId(event.target.value)}><option value="">Sem categoria</option>{categoryOptions.map((row) => <option key={row.id} value={row.id}>{row.name}</option>)}</select></label>
    <label>Dia previsto<input type="number" min="1" max="31" value={dueDay} onChange={(event) => setDueDay(Number(event.target.value))} /></label>
    <label>Mês inicial<input type="month" value={startMonth} onChange={(event) => setStartMonth(event.target.value)} required /></label>
    <label>Mês final (opcional)<input type="month" min={startMonth || undefined} value={endMonth} onChange={(event) => setEndMonth(event.target.value)} /></label>
    <p className="form-hint full">Deixe o mês final vazio para repetir indefinidamente.</p>
    <label className="checkbox-label"><input type="checkbox" checked={active} onChange={(event) => setActive(event.target.checked)} /> Ativo</label>
    <label className="full">Observação<textarea rows={3} value={notes} onChange={(event) => setNotes(event.target.value)} /></label>
    <div className="form-actions full"><button type="button" className="ghost" onClick={onClose}>Cancelar</button><button className="primary" disabled={saving}>{saving ? "Salvando…" : "Salvar fixo"}</button></div>
  </form></Modal>;
}

function OccurrenceForm({ occurrence, onClose, onSaved }: { occurrence: RecurringOccurrence; onClose: () => void; onSaved: () => Promise<void> }) {
  const [amount, setAmount] = useState(moneyInput(occurrence.amountCents));
  const [notes, setNotes] = useState(occurrence.notes ?? "");
  const [error, setError] = useState(""); const [saving, setSaving] = useState(false);
  async function submit(event: React.FormEvent) {
    event.preventDefault(); const cents = parseMoney(amount);
    if (cents === null || cents <= 0) return setError("Informe um valor maior que zero.");
    setSaving(true); setError("");
    try { await api.updateRecurringOccurrence(occurrence.id, cents, notes.trim() || null); await onSaved(); }
    catch (err) { setError(errorMessage(err)); } finally { setSaving(false); }
  }
  return <Modal title={`Editar ${occurrence.recurringItem} em ${formatMonth(occurrence.fiscalMonth)}`} description="A alteração será aplicada somente à ocorrência selecionada." icon={<Pencil size={20} />} onClose={onClose}><form className="form-grid" onSubmit={submit}>{error && <div className="notice notice-error full">{error}</div>}<label className="full">Valor do mês (R$)<input value={amount} onChange={(event) => setAmount(event.target.value)} inputMode="decimal" autoFocus /></label><label className="full">Observação do mês<textarea rows={3} value={notes} onChange={(event) => setNotes(event.target.value)} /></label><p className="form-hint full">Esta alteração vale apenas para este mês e não muda o valor padrão do modelo.</p><div className="form-actions full"><button type="button" className="ghost" onClick={onClose}>Cancelar</button><button className="primary" disabled={saving}>{saving ? "Salvando…" : "Salvar valor"}</button></div></form></Modal>;
}
