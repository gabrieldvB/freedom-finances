pub const SCHEMA: &str = r#"
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS accounts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    type TEXT NOT NULL CHECK(type IN ('cash','checking','savings','investment','other')),
    initial_balance_cents INTEGER NOT NULL DEFAULT 0,
    active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS categories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    type TEXT NOT NULL CHECK(type IN ('income','expense')),
    color TEXT NOT NULL DEFAULT '#55d69d',
    icon TEXT NOT NULL DEFAULT 'tag',
    active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(name, type)
);

CREATE TABLE IF NOT EXISTS credit_cards (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    closing_day INTEGER NOT NULL CHECK(closing_day BETWEEN 1 AND 31),
    due_day INTEGER NOT NULL CHECK(due_day BETWEEN 1 AND 31),
    limit_cents INTEGER NOT NULL DEFAULT 0 CHECK(limit_cents >= 0),
    active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS credit_card_invoices (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    credit_card_id INTEGER NOT NULL REFERENCES credit_cards(id) ON DELETE RESTRICT,
    fiscal_month TEXT NOT NULL,
    closing_date TEXT NOT NULL,
    due_date TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open' CHECK(status IN ('open','closed','paid','cancelled')),
    paid_at TEXT,
    notes TEXT,
    declared_amount_cents INTEGER NOT NULL DEFAULT 0,
    declared_at TEXT,
    declared_note TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(credit_card_id, fiscal_month)
);

CREATE TABLE IF NOT EXISTS transactions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    description TEXT NOT NULL,
    amount_cents INTEGER NOT NULL CHECK(amount_cents > 0),
    type TEXT NOT NULL CHECK(type IN ('income','expense','transfer')),
    status TEXT NOT NULL DEFAULT 'open' CHECK(status IN ('open','paid','cancelled')),
    transaction_date TEXT NOT NULL,
    fiscal_month TEXT NOT NULL,
    account_id INTEGER REFERENCES accounts(id) ON DELETE RESTRICT,
    credit_card_id INTEGER REFERENCES credit_cards(id) ON DELETE RESTRICT,
    invoice_id INTEGER REFERENCES credit_card_invoices(id) ON DELETE RESTRICT,
    category_id INTEGER REFERENCES categories(id) ON DELETE SET NULL,
    installment_group_id TEXT,
    installment_number INTEGER,
    installment_total INTEGER,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK(account_id IS NOT NULL OR credit_card_id IS NOT NULL),
    CHECK(credit_card_id IS NOT NULL OR invoice_id IS NULL)
);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS recurring_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    type TEXT NOT NULL CHECK(type IN ('income','expense')),
    category_id INTEGER REFERENCES categories(id) ON DELETE SET NULL,
    account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE RESTRICT,
    default_amount_cents INTEGER NOT NULL CHECK(default_amount_cents > 0),
    due_day INTEGER NOT NULL CHECK(due_day BETWEEN 1 AND 31),
    start_month TEXT NOT NULL,
    end_month TEXT,
    active INTEGER NOT NULL DEFAULT 1,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS recurring_occurrences (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    recurring_item_id INTEGER NOT NULL REFERENCES recurring_items(id) ON DELETE RESTRICT,
    transaction_id INTEGER NOT NULL UNIQUE REFERENCES transactions(id) ON DELETE RESTRICT,
    fiscal_month TEXT NOT NULL,
    amount_cents INTEGER NOT NULL CHECK(amount_cents > 0),
    due_date TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open' CHECK(status IN ('open','paid','cancelled')),
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(recurring_item_id, fiscal_month)
);

CREATE INDEX IF NOT EXISTS idx_transactions_fiscal_month ON transactions(fiscal_month);
CREATE INDEX IF NOT EXISTS idx_transactions_invoice ON transactions(invoice_id);
CREATE INDEX IF NOT EXISTS idx_invoices_card_month ON credit_card_invoices(credit_card_id, fiscal_month);
CREATE INDEX IF NOT EXISTS idx_recurring_items_active_months ON recurring_items(active, start_month, end_month);
CREATE INDEX IF NOT EXISTS idx_recurring_occurrences_month ON recurring_occurrences(fiscal_month);
"#;
