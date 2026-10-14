use chrono::{Datelike, NaiveDate};
use rusqlite::{params, Connection, OptionalExtension, Row};
use tauri::State;

use crate::{
    commands::{validate_month, RecurringItemDto, RecurringItemInput, RecurringOccurrenceDto},
    db::Database,
    AppState,
};

const ITEM_SELECT: &str = r#"
SELECT r.id, r.name, r.type, r.category_id, c.name, r.account_id, a.name,
       r.default_amount_cents, r.due_day, r.start_month, r.end_month,
       r.active, r.notes
FROM recurring_items r
LEFT JOIN categories c ON c.id=r.category_id
JOIN accounts a ON a.id=r.account_id
"#;

const OCCURRENCE_SELECT: &str = r#"
SELECT o.id, o.recurring_item_id, r.name, r.type, r.category_id, c.name,
       r.account_id, a.name, o.transaction_id, o.fiscal_month,
       o.amount_cents, o.due_date, o.status, o.notes
FROM recurring_occurrences o
JOIN recurring_items r ON r.id=o.recurring_item_id
LEFT JOIN categories c ON c.id=r.category_id
JOIN accounts a ON a.id=r.account_id
"#;

fn map_item(row: &Row<'_>) -> rusqlite::Result<RecurringItemDto> {
    Ok(RecurringItemDto {
        id: row.get(0)?,
        name: row.get(1)?,
        item_type: row.get(2)?,
        category_id: row.get(3)?,
        category_name: row.get(4)?,
        account_id: row.get(5)?,
        account_name: row.get(6)?,
        default_amount_cents: row.get(7)?,
        due_day: row.get(8)?,
        start_month: row.get(9)?,
        end_month: row.get(10)?,
        active: row.get::<_, i64>(11)? != 0,
        notes: row.get(12)?,
    })
}

fn map_occurrence(row: &Row<'_>) -> rusqlite::Result<RecurringOccurrenceDto> {
    Ok(RecurringOccurrenceDto {
        id: row.get(0)?,
        recurring_item_id: row.get(1)?,
        recurring_item: row.get(2)?,
        item_type: row.get(3)?,
        category_id: row.get(4)?,
        category_name: row.get(5)?,
        account_id: row.get(6)?,
        account_name: row.get(7)?,
        transaction_id: row.get(8)?,
        fiscal_month: row.get(9)?,
        amount_cents: row.get(10)?,
        due_date: row.get(11)?,
        status: row.get(12)?,
        notes: row.get(13)?,
    })
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|current| current.trim().to_string())
        .filter(|current| !current.is_empty())
}

fn validate_item(connection: &Connection, input: &RecurringItemInput) -> Result<(), String> {
    if input.name.trim().is_empty() {
        return Err("Informe o nome do fixo.".into());
    }
    if !["income", "expense"].contains(&input.item_type.as_str()) {
        return Err("Tipo de fixo inválido.".into());
    }
    if input.default_amount_cents <= 0 {
        return Err("O valor padrão deve ser maior que zero.".into());
    }
    if !(1..=31).contains(&input.due_day) {
        return Err("O dia previsto deve ficar entre 1 e 31.".into());
    }
    validate_month(&input.start_month)?;
    if let Some(end_month) = input.end_month.as_deref() {
        validate_month(end_month)?;
        if end_month < input.start_month.as_str() {
            return Err("O mês final não pode ser anterior ao mês inicial.".into());
        }
    }
    let account_exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM accounts WHERE id=?1)",
            [input.account_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if !account_exists {
        return Err("Conta não encontrada.".into());
    }
    if let Some(category_id) = input.category_id {
        let category_type: Option<String> = connection
            .query_row(
                "SELECT type FROM categories WHERE id=?1",
                [category_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        match category_type {
            None => return Err("Categoria não encontrada.".into()),
            Some(category_type) if category_type != input.item_type => {
                return Err("A categoria não corresponde ao tipo do fixo.".into())
            }
            _ => {}
        }
    }
    Ok(())
}

fn get_item_db(connection: &Connection, id: i64) -> Result<RecurringItemDto, String> {
    let sql = format!("{ITEM_SELECT} WHERE r.id=?1");
    connection
        .query_row(&sql, [id], map_item)
        .map_err(|_| "Fixo não encontrado.".into())
}

pub fn list_recurring_items_db(connection: &Connection) -> Result<Vec<RecurringItemDto>, String> {
    let sql = format!("{ITEM_SELECT} ORDER BY r.active DESC, r.type, r.name");
    let mut statement = connection.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([], map_item)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

#[tauri::command]
pub fn list_recurring_items(state: State<'_, AppState>) -> Result<Vec<RecurringItemDto>, String> {
    list_recurring_items_db(&state.db.open().map_err(|e| e.to_string())?)
}

#[tauri::command]
pub fn create_recurring_item(
    state: State<'_, AppState>,
    input: RecurringItemInput,
) -> Result<RecurringItemDto, String> {
    create_recurring_item_db(&state.db, input)
}

pub fn create_recurring_item_db(
    db: &Database,
    mut input: RecurringItemInput,
) -> Result<RecurringItemDto, String> {
    input.end_month = normalize_optional(input.end_month);
    input.notes = normalize_optional(input.notes);
    let connection = db.open().map_err(|e| e.to_string())?;
    validate_item(&connection, &input)?;
    connection.execute(
        "INSERT INTO recurring_items(name, type, category_id, account_id, default_amount_cents, due_day, start_month, end_month, active, notes) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![input.name.trim(), input.item_type, input.category_id, input.account_id, input.default_amount_cents, input.due_day, input.start_month, input.end_month, input.active, input.notes],
    ).map_err(|e| format!("Não foi possível criar o fixo: {e}"))?;
    get_item_db(&connection, connection.last_insert_rowid())
}

#[tauri::command]
pub fn update_recurring_item(
    state: State<'_, AppState>,
    id: i64,
    input: RecurringItemInput,
) -> Result<RecurringItemDto, String> {
    update_recurring_item_db(&state.db, id, input)
}

pub fn update_recurring_item_db(
    db: &Database,
    id: i64,
    mut input: RecurringItemInput,
) -> Result<RecurringItemDto, String> {
    input.end_month = normalize_optional(input.end_month);
    input.notes = normalize_optional(input.notes);
    let connection = db.open().map_err(|e| e.to_string())?;
    validate_item(&connection, &input)?;
    let changed = connection.execute(
        "UPDATE recurring_items SET name=?1, type=?2, category_id=?3, account_id=?4, default_amount_cents=?5, due_day=?6, start_month=?7, end_month=?8, active=?9, notes=?10, updated_at=CURRENT_TIMESTAMP WHERE id=?11",
        params![input.name.trim(), input.item_type, input.category_id, input.account_id, input.default_amount_cents, input.due_day, input.start_month, input.end_month, input.active, input.notes, id],
    ).map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err("Fixo não encontrado.".into());
    }
    get_item_db(&connection, id)
}

#[tauri::command]
pub fn set_recurring_item_active(
    state: State<'_, AppState>,
    id: i64,
    active: bool,
) -> Result<RecurringItemDto, String> {
    let connection = state.db.open().map_err(|e| e.to_string())?;
    let changed = connection
        .execute(
            "UPDATE recurring_items SET active=?1, updated_at=CURRENT_TIMESTAMP WHERE id=?2",
            (active, id),
        )
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err("Fixo não encontrado.".into());
    }
    get_item_db(&connection, id)
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    (NaiveDate::from_ymd_opt(next_year, next_month, 1).unwrap() - chrono::Duration::days(1)).day()
}

fn occurrence_due_date(fiscal_month: &str, due_day: u32) -> Result<String, String> {
    validate_month(fiscal_month)?;
    let base = NaiveDate::parse_from_str(&format!("{fiscal_month}-01"), "%Y-%m-%d")
        .map_err(|e| e.to_string())?;
    let date = NaiveDate::from_ymd_opt(
        base.year(),
        base.month(),
        due_day.min(days_in_month(base.year(), base.month())),
    )
    .unwrap();
    Ok(date.format("%Y-%m-%d").to_string())
}

pub fn list_recurring_occurrences_db(
    connection: &Connection,
    fiscal_month: &str,
) -> Result<Vec<RecurringOccurrenceDto>, String> {
    validate_month(fiscal_month)?;
    let sql = format!(
        "{OCCURRENCE_SELECT} WHERE o.fiscal_month=?1 ORDER BY r.type DESC, o.due_date, r.name"
    );
    let mut statement = connection.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([fiscal_month], map_occurrence)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

#[tauri::command]
pub fn list_recurring_occurrences(
    state: State<'_, AppState>,
    fiscal_month: String,
) -> Result<Vec<RecurringOccurrenceDto>, String> {
    list_recurring_occurrences_db(&state.db.open().map_err(|e| e.to_string())?, &fiscal_month)
}

#[tauri::command]
pub fn ensure_recurring_occurrences_for_month(
    state: State<'_, AppState>,
    fiscal_month: String,
) -> Result<Vec<RecurringOccurrenceDto>, String> {
    ensure_recurring_occurrences_for_month_db(&state.db, &fiscal_month)
}

pub fn ensure_recurring_occurrences_for_month_db(
    db: &Database,
    fiscal_month: &str,
) -> Result<Vec<RecurringOccurrenceDto>, String> {
    validate_month(fiscal_month)?;
    let mut connection = db.open().map_err(|e| e.to_string())?;
    let transaction = connection.transaction().map_err(|e| e.to_string())?;
    let items = {
        let mut statement = transaction.prepare(
            "SELECT id, name, type, category_id, account_id, default_amount_cents, due_day, notes FROM recurring_items WHERE active=1 AND start_month<=?1 AND (end_month IS NULL OR end_month>=?1) ORDER BY id",
        ).map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([fiscal_month], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, u32>(6)?,
                    row.get::<_, Option<String>>(7)?,
                ))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        rows
    };

    for (item_id, name, item_type, category_id, account_id, amount, due_day, notes) in items {
        let exists: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM recurring_occurrences WHERE recurring_item_id=?1 AND fiscal_month=?2)",
                (item_id, fiscal_month),
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        if exists {
            continue;
        }
        let due_date = occurrence_due_date(fiscal_month, due_day)?;
        transaction.execute(
            "INSERT INTO transactions(description, amount_cents, type, status, transaction_date, fiscal_month, account_id, category_id, notes) VALUES (?1,?2,?3,'open',?4,?5,?6,?7,?8)",
            params![name, amount, item_type, due_date, fiscal_month, account_id, category_id, notes],
        ).map_err(|e| format!("Não foi possível criar o lançamento do fixo: {e}"))?;
        let transaction_id = transaction.last_insert_rowid();
        transaction.execute(
            "INSERT INTO recurring_occurrences(recurring_item_id, transaction_id, fiscal_month, amount_cents, due_date, status, notes) VALUES (?1,?2,?3,?4,?5,'open',?6)",
            params![item_id, transaction_id, fiscal_month, amount, due_date, notes],
        ).map_err(|e| format!("Não foi possível gerar a ocorrência: {e}"))?;
    }
    transaction.commit().map_err(|e| e.to_string())?;
    list_recurring_occurrences_db(&connection, fiscal_month)
}

#[tauri::command]
pub fn update_recurring_occurrence(
    state: State<'_, AppState>,
    id: i64,
    amount_cents: i64,
    notes: Option<String>,
) -> Result<RecurringOccurrenceDto, String> {
    update_recurring_occurrence_db(&state.db, id, amount_cents, notes)
}

pub fn update_recurring_occurrence_db(
    db: &Database,
    id: i64,
    amount_cents: i64,
    notes: Option<String>,
) -> Result<RecurringOccurrenceDto, String> {
    if amount_cents <= 0 {
        return Err("O valor do mês deve ser maior que zero.".into());
    }
    let notes = normalize_optional(notes);
    let mut connection = db.open().map_err(|e| e.to_string())?;
    let transaction = connection.transaction().map_err(|e| e.to_string())?;
    let transaction_id: i64 = transaction
        .query_row(
            "SELECT transaction_id FROM recurring_occurrences WHERE id=?1",
            [id],
            |row| row.get(0),
        )
        .map_err(|_| "Ocorrência não encontrada.".to_string())?;
    transaction.execute(
        "UPDATE recurring_occurrences SET amount_cents=?1, notes=?2, updated_at=CURRENT_TIMESTAMP WHERE id=?3",
        params![amount_cents, notes, id],
    ).map_err(|e| e.to_string())?;
    transaction.execute(
        "UPDATE transactions SET amount_cents=?1, notes=?2, updated_at=CURRENT_TIMESTAMP WHERE id=?3",
        params![amount_cents, notes, transaction_id],
    ).map_err(|e| e.to_string())?;
    transaction.commit().map_err(|e| e.to_string())?;
    get_occurrence_db(&connection, id)
}

#[tauri::command]
pub fn set_recurring_occurrence_status(
    state: State<'_, AppState>,
    id: i64,
    status: String,
) -> Result<RecurringOccurrenceDto, String> {
    set_recurring_occurrence_status_db(&state.db, id, &status)
}

pub fn set_recurring_occurrence_status_db(
    db: &Database,
    id: i64,
    status: &str,
) -> Result<RecurringOccurrenceDto, String> {
    if !["open", "paid", "cancelled"].contains(&status) {
        return Err("Status de ocorrência inválido.".into());
    }
    let mut connection = db.open().map_err(|e| e.to_string())?;
    let transaction = connection.transaction().map_err(|e| e.to_string())?;
    let transaction_id: i64 = transaction
        .query_row(
            "SELECT transaction_id FROM recurring_occurrences WHERE id=?1",
            [id],
            |row| row.get(0),
        )
        .map_err(|_| "Ocorrência não encontrada.".to_string())?;
    transaction
        .execute(
            "UPDATE recurring_occurrences SET status=?1, updated_at=CURRENT_TIMESTAMP WHERE id=?2",
            (status, id),
        )
        .map_err(|e| e.to_string())?;
    transaction
        .execute(
            "UPDATE transactions SET status=?1, updated_at=CURRENT_TIMESTAMP WHERE id=?2",
            (status, transaction_id),
        )
        .map_err(|e| e.to_string())?;
    transaction.commit().map_err(|e| e.to_string())?;
    get_occurrence_db(&connection, id)
}

fn get_occurrence_db(connection: &Connection, id: i64) -> Result<RecurringOccurrenceDto, String> {
    let sql = format!("{OCCURRENCE_SELECT} WHERE o.id=?1");
    connection
        .query_row(&sql, [id], map_occurrence)
        .map_err(|_| "Ocorrência não encontrada.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dashboard;
    use uuid::Uuid;

    struct Fixture {
        root: std::path::PathBuf,
        db: Database,
        account_id: i64,
        income_category_id: i64,
        expense_category_id: i64,
    }

    fn fixture() -> Fixture {
        let root =
            std::env::temp_dir().join(format!("freedom-finances-recurring-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let db = Database::at(root.join("test.db")).unwrap();
        let connection = db.open().unwrap();
        let account_id = connection
            .query_row("SELECT id FROM accounts LIMIT 1", [], |row| row.get(0))
            .unwrap();
        let income_category_id = connection
            .query_row(
                "SELECT id FROM categories WHERE type='income' LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let expense_category_id = connection
            .query_row(
                "SELECT id FROM categories WHERE type='expense' LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        drop(connection);
        Fixture {
            root,
            db,
            account_id,
            income_category_id,
            expense_category_id,
        }
    }

    fn input(fixture: &Fixture, item_type: &str) -> RecurringItemInput {
        RecurringItemInput {
            name: if item_type == "income" {
                "Salário".into()
            } else {
                "Água".into()
            },
            item_type: item_type.into(),
            category_id: Some(if item_type == "income" {
                fixture.income_category_id
            } else {
                fixture.expense_category_id
            }),
            account_id: fixture.account_id,
            default_amount_cents: if item_type == "income" {
                300_000
            } else {
                9_000
            },
            due_day: if item_type == "income" { 5 } else { 10 },
            start_month: "2026-11".into(),
            end_month: None,
            active: true,
            notes: None,
        }
    }

    fn cleanup(fixture: Fixture) {
        drop(fixture.db);
        std::fs::remove_dir_all(fixture.root).unwrap();
    }

    #[test]
    fn creates_income_recurring_item() {
        let fixture = fixture();
        let item = create_recurring_item_db(&fixture.db, input(&fixture, "income")).unwrap();
        assert_eq!(item.item_type, "income");
        assert_eq!(item.default_amount_cents, 300_000);
        cleanup(fixture);
    }

    #[test]
    fn creates_expense_recurring_item() {
        let fixture = fixture();
        let item = create_recurring_item_db(&fixture.db, input(&fixture, "expense")).unwrap();
        assert_eq!(item.item_type, "expense");
        assert_eq!(item.default_amount_cents, 9_000);
        cleanup(fixture);
    }

    #[test]
    fn generates_occurrence_and_linked_transaction_for_month() {
        let fixture = fixture();
        create_recurring_item_db(&fixture.db, input(&fixture, "expense")).unwrap();
        let rows = ensure_recurring_occurrences_for_month_db(&fixture.db, "2026-11").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].due_date, "2026-11-10");
        let transaction: (i64, String, i64, String) = fixture
            .db
            .open()
            .unwrap()
            .query_row(
                "SELECT amount_cents, type, account_id, status FROM transactions WHERE id=?1",
                [rows[0].transaction_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(
            transaction,
            (9_000, "expense".into(), fixture.account_id, "open".into())
        );
        cleanup(fixture);
    }

    #[test]
    fn generation_is_idempotent() {
        let fixture = fixture();
        create_recurring_item_db(&fixture.db, input(&fixture, "expense")).unwrap();
        ensure_recurring_occurrences_for_month_db(&fixture.db, "2026-11").unwrap();
        let rows = ensure_recurring_occurrences_for_month_db(&fixture.db, "2026-11").unwrap();
        assert_eq!(rows.len(), 1);
        let count: i64 = fixture
            .db
            .open()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM transactions", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
        cleanup(fixture);
    }

    #[test]
    fn item_does_not_generate_before_start_month() {
        let fixture = fixture();
        create_recurring_item_db(&fixture.db, input(&fixture, "income")).unwrap();

        let rows = ensure_recurring_occurrences_for_month_db(&fixture.db, "2026-10").unwrap();

        assert!(rows.is_empty());
        cleanup(fixture);
    }

    #[test]
    fn item_generates_in_start_month() {
        let fixture = fixture();
        create_recurring_item_db(&fixture.db, input(&fixture, "income")).unwrap();

        let rows = ensure_recurring_occurrences_for_month_db(&fixture.db, "2026-11").unwrap();

        assert_eq!(rows.len(), 1);
        cleanup(fixture);
    }

    #[test]
    fn item_generates_in_end_month() {
        let fixture = fixture();
        let mut bounded = input(&fixture, "income");
        bounded.end_month = Some("2027-07".into());
        create_recurring_item_db(&fixture.db, bounded).unwrap();

        let rows = ensure_recurring_occurrences_for_month_db(&fixture.db, "2027-07").unwrap();

        assert_eq!(rows.len(), 1);
        cleanup(fixture);
    }

    #[test]
    fn item_does_not_generate_after_end_month() {
        let fixture = fixture();
        let mut bounded = input(&fixture, "income");
        bounded.end_month = Some("2027-07".into());
        create_recurring_item_db(&fixture.db, bounded).unwrap();

        let rows = ensure_recurring_occurrences_for_month_db(&fixture.db, "2027-08").unwrap();

        assert!(rows.is_empty());
        cleanup(fixture);
    }

    #[test]
    fn item_without_end_month_keeps_generating() {
        let fixture = fixture();
        create_recurring_item_db(&fixture.db, input(&fixture, "income")).unwrap();

        let rows = ensure_recurring_occurrences_for_month_db(&fixture.db, "2035-03").unwrap();

        assert_eq!(rows.len(), 1);
        cleanup(fixture);
    }

    #[test]
    fn rejects_end_month_before_start_month() {
        let fixture = fixture();
        let mut invalid = input(&fixture, "income");
        invalid.end_month = Some("2026-10".into());

        let error = create_recurring_item_db(&fixture.db, invalid).unwrap_err();

        assert_eq!(error, "O mês final não pode ser anterior ao mês inicial.");
        cleanup(fixture);
    }

    #[test]
    fn edits_month_amount_without_changing_default() {
        let fixture = fixture();
        let item = create_recurring_item_db(&fixture.db, input(&fixture, "expense")).unwrap();
        let occurrence = ensure_recurring_occurrences_for_month_db(&fixture.db, "2026-11")
            .unwrap()
            .remove(0);
        let updated = update_recurring_occurrence_db(
            &fixture.db,
            occurrence.id,
            9_620,
            Some("Leitura real".into()),
        )
        .unwrap();
        assert_eq!(updated.amount_cents, 9_620);
        assert_eq!(
            get_item_db(&fixture.db.open().unwrap(), item.id)
                .unwrap()
                .default_amount_cents,
            9_000
        );
        let transaction_amount: i64 = fixture
            .db
            .open()
            .unwrap()
            .query_row(
                "SELECT amount_cents FROM transactions WHERE id=?1",
                [occurrence.transaction_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(transaction_amount, 9_620);
        cleanup(fixture);
    }

    #[test]
    fn paid_occurrence_updates_transaction_status() {
        let fixture = fixture();
        create_recurring_item_db(&fixture.db, input(&fixture, "expense")).unwrap();
        let occurrence = ensure_recurring_occurrences_for_month_db(&fixture.db, "2026-11")
            .unwrap()
            .remove(0);
        assert_eq!(
            set_recurring_occurrence_status_db(&fixture.db, occurrence.id, "paid")
                .unwrap()
                .status,
            "paid"
        );
        let status: String = fixture
            .db
            .open()
            .unwrap()
            .query_row(
                "SELECT status FROM transactions WHERE id=?1",
                [occurrence.transaction_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(status, "paid");
        cleanup(fixture);
    }

    #[test]
    fn cancelled_occurrence_cancels_transaction() {
        let fixture = fixture();
        create_recurring_item_db(&fixture.db, input(&fixture, "expense")).unwrap();
        let occurrence = ensure_recurring_occurrences_for_month_db(&fixture.db, "2026-11")
            .unwrap()
            .remove(0);
        assert_eq!(
            set_recurring_occurrence_status_db(&fixture.db, occurrence.id, "cancelled")
                .unwrap()
                .status,
            "cancelled"
        );
        let status: String = fixture
            .db
            .open()
            .unwrap()
            .query_row(
                "SELECT status FROM transactions WHERE id=?1",
                [occurrence.transaction_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(status, "cancelled");
        cleanup(fixture);
    }

    #[test]
    fn projected_balance_includes_fixed_income() {
        let fixture = fixture();
        create_recurring_item_db(&fixture.db, input(&fixture, "income")).unwrap();
        ensure_recurring_occurrences_for_month_db(&fixture.db, "2026-11").unwrap();
        let dashboard = dashboard::build_dashboard(&fixture.db.open().unwrap(), "2026-11").unwrap();
        assert_eq!(dashboard.projected_income_cents, 300_000);
        assert_eq!(dashboard.projected_balance_cents, 300_000);
        cleanup(fixture);
    }

    #[test]
    fn projected_balance_includes_fixed_expense() {
        let fixture = fixture();
        create_recurring_item_db(&fixture.db, input(&fixture, "expense")).unwrap();
        ensure_recurring_occurrences_for_month_db(&fixture.db, "2026-11").unwrap();
        let dashboard = dashboard::build_dashboard(&fixture.db.open().unwrap(), "2026-11").unwrap();
        assert_eq!(dashboard.projected_direct_expenses_cents, 9_000);
        assert_eq!(dashboard.projected_balance_cents, -9_000);
        cleanup(fixture);
    }

    #[test]
    fn inactive_item_does_not_generate_occurrence() {
        let fixture = fixture();
        let mut inactive = input(&fixture, "expense");
        inactive.active = false;
        create_recurring_item_db(&fixture.db, inactive).unwrap();
        assert!(
            ensure_recurring_occurrences_for_month_db(&fixture.db, "2026-11")
                .unwrap()
                .is_empty()
        );
        cleanup(fixture);
    }

    #[test]
    fn item_outside_start_and_end_month_does_not_generate() {
        let fixture = fixture();
        let mut bounded = input(&fixture, "expense");
        bounded.start_month = "2026-10".into();
        bounded.end_month = Some("2026-11".into());
        create_recurring_item_db(&fixture.db, bounded).unwrap();
        assert!(
            ensure_recurring_occurrences_for_month_db(&fixture.db, "2026-09")
                .unwrap()
                .is_empty()
        );
        assert!(
            ensure_recurring_occurrences_for_month_db(&fixture.db, "2026-12")
                .unwrap()
                .is_empty()
        );
        cleanup(fixture);
    }

    #[test]
    fn salary_and_water_acceptance_scenario_updates_projection_and_payment() {
        let fixture = fixture();
        create_recurring_item_db(&fixture.db, input(&fixture, "income")).unwrap();
        create_recurring_item_db(&fixture.db, input(&fixture, "expense")).unwrap();
        let rows = ensure_recurring_occurrences_for_month_db(&fixture.db, "2026-11").unwrap();
        let water = rows
            .iter()
            .find(|row| row.recurring_item == "Água")
            .unwrap();
        update_recurring_occurrence_db(&fixture.db, water.id, 9_620, None).unwrap();

        let dashboard = dashboard::build_dashboard(&fixture.db.open().unwrap(), "2026-11").unwrap();
        assert_eq!(dashboard.projected_income_cents, 300_000);
        assert_eq!(dashboard.projected_direct_expenses_cents, 9_620);
        assert_eq!(dashboard.projected_balance_cents, 290_380);

        let paid = set_recurring_occurrence_status_db(&fixture.db, water.id, "paid").unwrap();
        assert_eq!(paid.status, "paid");
        let transaction_status: String = fixture
            .db
            .open()
            .unwrap()
            .query_row(
                "SELECT status FROM transactions WHERE id=?1",
                [water.transaction_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(transaction_status, "paid");
        cleanup(fixture);
    }
}
