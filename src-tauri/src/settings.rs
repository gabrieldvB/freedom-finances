use chrono::{Datelike, Local};
use rusqlite::{Connection, OptionalExtension};
use tauri::State;

use crate::{commands::validate_month, AppState};

const DEFAULT_FISCAL_MONTH_KEY: &str = "default_fiscal_month";

pub fn get_setting_db(connection: &Connection, key: &str) -> Result<Option<String>, String> {
    if key.trim().is_empty() {
        return Err("A chave da configuração não pode ficar vazia.".into());
    }
    connection
        .query_row("SELECT value FROM settings WHERE key=?1", [key], |row| {
            row.get(0)
        })
        .optional()
        .map_err(|e| e.to_string())
}

pub fn set_setting_db(connection: &Connection, key: &str, value: &str) -> Result<(), String> {
    if key.trim().is_empty() {
        return Err("A chave da configuração não pode ficar vazia.".into());
    }
    if key == DEFAULT_FISCAL_MONTH_KEY {
        validate_month(value)?;
    }
    connection
        .execute(
            "INSERT INTO settings(key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            (key, value),
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn current_fiscal_month() -> String {
    let now = Local::now();
    format!("{:04}-{:02}", now.year(), now.month())
}

pub fn get_default_fiscal_month_db(connection: &Connection) -> Result<String, String> {
    match get_setting_db(connection, DEFAULT_FISCAL_MONTH_KEY)? {
        Some(value) => {
            validate_month(&value)?;
            Ok(value)
        }
        None => Ok(current_fiscal_month()),
    }
}

pub fn set_default_fiscal_month_db(
    connection: &Connection,
    fiscal_month: &str,
) -> Result<String, String> {
    validate_month(fiscal_month)?;
    set_setting_db(connection, DEFAULT_FISCAL_MONTH_KEY, fiscal_month)?;
    Ok(fiscal_month.to_string())
}

#[tauri::command]
pub fn get_setting(state: State<'_, AppState>, key: String) -> Result<Option<String>, String> {
    get_setting_db(&state.db.open().map_err(|e| e.to_string())?, &key)
}

#[tauri::command]
pub fn set_setting(state: State<'_, AppState>, key: String, value: String) -> Result<(), String> {
    set_setting_db(&state.db.open().map_err(|e| e.to_string())?, &key, &value)
}

#[tauri::command]
pub fn get_default_fiscal_month(state: State<'_, AppState>) -> Result<String, String> {
    get_default_fiscal_month_db(&state.db.open().map_err(|e| e.to_string())?)
}

#[tauri::command]
pub fn set_default_fiscal_month(
    state: State<'_, AppState>,
    fiscal_month: String,
) -> Result<String, String> {
    set_default_fiscal_month_db(&state.db.open().map_err(|e| e.to_string())?, &fiscal_month)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use uuid::Uuid;

    #[test]
    fn saves_default_fiscal_month_in_settings() {
        let root =
            std::env::temp_dir().join(format!("freedom-finances-settings-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let db = Database::at(root.join("test.db")).unwrap();
        let connection = db.open().unwrap();

        assert_eq!(
            set_default_fiscal_month_db(&connection, "2026-11").unwrap(),
            "2026-11"
        );
        assert_eq!(
            get_setting_db(&connection, DEFAULT_FISCAL_MONTH_KEY).unwrap(),
            Some("2026-11".into())
        );
        drop(connection);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn loads_default_fiscal_month_after_database_is_reopened() {
        let root =
            std::env::temp_dir().join(format!("freedom-finances-settings-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("test.db");
        let db = Database::at(path.clone()).unwrap();
        set_default_fiscal_month_db(&db.open().unwrap(), "2027-01").unwrap();
        drop(db);

        let reopened = Database::at(path).unwrap();
        assert_eq!(
            get_default_fiscal_month_db(&reopened.open().unwrap()).unwrap(),
            "2027-01"
        );
        drop(reopened);
        std::fs::remove_dir_all(root).unwrap();
    }
}
