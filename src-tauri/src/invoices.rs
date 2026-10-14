use chrono::{Datelike, Local, NaiveDate};
use rusqlite::{params, Connection, OptionalExtension, Row};
use tauri::State;

use crate::{
    commands::{validate_month, InvoiceDto},
    db::Database,
    transactions, AppState,
};

fn days_in_month(year: i32, month: u32) -> u32 {
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    (NaiveDate::from_ymd_opt(next_year, next_month, 1).unwrap() - chrono::Duration::days(1)).day()
}

fn date_with_day(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day.min(days_in_month(year, month))).unwrap()
}

pub fn invoice_dates(
    fiscal_month: &str,
    closing_day: u32,
    due_day: u32,
) -> Result<(String, String), String> {
    validate_month(fiscal_month)?;
    let base = NaiveDate::parse_from_str(&format!("{fiscal_month}-01"), "%Y-%m-%d")
        .map_err(|e| e.to_string())?;
    let closing = date_with_day(base.year(), base.month(), closing_day);
    let (due_year, due_month) = if base.month() == 12 {
        (base.year() + 1, 1)
    } else {
        (base.year(), base.month() + 1)
    };
    let due = date_with_day(due_year, due_month, due_day);
    Ok((
        closing.format("%Y-%m-%d").to_string(),
        due.format("%Y-%m-%d").to_string(),
    ))
}

pub fn find_or_create_invoice(
    connection: &Connection,
    credit_card_id: i64,
    fiscal_month: &str,
) -> Result<i64, String> {
    validate_month(fiscal_month)?;
    if let Some(id) = connection
        .query_row(
            "SELECT id FROM credit_card_invoices WHERE credit_card_id=?1 AND fiscal_month=?2",
            (credit_card_id, fiscal_month),
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
    {
        return Ok(id);
    }
    let (closing_day, due_day): (u32, u32) = connection
        .query_row(
            "SELECT closing_day, due_day FROM credit_cards WHERE id=?1 AND active=1",
            [credit_card_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| "Cartão não encontrado ou inativo.".to_string())?;
    let (closing_date, due_date) = invoice_dates(fiscal_month, closing_day, due_day)?;
    connection.execute("INSERT INTO credit_card_invoices(credit_card_id, fiscal_month, closing_date, due_date, status) VALUES (?1, ?2, ?3, ?4, 'open')", params![credit_card_id, fiscal_month, closing_date, due_date]).map_err(|e| e.to_string())?;
    Ok(connection.last_insert_rowid())
}

const INVOICE_SELECT: &str = r#"
SELECT i.id, i.credit_card_id, c.name, i.fiscal_month, i.closing_date, i.due_date,
       i.status, i.paid_at, i.notes,
       COALESCE(SUM(CASE WHEN t.status != 'cancelled' THEN t.amount_cents ELSE 0 END), 0) AS total_cents,
       SUM(CASE WHEN t.status != 'cancelled' THEN 1 ELSE 0 END) AS transaction_count
FROM credit_card_invoices i
JOIN credit_cards c ON c.id=i.credit_card_id
LEFT JOIN transactions t ON t.invoice_id=i.id
"#;

fn map_invoice(row: &Row<'_>) -> rusqlite::Result<InvoiceDto> {
    Ok(InvoiceDto {
        id: row.get(0)?,
        credit_card_id: row.get(1)?,
        credit_card: row.get(2)?,
        fiscal_month: row.get(3)?,
        closing_date: row.get(4)?,
        due_date: row.get(5)?,
        status: row.get(6)?,
        paid_at: row.get(7)?,
        notes: row.get(8)?,
        total_cents: row.get(9)?,
        transaction_count: row.get(10)?,
    })
}

#[tauri::command]
pub fn list_invoices(
    state: State<'_, AppState>,
    credit_card_id: Option<i64>,
) -> Result<Vec<InvoiceDto>, String> {
    list_invoices_db(&state.db.open().map_err(|e| e.to_string())?, credit_card_id)
}

pub fn list_invoices_db(
    connection: &Connection,
    credit_card_id: Option<i64>,
) -> Result<Vec<InvoiceDto>, String> {
    let sql = format!("{INVOICE_SELECT} WHERE (?1 IS NULL OR i.credit_card_id=?1) GROUP BY i.id ORDER BY i.fiscal_month DESC, c.name");
    let mut statement = connection.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([credit_card_id], map_invoice)
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

pub fn get_invoice_db(connection: &Connection, id: i64) -> Result<InvoiceDto, String> {
    let sql = format!("{INVOICE_SELECT} WHERE i.id=?1 GROUP BY i.id");
    connection
        .query_row(&sql, [id], map_invoice)
        .map_err(|_| "Fatura não encontrada.".into())
}

#[tauri::command]
pub fn get_invoice(state: State<'_, AppState>, id: i64) -> Result<InvoiceDto, String> {
    get_invoice_db(&state.db.open().map_err(|e| e.to_string())?, id)
}

#[tauri::command]
pub fn get_invoice_transactions(
    state: State<'_, AppState>,
    id: i64,
) -> Result<Vec<crate::commands::TransactionDto>, String> {
    transactions::list_transactions_by_invoice(&state.db.open().map_err(|e| e.to_string())?, id)
}

#[tauri::command]
pub fn set_invoice_status(
    state: State<'_, AppState>,
    id: i64,
    status: String,
) -> Result<InvoiceDto, String> {
    set_invoice_status_db(&state.db, id, &status)
}

pub fn set_invoice_status_db(db: &Database, id: i64, status: &str) -> Result<InvoiceDto, String> {
    if !["open", "closed", "paid", "cancelled"].contains(&status) {
        return Err("Status de fatura inválido.".into());
    }
    let mut connection = db.open().map_err(|e| e.to_string())?;
    let transaction = connection.transaction().map_err(|e| e.to_string())?;
    let exists: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM credit_card_invoices WHERE id=?1)",
            [id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if !exists {
        return Err("Fatura não encontrada.".into());
    }
    if status == "paid" {
        let now = Local::now().to_rfc3339();
        transaction.execute("UPDATE credit_card_invoices SET status='paid', paid_at=?1, updated_at=CURRENT_TIMESTAMP WHERE id=?2", (&now, id)).map_err(|e| e.to_string())?;
        transaction.execute("UPDATE transactions SET status='paid', updated_at=CURRENT_TIMESTAMP WHERE invoice_id=?1 AND status='open'", [id]).map_err(|e| e.to_string())?;
    } else {
        transaction.execute("UPDATE credit_card_invoices SET status=?1, paid_at=NULL, updated_at=CURRENT_TIMESTAMP WHERE id=?2", (status, id)).map_err(|e| e.to_string())?;
    }
    transaction.commit().map_err(|e| e.to_string())?;
    get_invoice_db(&connection, id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculates_invoice_dates_and_clamps_short_months() {
        assert_eq!(
            invoice_dates("2026-10", 20, 10).unwrap(),
            ("2026-10-20".into(), "2026-11-10".into())
        );
        assert_eq!(
            invoice_dates("2026-01", 31, 31).unwrap(),
            ("2026-01-31".into(), "2026-02-28".into())
        );
        assert_eq!(
            invoice_dates("2027-12", 15, 8).unwrap(),
            ("2027-12-15".into(), "2028-01-08".into())
        );
    }
}
