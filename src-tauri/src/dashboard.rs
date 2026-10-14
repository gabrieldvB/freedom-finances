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

pub(crate) fn build_dashboard(
    connection: &Connection,
    fiscal_month: &str,
) -> Result<DashboardDto, String> {
    let income_cents = sum(connection, "SELECT COALESCE(SUM(amount_cents),0) FROM transactions WHERE fiscal_month=?1 AND type='income' AND status!='cancelled'", fiscal_month)?;
    let transaction_expense_cents = sum(connection, "SELECT COALESCE(SUM(amount_cents),0) FROM transactions WHERE fiscal_month=?1 AND type='expense' AND status!='cancelled'", fiscal_month)?;
    let declared_expense_cents = sum(connection, "SELECT COALESCE(SUM(declared_amount_cents),0) FROM credit_card_invoices WHERE fiscal_month=?1 AND status!='cancelled'", fiscal_month)?;
    let expense_cents = transaction_expense_cents + declared_expense_cents;
    let transaction_open_cents = sum(connection, "SELECT COALESCE(SUM(amount_cents),0) FROM transactions WHERE fiscal_month=?1 AND type='expense' AND status='open'", fiscal_month)?;
    let declared_open_cents = sum(connection, "SELECT COALESCE(SUM(declared_amount_cents),0) FROM credit_card_invoices WHERE fiscal_month=?1 AND status IN ('open','closed')", fiscal_month)?;
    let open_cents = transaction_open_cents + declared_open_cents;
    let transaction_paid_cents = sum(connection, "SELECT COALESCE(SUM(amount_cents),0) FROM transactions WHERE fiscal_month=?1 AND type='expense' AND status='paid'", fiscal_month)?;
    let declared_paid_cents = sum(connection, "SELECT COALESCE(SUM(declared_amount_cents),0) FROM credit_card_invoices WHERE fiscal_month=?1 AND status='paid'", fiscal_month)?;
    let paid_cents = transaction_paid_cents + declared_paid_cents;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{commands::DeclaredInvoiceInput, db::Database};
    use uuid::Uuid;

    #[test]
    fn declared_invoice_without_transactions_is_in_dashboard_and_projection() {
        let root =
            std::env::temp_dir().join(format!("freedom-finances-dashboard-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let db = Database::at(root.join("test.db")).unwrap();
        let connection = db.open().unwrap();
        connection.execute("INSERT INTO credit_cards(name, closing_day, due_day, limit_cents) VALUES ('Inter',20,15,500000)", []).unwrap();
        let card_id = connection.last_insert_rowid();
        drop(connection);

        for (month, amount, due_date) in [
            ("2026-11", 102_636, "2026-11-15"),
            ("2026-12", 54_586, "2026-12-15"),
            ("2027-01", 29_087, "2027-01-15"),
        ] {
            invoices::create_declared_invoice_db(
                &db,
                DeclaredInvoiceInput {
                    credit_card_id: card_id,
                    fiscal_month: month.into(),
                    declared_amount_cents: amount,
                    due_date: due_date.into(),
                    closing_date: None,
                    status: "open".into(),
                    declared_note: None,
                },
            )
            .unwrap();
        }

        let dashboard = build_dashboard(&db.open().unwrap(), "2026-11").unwrap();
        assert_eq!(dashboard.expense_cents, 102_636);
        assert_eq!(dashboard.open_cents, 102_636);
        assert_eq!(dashboard.balance_cents, -102_636);
        assert_eq!(dashboard.open_invoices_total_cents, 102_636);
        assert_eq!(dashboard.open_invoices.len(), 1);
        assert_eq!(dashboard.open_invoices[0].transaction_count, 0);
        assert_eq!(dashboard.upcoming_invoices.len(), 2);
        assert_eq!(
            dashboard
                .upcoming_invoices
                .iter()
                .map(|invoice| invoice.total_cents)
                .sum::<i64>(),
            83_673
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
