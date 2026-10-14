use rusqlite::Connection;
use tauri::State;

use crate::{
    commands::{validate_month, CategoryTotalDto, DashboardDto},
    invoices, transactions, AppState,
};

#[tauri::command]
pub fn get_dashboard(
    state: State<'_, AppState>,
    fiscal_month: String,
) -> Result<DashboardDto, String> {
    validate_month(&fiscal_month)?;
    let connection = state.db.open().map_err(|e| e.to_string())?;
    build_dashboard(&connection, &fiscal_month)
}

fn sum(connection: &Connection, sql: &str, fiscal_month: &str) -> Result<i64, String> {
    connection
        .query_row(sql, [fiscal_month], |row| row.get(0))
        .map_err(|e| e.to_string())
}

fn build_dashboard(connection: &Connection, fiscal_month: &str) -> Result<DashboardDto, String> {
    let income_cents = sum(connection, "SELECT COALESCE(SUM(amount_cents),0) FROM transactions WHERE fiscal_month=?1 AND type='income' AND status!='cancelled'", fiscal_month)?;
    let expense_cents = sum(connection, "SELECT COALESCE(SUM(amount_cents),0) FROM transactions WHERE fiscal_month=?1 AND type='expense' AND status!='cancelled'", fiscal_month)?;
    let open_cents = sum(connection, "SELECT COALESCE(SUM(amount_cents),0) FROM transactions WHERE fiscal_month=?1 AND type='expense' AND status='open'", fiscal_month)?;
    let paid_cents = sum(connection, "SELECT COALESCE(SUM(amount_cents),0) FROM transactions WHERE fiscal_month=?1 AND type='expense' AND status='paid'", fiscal_month)?;

    let mut category_statement = connection
        .prepare(
            r#"
        SELECT COALESCE(c.name, 'Sem categoria'), COALESCE(c.color, '#91a69e'), SUM(t.amount_cents)
        FROM transactions t LEFT JOIN categories c ON c.id=t.category_id
        WHERE t.fiscal_month=?1 AND t.type='expense' AND t.status!='cancelled'
        GROUP BY c.id, c.name, c.color ORDER BY SUM(t.amount_cents) DESC
    "#,
        )
        .map_err(|e| e.to_string())?;
    let category_totals = category_statement
        .query_map([fiscal_month], |row| {
            Ok(CategoryTotalDto {
                category: row.get(0)?,
                color: row.get(1)?,
                total_cents: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let all_invoices = invoices::list_invoices_db(connection, None)?;
    let open_invoices: Vec<_> = all_invoices
        .iter()
        .filter(|item| {
            item.fiscal_month == fiscal_month && ["open", "closed"].contains(&item.status.as_str())
        })
        .cloned()
        .collect();
    let mut upcoming_invoices: Vec<_> = all_invoices
        .iter()
        .filter(|item| {
            item.fiscal_month.as_str() > fiscal_month
                && ["open", "closed"].contains(&item.status.as_str())
        })
        .cloned()
        .collect();
    upcoming_invoices.sort_by(|left, right| left.fiscal_month.cmp(&right.fiscal_month));
    upcoming_invoices.truncate(8);
    let open_invoices_total_cents = open_invoices.iter().map(|item| item.total_cents).sum();
    let mut recent_transactions = transactions::list_transactions_db(connection, fiscal_month)?;
    recent_transactions.truncate(8);

    Ok(DashboardDto {
        fiscal_month: fiscal_month.into(),
        income_cents,
        expense_cents,
        balance_cents: income_cents - expense_cents,
        open_cents,
        paid_cents,
        open_invoices_total_cents,
        open_invoices,
        upcoming_invoices,
        category_totals,
        recent_transactions,
    })
}
