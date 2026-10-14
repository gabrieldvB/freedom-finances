use chrono::{Datelike, NaiveDate};
use rusqlite::{params, Connection, OptionalExtension, Row};
use tauri::State;
use uuid::Uuid;

use crate::{
    commands::{validate_date, validate_month, InstallmentInput, TransactionDto, TransactionInput},
    db::Database,
    invoices::find_or_create_invoice,
    recurring, AppState,
};

const TRANSACTION_SELECT: &str = r#"
SELECT t.id, t.description, t.amount_cents, t.type, t.status, t.transaction_date,
       t.fiscal_month, t.account_id, a.name, t.credit_card_id, cc.name,
       t.invoice_id, t.category_id, cat.name, t.installment_group_id,
       t.installment_number, t.installment_total, t.notes, ro.id
FROM transactions t
LEFT JOIN accounts a ON a.id=t.account_id
LEFT JOIN credit_cards cc ON cc.id=t.credit_card_id
LEFT JOIN categories cat ON cat.id=t.category_id
LEFT JOIN recurring_occurrences ro ON ro.transaction_id=t.id
"#;

fn map_transaction(row: &Row<'_>) -> rusqlite::Result<TransactionDto> {
    Ok(TransactionDto {
        id: row.get(0)?,
        description: row.get(1)?,
        amount_cents: row.get(2)?,
        transaction_type: row.get(3)?,
        status: row.get(4)?,
        transaction_date: row.get(5)?,
        fiscal_month: row.get(6)?,
        account_id: row.get(7)?,
        account_name: row.get(8)?,
        credit_card_id: row.get(9)?,
        credit_card_name: row.get(10)?,
        invoice_id: row.get(11)?,
        category_id: row.get(12)?,
        category_name: row.get(13)?,
        installment_group_id: row.get(14)?,
        installment_number: row.get(15)?,
        installment_total: row.get(16)?,
        notes: row.get(17)?,
        recurring_occurrence_id: row.get(18)?,
    })
}

fn validate_input(input: &TransactionInput) -> Result<(), String> {
    if input.description.trim().is_empty() {
        return Err("Informe a descrição.".into());
    }
    if input.amount_cents <= 0 {
        return Err("O valor deve ser maior que zero.".into());
    }
    if !["income", "expense", "transfer"].contains(&input.transaction_type.as_str()) {
        return Err("Tipo de lançamento inválido.".into());
    }
    if !["open", "paid", "cancelled"].contains(&input.status.as_str()) {
        return Err("Status de lançamento inválido.".into());
    }
    validate_date(&input.transaction_date)?;
    validate_month(&input.fiscal_month)?;
    if input.account_id.is_none() == input.credit_card_id.is_none() {
        return Err("Escolha uma conta ou um cartão, nunca ambos.".into());
    }
    if input.transaction_type == "income" && input.credit_card_id.is_some() {
        return Err("Receitas não podem ser lançadas em cartão.".into());
    }
    Ok(())
}

fn get(connection: &Connection, id: i64) -> Result<TransactionDto, String> {
    let sql = format!("{TRANSACTION_SELECT} WHERE t.id=?1");
    connection
        .query_row(&sql, [id], map_transaction)
        .map_err(|_| "Lançamento não encontrado.".into())
}

pub fn list_transactions_db(
    connection: &Connection,
    fiscal_month: &str,
) -> Result<Vec<TransactionDto>, String> {
    validate_month(fiscal_month)?;
    let sql = format!(
        "{TRANSACTION_SELECT} WHERE t.fiscal_month=?1 ORDER BY t.transaction_date DESC, t.id DESC"
    );
    let mut statement = connection.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([fiscal_month], map_transaction)
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

pub fn list_transactions_by_invoice(
    connection: &Connection,
    invoice_id: i64,
) -> Result<Vec<TransactionDto>, String> {
    let sql = format!("{TRANSACTION_SELECT} WHERE t.invoice_id=?1 ORDER BY t.installment_number, t.transaction_date, t.id");
    let mut statement = connection.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([invoice_id], map_transaction)
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_transactions(
    state: State<'_, AppState>,
    fiscal_month: String,
) -> Result<Vec<TransactionDto>, String> {
    recurring::ensure_recurring_occurrences_for_month_db(&state.db, &fiscal_month)?;
    list_transactions_db(&state.db.open().map_err(|e| e.to_string())?, &fiscal_month)
}

fn insert_transaction(
    connection: &Connection,
    input: &TransactionInput,
    installment: Option<(&str, u32, u32)>,
) -> Result<i64, String> {
    validate_input(input)?;
    let invoice_id = match input.credit_card_id {
        Some(card_id) => Some(find_or_create_invoice(
            connection,
            card_id,
            &input.fiscal_month,
        )?),
        None => None,
    };
    let (group_id, installment_number, installment_total) = installment
        .map(|(group, number, total)| (Some(group), Some(number), Some(total)))
        .unwrap_or((None, None, None));
    connection.execute(
        "INSERT INTO transactions(description, amount_cents, type, status, transaction_date, fiscal_month, account_id, credit_card_id, invoice_id, category_id, installment_group_id, installment_number, installment_total, notes) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
        params![input.description.trim(), input.amount_cents, input.transaction_type, input.status, input.transaction_date, input.fiscal_month, input.account_id, input.credit_card_id, invoice_id, input.category_id, group_id, installment_number, installment_total, input.notes]
    ).map_err(|e| format!("Não foi possível criar o lançamento: {e}"))?;
    Ok(connection.last_insert_rowid())
}

#[tauri::command]
pub fn create_transaction(
    state: State<'_, AppState>,
    input: TransactionInput,
) -> Result<TransactionDto, String> {
    let connection = state.db.open().map_err(|e| e.to_string())?;
    let id = insert_transaction(&connection, &input, None)?;
    get(&connection, id)
}

#[tauri::command]
pub fn update_transaction(
    state: State<'_, AppState>,
    id: i64,
    input: TransactionInput,
) -> Result<TransactionDto, String> {
    validate_input(&input)?;
    let connection = state.db.open().map_err(|e| e.to_string())?;
    let invoice_id = match input.credit_card_id {
        Some(card_id) => Some(find_or_create_invoice(
            &connection,
            card_id,
            &input.fiscal_month,
        )?),
        None => None,
    };
    let changed = connection.execute("UPDATE transactions SET description=?1, amount_cents=?2, type=?3, status=?4, transaction_date=?5, fiscal_month=?6, account_id=?7, credit_card_id=?8, invoice_id=?9, category_id=?10, notes=?11, updated_at=CURRENT_TIMESTAMP WHERE id=?12", params![input.description.trim(), input.amount_cents, input.transaction_type, input.status, input.transaction_date, input.fiscal_month, input.account_id, input.credit_card_id, invoice_id, input.category_id, input.notes, id]).map_err(|e| format!("Não foi possível atualizar o lançamento: {e}"))?;
    if changed == 0 {
        return Err("Lançamento não encontrado.".into());
    }
    get(&connection, id)
}

#[tauri::command]
pub fn set_transaction_status(
    state: State<'_, AppState>,
    id: i64,
    status: String,
) -> Result<(), String> {
    set_transaction_status_db(&state.db, id, &status)
}

pub fn set_transaction_status_db(db: &Database, id: i64, status: &str) -> Result<(), String> {
    if !["open", "paid", "cancelled"].contains(&status) {
        return Err("Status inválido.".into());
    }
    let mut connection = db.open().map_err(|e| e.to_string())?;
    let transaction = connection.transaction().map_err(|e| e.to_string())?;
    let changed = transaction
        .execute(
            "UPDATE transactions SET status=?1, updated_at=CURRENT_TIMESTAMP WHERE id=?2",
            (status, id),
        )
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err("Lançamento não encontrado.".into());
    }
    transaction
        .execute(
            "UPDATE recurring_occurrences SET status=?1, updated_at=CURRENT_TIMESTAMP WHERE transaction_id=?2",
            (status, id),
        )
        .map_err(|e| e.to_string())?;
    transaction.commit().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_transaction(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    delete_transaction_db(&state.db, id)
}

pub fn delete_transaction_db(db: &Database, id: i64) -> Result<(), String> {
    let mut connection = db.open().map_err(|e| e.to_string())?;
    let transaction = connection.transaction().map_err(|e| e.to_string())?;
    let recurring_occurrence_id = transaction
        .query_row(
            "SELECT id FROM recurring_occurrences WHERE transaction_id=?1",
            [id],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|_| "Não foi possível verificar o histórico do lançamento.".to_string())?;
    if let Some(occurrence_id) = recurring_occurrence_id {
        transaction
            .execute(
                "UPDATE recurring_occurrences SET status='cancelled', updated_at=CURRENT_TIMESTAMP WHERE id=?1",
                [occurrence_id],
            )
            .map_err(|e| format!("Não foi possível cancelar a ocorrência mensal: {e}"))?;
        transaction
            .execute(
                "UPDATE transactions SET status='cancelled', updated_at=CURRENT_TIMESTAMP WHERE id=?1",
                [id],
            )
            .map_err(|e| format!("Não foi possível cancelar o lançamento do mês: {e}"))?;
        return transaction.commit().map_err(|e| e.to_string());
    }
    let changed = transaction
        .execute("DELETE FROM transactions WHERE id=?1", [id])
        .map_err(|_| {
            "Não foi possível excluir o lançamento porque ele possui histórico vinculado."
                .to_string()
        })?;
    if changed == 0 {
        Err("Lançamento não encontrado.".into())
    } else {
        transaction.commit().map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub fn delete_installment_transactions(
    state: State<'_, AppState>,
    transaction_id: i64,
    scope: String,
) -> Result<(), String> {
    delete_installment_transactions_db(&state.db, transaction_id, &scope)
}

pub fn delete_installment_transactions_db(
    db: &Database,
    transaction_id: i64,
    scope: &str,
) -> Result<(), String> {
    if !["single", "from_current", "all"].contains(&scope) {
        return Err("Escopo de exclusão inválido.".into());
    }
    const DELETE_ERROR: &str =
        "Não foi possível excluir as parcelas selecionadas. Nenhuma alteração foi aplicada.";

    let mut connection = db.open().map_err(|_| DELETE_ERROR.to_string())?;
    let transaction = connection
        .transaction()
        .map_err(|_| DELETE_ERROR.to_string())?;
    let selected = transaction
        .query_row(
            r#"
            SELECT t.installment_group_id, t.installment_number, ro.id
            FROM transactions t
            LEFT JOIN recurring_occurrences ro ON ro.transaction_id=t.id
            WHERE t.id=?1
            "#,
            [transaction_id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<u32>>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                ))
            },
        )
        .optional()
        .map_err(|_| DELETE_ERROR.to_string())?
        .ok_or_else(|| "Lançamento não encontrado.".to_string())?;

    let (installment_group_id, installment_number, recurring_occurrence_id) = selected;
    if let Some(occurrence_id) = recurring_occurrence_id {
        transaction
            .execute(
                "UPDATE recurring_occurrences SET status='cancelled', updated_at=CURRENT_TIMESTAMP WHERE id=?1",
                [occurrence_id],
            )
            .map_err(|_| DELETE_ERROR.to_string())?;
        transaction
            .execute(
                "UPDATE transactions SET status='cancelled', updated_at=CURRENT_TIMESTAMP WHERE id=?1",
                [transaction_id],
            )
            .map_err(|_| DELETE_ERROR.to_string())?;
        return transaction.commit().map_err(|_| DELETE_ERROR.to_string());
    }

    let Some(group_id) = installment_group_id else {
        transaction
            .execute("DELETE FROM transactions WHERE id=?1", [transaction_id])
            .map_err(|_| DELETE_ERROR.to_string())?;
        return transaction.commit().map_err(|_| DELETE_ERROR.to_string());
    };

    let changed = match scope {
        "single" => transaction.execute(
            "DELETE FROM transactions WHERE id=?1 AND installment_group_id=?2",
            params![transaction_id, group_id],
        ),
        "from_current" => {
            let number = installment_number
                .ok_or_else(|| "Número da parcela não encontrado.".to_string())?;
            transaction.execute(
                "DELETE FROM transactions WHERE installment_group_id=?1 AND installment_number>=?2",
                params![group_id, number],
            )
        }
        "all" => transaction.execute(
            "DELETE FROM transactions WHERE installment_group_id=?1",
            [group_id],
        ),
        _ => unreachable!(),
    }
    .map_err(|_| DELETE_ERROR.to_string())?;
    if changed == 0 {
        return Err("Nenhuma parcela foi encontrada para exclusão.".into());
    }
    transaction.commit().map_err(|_| DELETE_ERROR.to_string())
}

pub fn split_cents(total: i64, installments: u32) -> Result<Vec<i64>, String> {
    if total <= 0 {
        return Err("O valor total deve ser maior que zero.".into());
    }
    if !(2..=120).contains(&installments) {
        return Err("Use entre 2 e 120 parcelas.".into());
    }
    let count = i64::from(installments);
    let base = total / count;
    let remainder = total % count;
    Ok((0..count)
        .map(|index| base + i64::from(index < remainder))
        .collect())
}

pub fn add_months(value: &str, months: u32) -> Result<String, String> {
    validate_month(value)?;
    let date =
        NaiveDate::parse_from_str(&format!("{value}-01"), "%Y-%m-%d").map_err(|e| e.to_string())?;
    let absolute = date.year() * 12 + date.month0() as i32 + months as i32;
    let year = absolute.div_euclid(12);
    let month = absolute.rem_euclid(12) as u32 + 1;
    Ok(format!("{year:04}-{month:02}"))
}

#[tauri::command]
pub fn create_installments(
    state: State<'_, AppState>,
    input: InstallmentInput,
) -> Result<Vec<TransactionDto>, String> {
    create_installments_db(&state.db, input)
}

pub fn create_installments_db(
    db: &Database,
    input: InstallmentInput,
) -> Result<Vec<TransactionDto>, String> {
    if input.description.trim().is_empty() {
        return Err("Informe a descrição.".into());
    }
    validate_date(&input.purchase_date)?;
    validate_month(&input.first_fiscal_month)?;
    let values = split_cents(input.total_amount_cents, input.installments)?;
    let mut connection = db.open().map_err(|e| e.to_string())?;
    let transaction = connection.transaction().map_err(|e| e.to_string())?;
    let group_id = Uuid::new_v4().to_string();
    let mut ids = Vec::with_capacity(input.installments as usize);
    for (index, amount_cents) in values.into_iter().enumerate() {
        let month = add_months(&input.first_fiscal_month, index as u32)?;
        let transaction_input = TransactionInput {
            description: input.description.clone(),
            amount_cents,
            transaction_type: "expense".into(),
            status: "open".into(),
            transaction_date: input.purchase_date.clone(),
            fiscal_month: month,
            account_id: None,
            credit_card_id: Some(input.credit_card_id),
            category_id: input.category_id,
            notes: input.notes.clone(),
        };
        ids.push(insert_transaction(
            &transaction,
            &transaction_input,
            Some((&group_id, index as u32 + 1, input.installments)),
        )?);
    }
    transaction.commit().map_err(|e| e.to_string())?;
    ids.into_iter().map(|id| get(&connection, id)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        backup, commands::RecurringItemInput, dashboard, db::Database, invoices, recurring,
    };

    struct InstallmentFixture {
        root: std::path::PathBuf,
        db: Database,
        card_id: i64,
    }

    fn installment_fixture() -> InstallmentFixture {
        let root = std::env::temp_dir().join(format!(
            "freedom-finances-installment-delete-{}",
            Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let db = Database::at(root.join("test.db")).unwrap();
        let connection = db.open().unwrap();
        connection.execute("INSERT INTO credit_cards(name, closing_day, due_day, limit_cents) VALUES ('Cartão teste',20,10,500000)", []).unwrap();
        let card_id = connection.last_insert_rowid();
        drop(connection);
        InstallmentFixture { root, db, card_id }
    }

    fn create_test_installments(
        fixture: &InstallmentFixture,
        description: &str,
        total_amount_cents: i64,
        installments: u32,
        first_fiscal_month: &str,
    ) -> Vec<TransactionDto> {
        create_installments_db(
            &fixture.db,
            InstallmentInput {
                description: description.into(),
                total_amount_cents,
                installments,
                purchase_date: "2026-10-13".into(),
                first_fiscal_month: first_fiscal_month.into(),
                credit_card_id: fixture.card_id,
                category_id: None,
                notes: None,
            },
        )
        .unwrap()
    }

    fn cleanup_installment_fixture(fixture: InstallmentFixture) {
        drop(fixture.db);
        std::fs::remove_dir_all(fixture.root).unwrap();
    }

    #[test]
    fn splits_cents_without_losing_remainder() {
        assert_eq!(split_cents(10_000, 3).unwrap(), vec![3_334, 3_333, 3_333]);
        assert_eq!(split_cents(10, 4).unwrap(), vec![3, 3, 2, 2]);
    }

    #[test]
    fn crosses_year_when_adding_months() {
        assert_eq!(add_months("2026-11", 0).unwrap(), "2026-11");
        assert_eq!(add_months("2026-11", 2).unwrap(), "2027-01");
    }

    #[test]
    fn installment_purchase_creates_future_invoices_and_payment_cascades() {
        let root = std::env::temp_dir().join(format!("freedom-finances-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let db = Database::at(root.join("freedom_finances.db")).unwrap();
        let connection = db.open().unwrap();
        connection.execute("INSERT INTO credit_cards(name, closing_day, due_day, limit_cents) VALUES ('Inter',20,10,500000)", []).unwrap();
        let card_id = connection.last_insert_rowid();
        drop(connection);

        let created = create_installments_db(
            &db,
            InstallmentInput {
                description: "Compra teste".into(),
                total_amount_cents: 10_000,
                installments: 3,
                purchase_date: "2026-10-13".into(),
                first_fiscal_month: "2026-11".into(),
                credit_card_id: card_id,
                category_id: None,
                notes: None,
            },
        )
        .unwrap();
        assert_eq!(
            created
                .iter()
                .map(|row| row.amount_cents)
                .collect::<Vec<_>>(),
            vec![3_334, 3_333, 3_333]
        );
        assert_eq!(
            created
                .iter()
                .map(|row| row.fiscal_month.as_str())
                .collect::<Vec<_>>(),
            vec!["2026-11", "2026-12", "2027-01"]
        );
        assert!(created
            .iter()
            .all(|row| row.invoice_id.is_some() && row.status == "open"));

        let invoices = invoices::list_invoices_db(&db.open().unwrap(), Some(card_id)).unwrap();
        assert_eq!(invoices.len(), 3);
        let first = invoices
            .iter()
            .find(|row| row.fiscal_month == "2026-11")
            .unwrap();
        invoices::set_invoice_status_db(&db, first.id, "paid").unwrap();
        let paid_rows = list_transactions_by_invoice(&db.open().unwrap(), first.id).unwrap();
        assert!(paid_rows.iter().all(|row| row.status == "paid"));
        let backup_path = backup::create_backup_db(&db).unwrap();
        let backup_connection = Connection::open(backup_path).unwrap();
        let integrity: String = backup_connection
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .unwrap();
        assert_eq!(integrity, "ok");
        drop(backup_connection);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn deleting_single_installment_preserves_the_rest_of_the_group() {
        let fixture = installment_fixture();
        let created = create_test_installments(&fixture, "Compra 4x", 40_000, 4, "2026-11");
        let group_id = created[0].installment_group_id.as_deref().unwrap();

        delete_installment_transactions_db(&fixture.db, created[1].id, "single").unwrap();

        let connection = fixture.db.open().unwrap();
        let remaining = connection
            .prepare("SELECT installment_number FROM transactions WHERE installment_group_id=?1 ORDER BY installment_number")
            .unwrap()
            .query_map([group_id], |row| row.get::<_, u32>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(remaining, vec![1, 3, 4]);
        assert!(get(&connection, created[1].id).is_err());
        drop(connection);
        cleanup_installment_fixture(fixture);
    }

    #[test]
    fn deleting_from_current_installment_preserves_earlier_and_other_groups() {
        let fixture = installment_fixture();
        let created = create_test_installments(&fixture, "Compra 12x", 120_000, 12, "2026-11");
        let other = create_test_installments(&fixture, "Outra compra", 25_000, 3, "2028-01");
        let group_id = created[0].installment_group_id.as_deref().unwrap();
        let other_group_id = other[0].installment_group_id.as_deref().unwrap();

        delete_installment_transactions_db(&fixture.db, created[4].id, "from_current").unwrap();

        let connection = fixture.db.open().unwrap();
        let remaining = connection
            .prepare("SELECT installment_number FROM transactions WHERE installment_group_id=?1 ORDER BY installment_number")
            .unwrap()
            .query_map([group_id], |row| row.get::<_, u32>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let other_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM transactions WHERE installment_group_id=?1",
                [other_group_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(remaining, vec![1, 2, 3, 4]);
        assert_eq!(other_count, 3);
        drop(connection);
        cleanup_installment_fixture(fixture);
    }

    #[test]
    fn deleting_all_twelve_installments_recalculates_invoices_and_dashboard() {
        let fixture = installment_fixture();
        let created = create_test_installments(&fixture, "Compra anual", 120_000, 12, "2026-11");
        let other = create_test_installments(&fixture, "Compra preservada", 25_000, 3, "2028-01");
        let group_id = created[0].installment_group_id.as_deref().unwrap();
        let other_group_id = other[0].installment_group_id.as_deref().unwrap();
        let first_invoice_id = created[0].invoice_id.unwrap();
        let connection = fixture.db.open().unwrap();
        let invoice_before = invoices::get_invoice_db(&connection, first_invoice_id).unwrap();
        let dashboard_before = dashboard::build_dashboard(&connection, "2026-12").unwrap();
        assert_eq!(invoice_before.transactions_total_cents, 10_000);
        assert_eq!(dashboard_before.projected_card_invoices_cents, 10_000);
        drop(connection);

        delete_installment_transactions_db(&fixture.db, created[0].id, "all").unwrap();

        let connection = fixture.db.open().unwrap();
        let remaining_group_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM transactions WHERE installment_group_id=?1",
                [group_id],
                |row| row.get(0),
            )
            .unwrap();
        let other_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM transactions WHERE installment_group_id=?1",
                [other_group_id],
                |row| row.get(0),
            )
            .unwrap();
        let invoice_after = invoices::get_invoice_db(&connection, first_invoice_id).unwrap();
        let dashboard_after = dashboard::build_dashboard(&connection, "2026-12").unwrap();
        let future_rows: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM transactions WHERE description='Compra anual' AND fiscal_month>='2026-11'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(remaining_group_count, 0);
        assert_eq!(future_rows, 0);
        assert_eq!(other_count, 3);
        assert_eq!(invoice_after.transactions_total_cents, 0);
        assert_eq!(invoice_after.transaction_count, 0);
        assert_eq!(dashboard_after.projected_card_invoices_cents, 0);
        drop(connection);
        cleanup_installment_fixture(fixture);
    }

    #[test]
    fn installment_delete_command_prioritizes_recurring_occurrence_safety() {
        let fixture = installment_fixture();
        let connection = fixture.db.open().unwrap();
        let account_id = connection
            .query_row("SELECT id FROM accounts LIMIT 1", [], |row| row.get(0))
            .unwrap();
        drop(connection);
        let item = recurring::create_recurring_item_db(
            &fixture.db,
            RecurringItemInput {
                name: "Fixo protegido".into(),
                item_type: "expense".into(),
                category_id: None,
                account_id,
                default_amount_cents: 10_000,
                due_day: 5,
                start_month: "2026-11".into(),
                end_month: None,
                active: true,
                notes: None,
            },
        )
        .unwrap();
        let occurrence =
            recurring::ensure_recurring_occurrences_for_month_db(&fixture.db, "2026-11")
                .unwrap()
                .remove(0);
        fixture.db.open().unwrap().execute(
            "UPDATE transactions SET installment_group_id='grupo-improvavel', installment_number=1, installment_total=12 WHERE id=?1",
            [occurrence.transaction_id],
        ).unwrap();

        delete_installment_transactions_db(&fixture.db, occurrence.transaction_id, "all").unwrap();

        let connection = fixture.db.open().unwrap();
        let statuses: (String, String) = connection
            .query_row(
                "SELECT o.status, t.status FROM recurring_occurrences o JOIN transactions t ON t.id=o.transaction_id WHERE o.id=?1",
                [occurrence.id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let item_exists: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM recurring_items WHERE id=?1)",
                [item.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(statuses, ("cancelled".into(), "cancelled".into()));
        assert!(item_exists);
        drop(connection);
        cleanup_installment_fixture(fixture);
    }

    #[test]
    fn deleting_a_normal_transaction_removes_it() {
        let root = std::env::temp_dir().join(format!(
            "freedom-finances-normal-transaction-{}",
            Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let db = Database::at(root.join("test.db")).unwrap();
        let connection = db.open().unwrap();
        let account_id = connection
            .query_row("SELECT id FROM accounts LIMIT 1", [], |row| row.get(0))
            .unwrap();
        let id = insert_transaction(
            &connection,
            &TransactionInput {
                description: "Lançamento avulso".into(),
                amount_cents: 12_300,
                transaction_type: "expense".into(),
                status: "open".into(),
                transaction_date: "2026-11-10".into(),
                fiscal_month: "2026-11".into(),
                account_id: Some(account_id),
                credit_card_id: None,
                category_id: None,
                notes: None,
            },
            None,
        )
        .unwrap();
        drop(connection);

        delete_transaction_db(&db, id).unwrap();

        let exists: bool = db
            .open()
            .unwrap()
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM transactions WHERE id=?1)",
                [id],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!exists);
        drop(db);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn deleting_a_recurring_transaction_cancels_only_that_month() {
        let root = std::env::temp_dir().join(format!(
            "freedom-finances-protected-transaction-{}",
            Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let db = Database::at(root.join("test.db")).unwrap();
        let connection = db.open().unwrap();
        let account_id = connection
            .query_row("SELECT id FROM accounts LIMIT 1", [], |row| row.get(0))
            .unwrap();
        drop(connection);
        let item = recurring::create_recurring_item_db(
            &db,
            RecurringItemInput {
                name: "Salário".into(),
                item_type: "income".into(),
                category_id: None,
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
        let occurrence = recurring::ensure_recurring_occurrences_for_month_db(&db, "2026-11")
            .unwrap()
            .remove(0);

        delete_transaction_db(&db, occurrence.transaction_id).unwrap();

        let saved = list_transactions_db(&db.open().unwrap(), "2026-11").unwrap();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].recurring_occurrence_id, Some(occurrence.id));
        assert_eq!(saved[0].status, "cancelled");
        let connection = db.open().unwrap();
        let occurrence_status: String = connection
            .query_row(
                "SELECT status FROM recurring_occurrences WHERE id=?1",
                [occurrence.id],
                |row| row.get(0),
            )
            .unwrap();
        let item_active: bool = connection
            .query_row(
                "SELECT active FROM recurring_items WHERE id=?1",
                [item.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(occurrence_status, "cancelled");
        assert!(item_active);
        drop(connection);

        let december =
            recurring::ensure_recurring_occurrences_for_month_db(&db, "2026-12").unwrap();
        assert_eq!(december.len(), 1);
        assert_eq!(december[0].recurring_item_id, item.id);
        assert_eq!(december[0].status, "open");
        drop(db);
        std::fs::remove_dir_all(root).unwrap();
    }
}
