use rusqlite::{Connection, OptionalExtension};
use tauri::State;

use crate::{
    commands::{validate_month, MonthlyBalanceDto},
    db::Database,
    AppState,
};

pub fn get_monthly_balance_db(
    connection: &Connection,
    fiscal_month: &str,
) -> Result<MonthlyBalanceDto, String> {
    validate_month(fiscal_month)?;
    let saved = connection
        .query_row(
            "SELECT opening_balance_cents, notes FROM monthly_balances WHERE fiscal_month=?1",
            [fiscal_month],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let (opening_balance_cents, notes) = saved.unwrap_or((0, None));
    Ok(MonthlyBalanceDto {
        fiscal_month: fiscal_month.to_string(),
        opening_balance_cents,
        notes,
    })
}

#[tauri::command]
pub fn get_monthly_balance(
    state: State<'_, AppState>,
    fiscal_month: String,
) -> Result<MonthlyBalanceDto, String> {
    get_monthly_balance_db(&state.db.open().map_err(|e| e.to_string())?, &fiscal_month)
}

pub fn set_monthly_balance_db(
    db: &Database,
    fiscal_month: &str,
    opening_balance_cents: i64,
    notes: Option<String>,
) -> Result<MonthlyBalanceDto, String> {
    validate_month(fiscal_month)?;
    let notes = notes
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let connection = db.open().map_err(|e| e.to_string())?;
    connection
        .execute(
            r#"
            INSERT INTO monthly_balances(fiscal_month, opening_balance_cents, notes)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(fiscal_month) DO UPDATE SET
                opening_balance_cents=excluded.opening_balance_cents,
                notes=excluded.notes,
                updated_at=CURRENT_TIMESTAMP
            "#,
            (fiscal_month, opening_balance_cents, notes),
        )
        .map_err(|e| format!("Não foi possível salvar o saldo inicial: {e}"))?;
    get_monthly_balance_db(&connection, fiscal_month)
}

#[tauri::command]
pub fn set_monthly_balance(
    state: State<'_, AppState>,
    fiscal_month: String,
    opening_balance_cents: i64,
    notes: Option<String>,
) -> Result<MonthlyBalanceDto, String> {
    set_monthly_balance_db(&state.db, &fiscal_month, opening_balance_cents, notes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn monthly_opening_balance_can_be_saved_updated_and_reloaded() {
        let root = std::env::temp_dir().join(format!(
            "freedom-finances-monthly-balance-{}",
            Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let db = Database::at(root.join("test.db")).unwrap();

        let saved =
            set_monthly_balance_db(&db, "2026-11", 147_312, Some("  Saldo bancário  ".into()))
                .unwrap();
        assert_eq!(saved.opening_balance_cents, 147_312);
        assert_eq!(saved.notes.as_deref(), Some("Saldo bancário"));

        set_monthly_balance_db(&db, "2026-11", 200_000, None).unwrap();
        let reloaded = get_monthly_balance_db(&db.open().unwrap(), "2026-11").unwrap();
        assert_eq!(reloaded.opening_balance_cents, 200_000);
        assert_eq!(reloaded.notes, None);

        drop(db);
        std::fs::remove_dir_all(root).unwrap();
    }
}
