use serde::{Deserialize, Serialize};
use tauri::State;

use crate::{db::Database, AppState};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountDto {
    pub id: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub account_type: String,
    pub initial_balance_cents: i64,
    pub active: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryDto {
    pub id: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub category_type: String,
    pub color: String,
    pub icon: String,
    pub active: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryInput {
    pub name: String,
    #[serde(rename = "type")]
    pub category_type: String,
    pub color: String,
    pub icon: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardDto {
    pub id: i64,
    pub name: String,
    pub closing_day: u32,
    pub due_day: u32,
    pub limit_cents: i64,
    pub active: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardInput {
    pub name: String,
    pub closing_day: u32,
    pub due_day: u32,
    pub limit_cents: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionDto {
    pub id: i64,
    pub description: String,
    pub amount_cents: i64,
    #[serde(rename = "type")]
    pub transaction_type: String,
    pub status: String,
    pub transaction_date: String,
    pub fiscal_month: String,
    pub account_id: Option<i64>,
    pub account_name: Option<String>,
    pub credit_card_id: Option<i64>,
    pub credit_card_name: Option<String>,
    pub invoice_id: Option<i64>,
    pub category_id: Option<i64>,
    pub category_name: Option<String>,
    pub installment_group_id: Option<String>,
    pub installment_number: Option<i64>,
    pub installment_total: Option<i64>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionInput {
    pub description: String,
    pub amount_cents: i64,
    #[serde(rename = "type")]
    pub transaction_type: String,
    pub status: String,
    pub transaction_date: String,
    pub fiscal_month: String,
    pub account_id: Option<i64>,
    pub credit_card_id: Option<i64>,
    pub category_id: Option<i64>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallmentInput {
    pub description: String,
    pub total_amount_cents: i64,
    pub installments: u32,
    pub purchase_date: String,
    pub first_fiscal_month: String,
    pub credit_card_id: i64,
    pub category_id: Option<i64>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceDto {
    pub id: i64,
    pub credit_card_id: i64,
    pub credit_card: String,
    pub fiscal_month: String,
    pub closing_date: String,
    pub due_date: String,
    pub status: String,
    pub paid_at: Option<String>,
    pub notes: Option<String>,
    pub declared_amount_cents: i64,
    pub declared_at: Option<String>,
    pub declared_note: Option<String>,
    pub transactions_total_cents: i64,
    pub total_cents: i64,
    pub transaction_count: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeclaredInvoiceInput {
    pub credit_card_id: i64,
    pub fiscal_month: String,
    pub declared_amount_cents: i64,
    pub due_date: String,
    pub closing_date: Option<String>,
    pub status: String,
    pub declared_note: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryTotalDto {
    pub category: String,
    pub color: String,
    pub total_cents: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardDto {
    pub fiscal_month: String,
    pub income_cents: i64,
    pub expense_cents: i64,
    pub balance_cents: i64,
    pub open_cents: i64,
    pub paid_cents: i64,
    pub open_invoices_total_cents: i64,
    pub open_invoices: Vec<InvoiceDto>,
    pub upcoming_invoices: Vec<InvoiceDto>,
    pub category_totals: Vec<CategoryTotalDto>,
    pub recent_transactions: Vec<TransactionDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppPathsDto {
    pub database: String,
    pub database_directory: String,
    pub backups: String,
}

pub fn validate_month(value: &str) -> Result<(), String> {
    chrono::NaiveDate::parse_from_str(&format!("{value}-01"), "%Y-%m-%d")
        .map(|_| ())
        .map_err(|_| "Mês fiscal inválido; use AAAA-MM.".to_string())
}

pub fn validate_date(value: &str) -> Result<(), String> {
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map(|_| ())
        .map_err(|_| "Data inválida; use AAAA-MM-DD.".to_string())
}

#[tauri::command]
pub fn list_accounts(state: State<'_, AppState>) -> Result<Vec<AccountDto>, String> {
    list_accounts_db(&state.db)
}

pub fn list_accounts_db(db: &Database) -> Result<Vec<AccountDto>, String> {
    let connection = db.open().map_err(|e| e.to_string())?;
    let mut statement = connection.prepare("SELECT id, name, type, initial_balance_cents, active FROM accounts ORDER BY active DESC, name").map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok(AccountDto {
                id: row.get(0)?,
                name: row.get(1)?,
                account_type: row.get(2)?,
                initial_balance_cents: row.get(3)?,
                active: row.get::<_, i64>(4)? != 0,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_account(
    state: State<'_, AppState>,
    name: String,
    account_type: String,
    initial_balance_cents: i64,
) -> Result<AccountDto, String> {
    if name.trim().is_empty() {
        return Err("Informe o nome da conta.".into());
    }
    if !["cash", "checking", "savings", "investment", "other"].contains(&account_type.as_str()) {
        return Err("Tipo de conta inválido.".into());
    }
    let connection = state.db.open().map_err(|e| e.to_string())?;
    connection
        .execute(
            "INSERT INTO accounts(name, type, initial_balance_cents) VALUES (?1, ?2, ?3)",
            (name.trim(), &account_type, initial_balance_cents),
        )
        .map_err(|e| e.to_string())?;
    Ok(AccountDto {
        id: connection.last_insert_rowid(),
        name: name.trim().to_string(),
        account_type,
        initial_balance_cents,
        active: true,
    })
}

#[tauri::command]
pub fn set_account_active(state: State<'_, AppState>, id: i64, active: bool) -> Result<(), String> {
    let connection = state.db.open().map_err(|e| e.to_string())?;
    let changed = connection
        .execute(
            "UPDATE accounts SET active=?1, updated_at=CURRENT_TIMESTAMP WHERE id=?2",
            (active, id),
        )
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        Err("Conta não encontrada.".into())
    } else {
        Ok(())
    }
}
