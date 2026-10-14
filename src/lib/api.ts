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
  InvoiceStatus,
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

  getPaths: () => invoke<AppPaths>("get_app_paths"),
  backup: () => invoke<string>("create_backup"),
  openDirectory: (kind: "database" | "backups") =>
    invoke<void>("open_directory", { kind }),
};

export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
