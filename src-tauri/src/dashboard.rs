use rusqlite::Connection;
use tauri::State;

use crate::{
    commands::{validate_month, CategoryTotalDto, DashboardDto},
    db::Database,
    invoices, recurring, settings, transactions, AppState,
};

#[tauri::command]
pub fn get_dashboard(
    state: State<'_, AppState>,
    fiscal_month: Option<String>,
) -> Result<DashboardDto, String> {
    get_dashboard_db(&state.db, fiscal_month.as_deref())
}

pub(crate) fn get_dashboard_db(
    db: &Database,
    fiscal_month: Option<&str>,
) -> Result<DashboardDto, String> {
    let connection = db.open().map_err(|e| e.to_string())?;
    let resolved_month = match fiscal_month {
        Some(value) => {
            validate_month(value)?;
            value.to_string()
        }
        None => settings::get_default_fiscal_month_db(&connection)?,
    };
    drop(connection);

    recurring::ensure_recurring_occurrences_for_month_db(db, &resolved_month)?;
    build_dashboard(&db.open().map_err(|e| e.to_string())?, &resolved_month)
}

fn sum(connection: &Connection, sql: &str, fiscal_month: &str) -> Result<i64, String> {
    connection
        .query_row(sql, [fiscal_month], |row| row.get(0))
        .map_err(|e| e.to_string())
}

fn sum_card_invoices_due_in_month(
    connection: &Connection,
    fiscal_month: &str,
) -> Result<i64, String> {
    let start = format!("{fiscal_month}-01");
    let end = format!("{}-01", transactions::add_months(fiscal_month, 1)?);
    connection
        .query_row(
            r#"
            SELECT COALESCE(SUM(
                i.declared_amount_cents + COALESCE((
                    SELECT SUM(t.amount_cents)
                    FROM transactions t
                    WHERE t.invoice_id=i.id AND t.status!='cancelled'
                ), 0)
            ), 0)
            FROM credit_card_invoices i
            WHERE i.status IN ('open','closed','paid')
              AND i.due_date>=?1 AND i.due_date<?2
            "#,
            (&start, &end),
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())
}

pub(crate) fn build_dashboard(
    connection: &Connection,
    fiscal_month: &str,
) -> Result<DashboardDto, String> {
    validate_month(fiscal_month)?;
    let projected_income_cents = sum(connection, "SELECT COALESCE(SUM(amount_cents),0) FROM transactions WHERE fiscal_month=?1 AND type='income' AND status!='cancelled'", fiscal_month)?;
    let projected_direct_expenses_cents = sum(connection, "SELECT COALESCE(SUM(amount_cents),0) FROM transactions WHERE fiscal_month=?1 AND type='expense' AND status!='cancelled' AND credit_card_id IS NULL AND invoice_id IS NULL", fiscal_month)?;
    let projected_card_invoices_cents = sum_card_invoices_due_in_month(connection, fiscal_month)?;
    let projected_balance_cents =
        projected_income_cents - projected_direct_expenses_cents - projected_card_invoices_cents;
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
        projected_income_cents,
        projected_direct_expenses_cents,
        projected_card_invoices_cents,
        projected_balance_cents,
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
    use crate::{
        commands::{DeclaredInvoiceInput, RecurringItemInput},
        db::Database,
        recurring, settings,
    };
    use uuid::Uuid;

    fn database() -> (std::path::PathBuf, Database, i64, i64) {
        let root =
            std::env::temp_dir().join(format!("freedom-finances-dashboard-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let db = Database::at(root.join("test.db")).unwrap();
        let connection = db.open().unwrap();
        let account_id = connection
            .query_row("SELECT id FROM accounts LIMIT 1", [], |row| row.get(0))
            .unwrap();
        connection.execute("INSERT INTO credit_cards(name, closing_day, due_day, limit_cents) VALUES ('Inter',20,15,500000)", []).unwrap();
        let card_id = connection.last_insert_rowid();
        drop(connection);
        (root, db, account_id, card_id)
    }

    fn declare_invoice(
        db: &Database,
        card_id: i64,
        amount: i64,
        status: &str,
    ) -> crate::commands::InvoiceDto {
        invoices::create_declared_invoice_db(
            db,
            DeclaredInvoiceInput {
                credit_card_id: card_id,
                fiscal_month: "2026-11".into(),
                declared_amount_cents: amount,
                due_date: "2026-11-15".into(),
                closing_date: None,
                status: status.into(),
                declared_note: None,
            },
        )
        .unwrap()
    }

    fn insert_account_transaction(db: &Database, account_id: i64, kind: &str, amount: i64) {
        db.open().unwrap().execute(
            "INSERT INTO transactions(description, amount_cents, type, status, transaction_date, fiscal_month, account_id) VALUES ('Teste', ?1, ?2, 'open', '2026-11-05', '2026-11', ?3)",
            (amount, kind, account_id),
        ).unwrap();
    }

    #[test]
    fn declared_invoice_without_transactions_is_in_dashboard_and_projection() {
        let (root, db, _, card_id) = database();

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
        assert_eq!(dashboard.projected_card_invoices_cents, 102_636);
        assert_eq!(dashboard.projected_balance_cents, -102_636);
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

    #[test]
    fn dashboard_uses_saved_default_when_month_is_omitted() {
        let (root, db, _, _) = database();
        let connection = db.open().unwrap();
        settings::set_default_fiscal_month_db(&connection, "2026-11").unwrap();
        drop(connection);
        let dashboard = get_dashboard_db(&db, None).unwrap();
        assert_eq!(dashboard.fiscal_month, "2026-11");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn dashboard_generates_fixed_income_for_each_selected_month() {
        let (root, db, account_id, _) = database();
        let income_category_id = db
            .open()
            .unwrap()
            .query_row(
                "SELECT id FROM categories WHERE type='income' LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        recurring::create_recurring_item_db(
            &db,
            RecurringItemInput {
                name: "Salário".into(),
                item_type: "income".into(),
                category_id: Some(income_category_id),
                account_id,
                default_amount_cents: 300_000,
                due_day: 5,
                start_month: "2026-11".into(),
                end_month: None,
                active: true,
                notes: None,
            },
        )
        .unwrap();

        for fiscal_month in ["2026-11", "2026-12", "2027-01"] {
            let dashboard = get_dashboard_db(&db, Some(fiscal_month)).unwrap();
            assert_eq!(
                dashboard.projected_income_cents, 300_000,
                "dashboard deveria incluir o salário em {fiscal_month}"
            );
        }

        let transaction_count: i64 = db
            .open()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM transactions", [], |row| row.get(0))
            .unwrap();
        assert_eq!(transaction_count, 3);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn projected_balance_uses_income_and_direct_expenses() {
        let (root, db, account_id, _) = database();
        insert_account_transaction(&db, account_id, "income", 300_000);
        insert_account_transaction(&db, account_id, "expense", 40_000);

        let dashboard = build_dashboard(&db.open().unwrap(), "2026-11").unwrap();
        assert_eq!(dashboard.projected_income_cents, 300_000);
        assert_eq!(dashboard.projected_direct_expenses_cents, 40_000);
        assert_eq!(dashboard.projected_card_invoices_cents, 0);
        assert_eq!(dashboard.projected_balance_cents, 260_000);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn declared_invoice_due_in_month_reduces_projected_balance() {
        let (root, db, _, card_id) = database();
        declare_invoice(&db, card_id, 102_636, "open");
        let dashboard = build_dashboard(&db.open().unwrap(), "2026-11").unwrap();
        assert_eq!(dashboard.projected_card_invoices_cents, 102_636);
        assert_eq!(dashboard.projected_balance_cents, -102_636);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn card_purchase_is_counted_only_inside_its_invoice() {
        let (root, db, account_id, card_id) = database();
        insert_account_transaction(&db, account_id, "income", 300_000);
        insert_account_transaction(&db, account_id, "expense", 40_000);
        let invoice = declare_invoice(&db, card_id, 102_636, "open");
        db.open().unwrap().execute(
            "INSERT INTO transactions(description, amount_cents, type, status, transaction_date, fiscal_month, credit_card_id, invoice_id) VALUES ('Mercado', 20000, 'expense', 'open', '2026-11-05', '2026-11', ?1, ?2)",
            (card_id, invoice.id),
        ).unwrap();

        let dashboard = build_dashboard(&db.open().unwrap(), "2026-11").unwrap();
        assert_eq!(dashboard.projected_income_cents, 300_000);
        assert_eq!(dashboard.projected_direct_expenses_cents, 40_000);
        assert_eq!(dashboard.projected_card_invoices_cents, 122_636);
        assert_eq!(dashboard.projected_balance_cents, 137_364);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn paid_invoice_remains_in_projected_month() {
        let (root, db, _, card_id) = database();
        declare_invoice(&db, card_id, 102_636, "paid");
        let dashboard = build_dashboard(&db.open().unwrap(), "2026-11").unwrap();
        assert_eq!(dashboard.projected_card_invoices_cents, 102_636);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cancelled_invoice_is_omitted_from_projected_month() {
        let (root, db, _, card_id) = database();
        let invoice = declare_invoice(&db, card_id, 102_636, "open");
        invoices::set_invoice_status_db(&db, invoice.id, "cancelled").unwrap();
        let dashboard = build_dashboard(&db.open().unwrap(), "2026-11").unwrap();
        assert_eq!(dashboard.projected_card_invoices_cents, 0);
        assert_eq!(dashboard.projected_balance_cents, 0);
        std::fs::remove_dir_all(root).unwrap();
    }
}
