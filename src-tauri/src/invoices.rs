use chrono::{Datelike, Local, NaiveDate};
use rusqlite::{params, Connection, OptionalExtension, Row};
use tauri::State;

use crate::{
    commands::{validate_date, validate_month, DeclaredInvoiceInput, InvoiceDto, InvoiceGroupsDto},
    db::Database,
    settings, transactions, AppState,
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
       i.declared_amount_cents, i.declared_at, i.declared_note,
       COALESCE(SUM(CASE WHEN t.status != 'cancelled' THEN t.amount_cents ELSE 0 END), 0) AS transactions_total_cents,
       i.declared_amount_cents + COALESCE(SUM(CASE WHEN t.status != 'cancelled' THEN t.amount_cents ELSE 0 END), 0) AS total_cents,
       COALESCE(SUM(CASE WHEN t.status != 'cancelled' THEN 1 ELSE 0 END), 0) AS transaction_count
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
        declared_amount_cents: row.get(9)?,
        declared_at: row.get(10)?,
        declared_note: row.get(11)?,
        transactions_total_cents: row.get(12)?,
        total_cents: row.get(13)?,
        transaction_count: row.get(14)?,
    })
}

fn normalized_note(note: Option<String>) -> Option<String> {
    note.map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn default_closing_date(due_date: &str, closing_day: u32) -> Result<String, String> {
    let due = NaiveDate::parse_from_str(due_date, "%Y-%m-%d").map_err(|e| e.to_string())?;
    let same_month = date_with_day(due.year(), due.month(), closing_day);
    let closing = if same_month <= due {
        same_month
    } else {
        let (year, month) = if due.month() == 1 {
            (due.year() - 1, 12)
        } else {
            (due.year(), due.month() - 1)
        };
        date_with_day(year, month, closing_day)
    };
    Ok(closing.format("%Y-%m-%d").to_string())
}

fn validate_declared_input(input: &DeclaredInvoiceInput) -> Result<(), String> {
    validate_month(&input.fiscal_month)?;
    validate_date(&input.due_date)?;
    if let Some(closing_date) = input.closing_date.as_deref() {
        validate_date(closing_date)?;
        if closing_date > input.due_date.as_str() {
            return Err("A data de fechamento não pode ser posterior ao vencimento.".into());
        }
    }
    if input.declared_amount_cents < 0 {
        return Err("O saldo declarado não pode ser negativo.".into());
    }
    if !["open", "closed", "paid"].contains(&input.status.as_str()) {
        return Err("Status de fatura declarada inválido.".into());
    }
    Ok(())
}

#[tauri::command]
pub fn create_declared_invoice(
    state: State<'_, AppState>,
    input: DeclaredInvoiceInput,
) -> Result<InvoiceDto, String> {
    create_declared_invoice_db(&state.db, input)
}

pub fn create_declared_invoice_db(
    db: &Database,
    input: DeclaredInvoiceInput,
) -> Result<InvoiceDto, String> {
    validate_declared_input(&input)?;
    let mut connection = db.open().map_err(|e| e.to_string())?;
    let transaction = connection.transaction().map_err(|e| e.to_string())?;
    let closing_day: u32 = transaction
        .query_row(
            "SELECT closing_day FROM credit_cards WHERE id=?1",
            [input.credit_card_id],
            |row| row.get(0),
        )
        .map_err(|_| "Cartão não encontrado.".to_string())?;
    let closing_date = match input.closing_date.as_deref() {
        Some(value) => value.to_string(),
        None => default_closing_date(&input.due_date, closing_day)?,
    };
    let now = Local::now().to_rfc3339();
    let note = normalized_note(input.declared_note);
    let paid_at = (input.status == "paid").then_some(now.as_str());
    let existing_id = transaction
        .query_row(
            "SELECT id FROM credit_card_invoices WHERE credit_card_id=?1 AND fiscal_month=?2",
            (input.credit_card_id, &input.fiscal_month),
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let id = if let Some(id) = existing_id {
        transaction.execute(
            "UPDATE credit_card_invoices SET closing_date=?1, due_date=?2, status=?3, paid_at=?4, declared_amount_cents=?5, declared_at=?6, declared_note=?7, updated_at=CURRENT_TIMESTAMP WHERE id=?8",
            params![closing_date, input.due_date, input.status, paid_at, input.declared_amount_cents, now, note, id],
        ).map_err(|e| format!("Não foi possível declarar a fatura: {e}"))?;
        id
    } else {
        transaction.execute(
            "INSERT INTO credit_card_invoices(credit_card_id, fiscal_month, closing_date, due_date, status, paid_at, declared_amount_cents, declared_at, declared_note) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![input.credit_card_id, input.fiscal_month, closing_date, input.due_date, input.status, paid_at, input.declared_amount_cents, now, note],
        ).map_err(|e| format!("Não foi possível declarar a fatura: {e}"))?;
        transaction.last_insert_rowid()
    };
    if input.status == "paid" {
        transaction.execute(
            "UPDATE transactions SET status='paid', updated_at=CURRENT_TIMESTAMP WHERE invoice_id=?1 AND status='open'",
            [id],
        ).map_err(|e| e.to_string())?;
    }
    transaction.commit().map_err(|e| e.to_string())?;
    get_invoice_db(&connection, id)
}

#[tauri::command]
pub fn update_invoice_declared_amount(
    state: State<'_, AppState>,
    id: i64,
    declared_amount_cents: i64,
    declared_note: Option<String>,
) -> Result<InvoiceDto, String> {
    update_invoice_declared_amount_db(&state.db, id, declared_amount_cents, declared_note)
}

pub fn update_invoice_declared_amount_db(
    db: &Database,
    id: i64,
    declared_amount_cents: i64,
    declared_note: Option<String>,
) -> Result<InvoiceDto, String> {
    if declared_amount_cents < 0 {
        return Err("O saldo declarado não pode ser negativo.".into());
    }
    let connection = db.open().map_err(|e| e.to_string())?;
    let changed = connection.execute(
        "UPDATE credit_card_invoices SET declared_amount_cents=?1, declared_at=?2, declared_note=?3, updated_at=CURRENT_TIMESTAMP WHERE id=?4",
        params![declared_amount_cents, Local::now().to_rfc3339(), normalized_note(declared_note), id],
    ).map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err("Fatura não encontrada.".into());
    }
    get_invoice_db(&connection, id)
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

#[tauri::command]
pub fn get_invoice_groups(
    state: State<'_, AppState>,
    fiscal_month: Option<String>,
) -> Result<InvoiceGroupsDto, String> {
    get_invoice_groups_db(
        &state.db.open().map_err(|e| e.to_string())?,
        fiscal_month.as_deref(),
    )
}

pub fn get_invoice_groups_db(
    connection: &Connection,
    fiscal_month: Option<&str>,
) -> Result<InvoiceGroupsDto, String> {
    let fiscal_month = match fiscal_month {
        Some(value) => {
            validate_month(value)?;
            value.to_string()
        }
        None => settings::get_default_fiscal_month_db(connection)?,
    };
    let invoices = list_invoices_db(connection, None)?;
    let current_invoices = invoices
        .iter()
        .filter(|invoice| {
            invoice.fiscal_month == fiscal_month && invoice.status.as_str() != "cancelled"
        })
        .cloned()
        .collect();
    let mut open_invoices: Vec<_> = invoices
        .iter()
        .filter(|invoice| ["open", "closed"].contains(&invoice.status.as_str()))
        .cloned()
        .collect();
    open_invoices.sort_by(|left, right| left.due_date.cmp(&right.due_date));
    let mut history_invoices: Vec<_> = invoices
        .into_iter()
        .filter(|invoice| invoice.status == "paid" && invoice.fiscal_month != fiscal_month)
        .collect();
    history_invoices.sort_by(|left, right| right.due_date.cmp(&left.due_date));
    Ok(InvoiceGroupsDto {
        fiscal_month,
        current_invoices,
        open_invoices,
        history_invoices,
    })
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
    use uuid::Uuid;

    fn database_with_card() -> (std::path::PathBuf, Database, i64) {
        let root =
            std::env::temp_dir().join(format!("freedom-finances-invoice-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let db = Database::at(root.join("test.db")).unwrap();
        let connection = db.open().unwrap();
        connection.execute("INSERT INTO credit_cards(name, closing_day, due_day, limit_cents) VALUES ('Inter',20,15,500000)", []).unwrap();
        let card_id = connection.last_insert_rowid();
        drop(connection);
        (root, db, card_id)
    }

    fn declared_input(
        card_id: i64,
        month: &str,
        amount: i64,
        due_date: &str,
    ) -> DeclaredInvoiceInput {
        DeclaredInvoiceInput {
            credit_card_id: card_id,
            fiscal_month: month.into(),
            declared_amount_cents: amount,
            due_date: due_date.into(),
            closing_date: None,
            status: "open".into(),
            declared_note: Some("Saldo anterior ao uso do app".into()),
        }
    }

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

    #[test]
    fn declared_invoice_keeps_cents_and_appears_without_transactions() {
        let (root, db, card_id) = database_with_card();
        let invoice = create_declared_invoice_db(
            &db,
            declared_input(card_id, "2026-11", 102_636, "2026-11-15"),
        )
        .unwrap();
        assert_eq!(invoice.declared_amount_cents, 102_636);
        assert_eq!(invoice.transactions_total_cents, 0);
        assert_eq!(invoice.total_cents, 102_636);
        assert_eq!(invoice.transaction_count, 0);
        assert!(invoice.declared_at.is_some());
        assert_eq!(
            list_invoices_db(&db.open().unwrap(), None).unwrap().len(),
            1
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn declared_amount_and_transactions_are_added_in_integer_cents() {
        let (root, db, card_id) = database_with_card();
        let invoice = create_declared_invoice_db(
            &db,
            declared_input(card_id, "2026-11", 102_636, "2026-11-15"),
        )
        .unwrap();
        let connection = db.open().unwrap();
        connection.execute(
            "INSERT INTO transactions(description, amount_cents, type, status, transaction_date, fiscal_month, credit_card_id, invoice_id) VALUES ('Compra', 1234, 'expense', 'open', '2026-10-14', '2026-11', ?1, ?2)",
            (card_id, invoice.id),
        ).unwrap();
        drop(connection);
        let total = get_invoice_db(&db.open().unwrap(), invoice.id).unwrap();
        assert_eq!(total.transactions_total_cents, 1_234);
        assert_eq!(total.total_cents, 103_870);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn paying_declared_invoice_changes_its_status() {
        let (root, db, card_id) = database_with_card();
        let invoice = create_declared_invoice_db(
            &db,
            declared_input(card_id, "2026-11", 102_636, "2026-11-15"),
        )
        .unwrap();
        let paid = set_invoice_status_db(&db, invoice.id, "paid").unwrap();
        assert_eq!(paid.status, "paid");
        assert_eq!(paid.total_cents, 102_636);
        assert!(paid.paid_at.is_some());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn creates_the_three_requested_open_invoices() {
        let (root, db, card_id) = database_with_card();
        for (month, amount, due_date) in [
            ("2026-11", 102_636, "2026-11-15"),
            ("2026-12", 54_586, "2026-12-15"),
            ("2027-01", 29_087, "2027-01-15"),
        ] {
            create_declared_invoice_db(&db, declared_input(card_id, month, amount, due_date))
                .unwrap();
        }
        let rows = list_invoices_db(&db.open().unwrap(), Some(card_id)).unwrap();
        assert_eq!(rows.len(), 3);
        assert!(rows
            .iter()
            .all(|invoice| invoice.status == "open" && invoice.transaction_count == 0));
        assert_eq!(
            rows.iter().map(|invoice| invoice.total_cents).sum::<i64>(),
            186_309
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn declared_invoice_without_transactions_is_current_for_selected_month() {
        let (root, db, card_id) = database_with_card();
        let invoice = create_declared_invoice_db(
            &db,
            declared_input(card_id, "2026-11", 102_636, "2026-11-15"),
        )
        .unwrap();
        let groups = get_invoice_groups_db(&db.open().unwrap(), Some("2026-11")).unwrap();
        assert_eq!(groups.current_invoices.len(), 1);
        assert_eq!(groups.current_invoices[0].id, invoice.id);
        assert_eq!(groups.current_invoices[0].transaction_count, 0);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn declared_open_invoice_appears_in_open_group() {
        let (root, db, card_id) = database_with_card();
        let invoice = create_declared_invoice_db(
            &db,
            declared_input(card_id, "2026-11", 102_636, "2026-11-15"),
        )
        .unwrap();
        let groups = get_invoice_groups_db(&db.open().unwrap(), Some("2026-11")).unwrap();
        assert!(groups.open_invoices.iter().any(|row| row.id == invoice.id));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn paid_invoice_is_current_but_not_open() {
        let (root, db, card_id) = database_with_card();
        let invoice = create_declared_invoice_db(
            &db,
            declared_input(card_id, "2026-11", 102_636, "2026-11-15"),
        )
        .unwrap();
        set_invoice_status_db(&db, invoice.id, "paid").unwrap();
        let groups = get_invoice_groups_db(&db.open().unwrap(), Some("2026-11")).unwrap();
        assert!(groups
            .current_invoices
            .iter()
            .any(|row| row.id == invoice.id));
        assert!(!groups.open_invoices.iter().any(|row| row.id == invoice.id));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cancelled_invoice_does_not_appear_in_groups() {
        let (root, db, card_id) = database_with_card();
        let invoice = create_declared_invoice_db(
            &db,
            declared_input(card_id, "2026-11", 102_636, "2026-11-15"),
        )
        .unwrap();
        set_invoice_status_db(&db, invoice.id, "cancelled").unwrap();
        let groups = get_invoice_groups_db(&db.open().unwrap(), Some("2026-11")).unwrap();
        assert!(groups.current_invoices.is_empty());
        assert!(groups.open_invoices.is_empty());
        assert!(groups.history_invoices.is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invoice_groups_use_default_fiscal_month_when_omitted() {
        let (root, db, card_id) = database_with_card();
        crate::settings::set_default_fiscal_month_db(&db.open().unwrap(), "2026-11").unwrap();
        let invoice = create_declared_invoice_db(
            &db,
            declared_input(card_id, "2026-11", 102_636, "2026-11-15"),
        )
        .unwrap();
        let groups = get_invoice_groups_db(&db.open().unwrap(), None).unwrap();
        assert_eq!(groups.fiscal_month, "2026-11");
        assert_eq!(groups.current_invoices[0].id, invoice.id);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn december_invoice_is_not_current_in_november() {
        let (root, db, card_id) = database_with_card();
        create_declared_invoice_db(
            &db,
            declared_input(card_id, "2026-12", 54_586, "2026-12-15"),
        )
        .unwrap();
        let groups = get_invoice_groups_db(&db.open().unwrap(), Some("2026-11")).unwrap();
        assert!(groups.current_invoices.is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn november_invoice_is_current_in_november() {
        let (root, db, card_id) = database_with_card();
        create_declared_invoice_db(
            &db,
            declared_input(card_id, "2026-11", 102_636, "2026-11-15"),
        )
        .unwrap();
        let groups = get_invoice_groups_db(&db.open().unwrap(), Some("2026-11")).unwrap();
        assert_eq!(groups.current_invoices.len(), 1);
        assert_eq!(groups.current_invoices[0].fiscal_month, "2026-11");
        std::fs::remove_dir_all(root).unwrap();
    }
}
