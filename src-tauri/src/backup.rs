use std::{process::Command, time::Duration};

use chrono::Local;
use rusqlite::{backup::Backup, Connection};
use tauri::State;

use crate::{commands::AppPathsDto, db::Database, AppState};

#[tauri::command]
pub fn get_app_paths(state: State<'_, AppState>) -> AppPathsDto {
    AppPathsDto {
        database: state.db.path().display().to_string(),
        database_directory: state.db.data_dir().display().to_string(),
        backups: state.db.backup_dir().display().to_string(),
    }
}

#[tauri::command]
pub fn create_backup(state: State<'_, AppState>) -> Result<String, String> {
    create_backup_db(&state.db)
}

pub fn create_backup_db(db: &Database) -> Result<String, String> {
    std::fs::create_dir_all(db.backup_dir())
        .map_err(|e| format!("Erro ao criar pasta de backup: {e}"))?;
    let filename = format!(
        "freedom_finances_{}.db",
        Local::now().format("%Y-%m-%d_%H-%M-%S")
    );
    let destination_path = db.backup_dir().join(filename);
    let source = db
        .open()
        .map_err(|e| format!("Erro ao abrir banco de origem: {e}"))?;
    let mut destination = Connection::open(&destination_path)
        .map_err(|e| format!("Erro ao criar arquivo de backup: {e}"))?;
    {
        let backup = Backup::new(&source, &mut destination)
            .map_err(|e| format!("Erro ao iniciar backup: {e}"))?;
        backup
            .run_to_completion(64, Duration::from_millis(10), None)
            .map_err(|e| format!("Erro ao copiar o banco: {e}"))?;
    }
    let integrity: String = destination
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|e| format!("Erro ao validar backup: {e}"))?;
    if integrity != "ok" {
        let _ = std::fs::remove_file(&destination_path);
        return Err(format!(
            "O backup falhou na verificação de integridade: {integrity}"
        ));
    }
    Ok(destination_path.display().to_string())
}

#[tauri::command]
pub fn open_directory(state: State<'_, AppState>, kind: String) -> Result<(), String> {
    let path = match kind.as_str() {
        "database" => state.db.data_dir(),
        "backups" => state.db.backup_dir(),
        _ => return Err("Pasta desconhecida.".into()),
    };
    Command::new("xdg-open")
        .arg(path)
        .spawn()
        .map_err(|e| format!("Não foi possível abrir a pasta: {e}"))?;
    Ok(())
}
