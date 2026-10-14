export type TransactionType = "income" | "expense" | "transfer";
export type TransactionStatus = "open" | "paid" | "cancelled";
export type InvoiceStatus = "open" | "closed" | "paid" | "cancelled";
export type CategoryType = "income" | "expense";

export interface Account {
  id: number;
  name: string;
  type: string;
  initialBalanceCents: number;
  active: boolean;
}

export interface Category {
  id: number;
  name: string;
  type: CategoryType;
  color: string;
  icon: string;
  active: boolean;
}

export interface CreditCard {
  id: number;
  name: string;
  closingDay: number;
  dueDay: number;
  limitCents: number;
  active: boolean;
}

export interface Transaction {
  id: number;
  description: string;
  amountCents: number;
  type: TransactionType;
  status: TransactionStatus;
  transactionDate: string;
  fiscalMonth: string;
  accountId: number | null;
  accountName: string | null;
  creditCardId: number | null;
  creditCardName: string | null;
  invoiceId: number | null;
  categoryId: number | null;
  categoryName: string | null;
  installmentGroupId: string | null;
  installmentNumber: number | null;
  installmentTotal: number | null;
  notes: string | null;
  recurringOccurrenceId: number | null;
}

export interface Invoice {
  id: number;
  creditCardId: number;
  creditCard: string;
  fiscalMonth: string;
  closingDate: string;
  dueDate: string;
  status: InvoiceStatus;
  paidAt: string | null;
  notes: string | null;
  declaredAmountCents: number;
  declaredAt: string | null;
  declaredNote: string | null;
  transactionsTotalCents: number;
  totalCents: number;
  transactionCount: number;
}

export interface DeclaredInvoiceInput {
  creditCardId: number;
  fiscalMonth: string;
  declaredAmountCents: number;
  dueDate: string;
  closingDate: string | null;
  status: Exclude<InvoiceStatus, "cancelled">;
  declaredNote: string | null;
}

export interface InvoiceGroups {
  fiscalMonth: string;
  currentInvoices: Invoice[];
  openInvoices: Invoice[];
  historyInvoices: Invoice[];
}

export type RecurringType = "income" | "expense";

export interface RecurringItem {
  id: number;
  name: string;
  type: RecurringType;
  categoryId: number | null;
  categoryName: string | null;
  accountId: number;
  accountName: string;
  defaultAmountCents: number;
  dueDay: number;
  startMonth: string;
  endMonth: string | null;
  active: boolean;
  notes: string | null;
}

export interface RecurringItemInput {
  name: string;
  type: RecurringType;
  categoryId: number | null;
  accountId: number;
  defaultAmountCents: number;
  dueDay: number;
  startMonth: string;
  endMonth: string | null;
  active: boolean;
  notes: string | null;
}

export interface RecurringOccurrence {
  id: number;
  recurringItemId: number;
  recurringItem: string;
  type: RecurringType;
  categoryId: number | null;
  categoryName: string | null;
  accountId: number;
  accountName: string;
  transactionId: number;
  fiscalMonth: string;
  amountCents: number;
  dueDate: string;
  status: TransactionStatus;
  notes: string | null;
}

export interface CategoryTotal {
  category: string;
  color: string;
  totalCents: number;
}

export interface DashboardData {
  fiscalMonth: string;
  projectedIncomeCents: number;
  projectedDirectExpensesCents: number;
  projectedCardInvoicesCents: number;
  projectedBalanceCents: number;
  incomeCents: number;
  expenseCents: number;
  balanceCents: number;
  openCents: number;
  paidCents: number;
  receivedCents: number;
  openingBalanceCents: number;
  realizedBalanceCents: number;
  projectedAccumulatedBalanceCents: number;
  openInvoicesTotalCents: number;
  openInvoices: Invoice[];
  upcomingInvoices: Invoice[];
  categoryTotals: CategoryTotal[];
  recentTransactions: Transaction[];
}

export interface MonthlyBalance {
  fiscalMonth: string;
  openingBalanceCents: number;
  notes: string | null;
}

export interface AppPaths {
  database: string;
  databaseDirectory: string;
  backups: string;
}

export interface TransactionInput {
  description: string;
  amountCents: number;
  type: TransactionType;
  status: TransactionStatus;
  transactionDate: string;
  fiscalMonth: string;
  accountId: number | null;
  creditCardId: number | null;
  categoryId: number | null;
  notes: string | null;
}

export interface InstallmentInput {
  description: string;
  totalAmountCents: number;
  installments: number;
  purchaseDate: string;
  firstFiscalMonth: string;
  creditCardId: number;
  categoryId: number | null;
  notes: string | null;
}
