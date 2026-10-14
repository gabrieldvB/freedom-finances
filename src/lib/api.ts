import { invoke } from "@tauri-apps/api/core";
import type {
  Account,
  AppPaths,
  Category,
  CreditCard,
  DashboardData,
  DeclaredInvoiceInput,
  InstallmentInput,
  Invoice,
  InvoiceGroups,
  InvoiceStatus,
  RecurringItem,
  RecurringItemInput,
  RecurringOccurrence,
  Transaction,
  TransactionInput,
  TransactionStatus,
} from "../types";

export const api = {
  dashboard: (fiscalMonth?: string) =>
    invoke<DashboardData>("get_dashboard", { fiscalMonth: fiscalMonth ?? null }),

  getSetting: (key: string) => invoke<string | null>("get_setting", { key }),
  setSetting: (key: string, value: string) =>
    invoke<void>("set_setting", { key, value }),
  getDefaultFiscalMonth: () => invoke<string>("get_default_fiscal_month"),
  setDefaultFiscalMonth: (fiscalMonth: string) =>
    invoke<string>("set_default_fiscal_month", { fiscalMonth }),

  listTransactions: (fiscalMonth: string) =>
    invoke<Transaction[]>("list_transactions", { fiscalMonth }),
  createTransaction: (input: TransactionInput) =>
    invoke<Transaction>("create_transaction", { input }),
  updateTransaction: (id: number, input: TransactionInput) =>
    invoke<Transaction>("update_transaction", { id, input }),
  setTransactionStatus: (id: number, status: TransactionStatus) =>
    invoke<void>("set_transaction_status", { id, status }),
  deleteTransaction: (id: number) => invoke<void>("delete_transaction", { id }),
  createInstallments: (input: InstallmentInput) =>
    invoke<Transaction[]>("create_installments", { input }),

  listAccounts: () => invoke<Account[]>("list_accounts"),
  createAccount: (name: string, accountType: string, initialBalanceCents: number) =>
    invoke<Account>("create_account", { name, accountType, initialBalanceCents }),
  setAccountActive: (id: number, active: boolean) =>
    invoke<void>("set_account_active", { id, active }),

  listCategories: () => invoke<Category[]>("list_categories"),
  saveCategory: (category: Partial<Category> & Pick<Category, "name" | "type" | "color" | "icon">) =>
    invoke<Category>(category.id ? "update_category" : "create_category", category.id
      ? { id: category.id, input: category }
      : { input: category }),
  setCategoryActive: (id: number, active: boolean) =>
    invoke<void>("set_category_active", { id, active }),

  listCards: () => invoke<CreditCard[]>("list_cards"),
  saveCard: (card: Partial<CreditCard> & Pick<CreditCard, "name" | "closingDay" | "dueDay" | "limitCents">) =>
    invoke<CreditCard>(card.id ? "update_card" : "create_card", card.id
      ? { id: card.id, input: card }
      : { input: card }),
  setCardActive: (id: number, active: boolean) =>
    invoke<void>("set_card_active", { id, active }),

  listInvoices: (creditCardId?: number) =>
    invoke<Invoice[]>("list_invoices", { creditCardId: creditCardId ?? null }),
  getInvoiceGroups: (fiscalMonth?: string) =>
    invoke<InvoiceGroups>("get_invoice_groups", { fiscalMonth: fiscalMonth ?? null }),
  createDeclaredInvoice: (input: DeclaredInvoiceInput) =>
    invoke<Invoice>("create_declared_invoice", { input }),
  updateInvoiceDeclaredAmount: (
    id: number,
    declaredAmountCents: number,
    declaredNote: string | null,
  ) => invoke<Invoice>("update_invoice_declared_amount", {
    id,
    declaredAmountCents,
    declaredNote,
  }),
  getInvoice: (id: number) => invoke<Invoice>("get_invoice", { id }),
  getInvoiceTransactions: (id: number) =>
    invoke<Transaction[]>("get_invoice_transactions", { id }),
  setInvoiceStatus: (id: number, status: InvoiceStatus) =>
    invoke<Invoice>("set_invoice_status", { id, status }),

  listRecurringItems: () => invoke<RecurringItem[]>("list_recurring_items"),
  saveRecurringItem: (item: RecurringItemInput & { id?: number }) =>
    invoke<RecurringItem>(item.id ? "update_recurring_item" : "create_recurring_item", item.id
      ? { id: item.id, input: item }
      : { input: item }),
  setRecurringItemActive: (id: number, active: boolean) =>
    invoke<RecurringItem>("set_recurring_item_active", { id, active }),
  deactivateRecurringItem: (id: number) =>
    invoke<RecurringItem>("deactivate_recurring_item", { id }),
  deleteRecurringItem: (id: number) =>
    invoke<void>("delete_recurring_item", { id }),
  deleteRecurringItemWithHistory: (recurringItemId: number) =>
    invoke<void>("delete_recurring_item_with_history", { recurringItemId }),
  endRecurringItemFromMonth: (recurringItemId: number, fiscalMonth: string) =>
    invoke<RecurringItem>("end_recurring_item_from_month", { recurringItemId, fiscalMonth }),
  listRecurringOccurrences: (fiscalMonth: string) =>
    invoke<RecurringOccurrence[]>("list_recurring_occurrences", { fiscalMonth }),
  ensureRecurringOccurrences: (fiscalMonth: string) =>
    invoke<RecurringOccurrence[]>("ensure_recurring_occurrences_for_month", { fiscalMonth }),
  updateRecurringOccurrence: (id: number, amountCents: number, notes: string | null) =>
    invoke<RecurringOccurrence>("update_recurring_occurrence", { id, amountCents, notes }),
  setRecurringOccurrenceStatus: (id: number, status: TransactionStatus) =>
    invoke<RecurringOccurrence>("set_recurring_occurrence_status", { id, status }),
  cancelRecurringOccurrence: (id: number) =>
    invoke<RecurringOccurrence>("cancel_recurring_occurrence", { id }),

  getInitialGlobalBalance: () => invoke<number>("get_initial_global_balance"),
  setInitialGlobalBalance: (balanceCents: number) =>
    invoke<number>("set_initial_global_balance", { balanceCents }),

  getPaths: () => invoke<AppPaths>("get_app_paths"),
  backup: () => invoke<string>("create_backup"),
  openDirectory: (kind: "database" | "backups") =>
    invoke<void>("open_directory", { kind }),
};

export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
