use tauri::State;

use crate::{
    commands::{CardDto, CardInput},
    AppState,
};

fn validate(input: &CardInput) -> Result<(), String> {
    if input.name.trim().is_empty() {
        return Err("Informe o nome do cartão.".into());
    }
    if !(1..=31).contains(&input.closing_day) || !(1..=31).contains(&input.due_day) {
        return Err("Fechamento e vencimento devem ficar entre 1 e 31.".into());
    }
    if input.limit_cents < 0 {
        return Err("O limite não pode ser negativo.".into());
    }
    Ok(())
}

#[tauri::command]
pub fn list_cards(state: State<'_, AppState>) -> Result<Vec<CardDto>, String> {
    let connection = state.db.open().map_err(|e| e.to_string())?;
    let mut statement = connection.prepare("SELECT id, name, closing_day, due_day, limit_cents, active FROM credit_cards ORDER BY active DESC, name").map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok(CardDto {
                id: row.get(0)?,
                name: row.get(1)?,
                closing_day: row.get(2)?,
                due_day: row.get(3)?,
                limit_cents: row.get(4)?,
                active: row.get::<_, i64>(5)? != 0,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

fn get(connection: &rusqlite::Connection, id: i64) -> Result<CardDto, String> {
    connection.query_row("SELECT id, name, closing_day, due_day, limit_cents, active FROM credit_cards WHERE id=?1", [id], |row| Ok(CardDto { id: row.get(0)?, name: row.get(1)?, closing_day: row.get(2)?, due_day: row.get(3)?, limit_cents: row.get(4)?, active: row.get::<_, i64>(5)? != 0 })).map_err(|_| "Cartão não encontrado.".into())
}

#[tauri::command]
pub fn create_card(state: State<'_, AppState>, input: CardInput) -> Result<CardDto, String> {
    validate(&input)?;
    let connection = state.db.open().map_err(|e| e.to_string())?;
    connection.execute("INSERT INTO credit_cards(name, closing_day, due_day, limit_cents) VALUES (?1, ?2, ?3, ?4)", (input.name.trim(), input.closing_day, input.due_day, input.limit_cents)).map_err(|e| format!("Não foi possível criar o cartão: {e}"))?;
    get(&connection, connection.last_insert_rowid())
}

#[tauri::command]
pub fn update_card(
    state: State<'_, AppState>,
    id: i64,
    input: CardInput,
) -> Result<CardDto, String> {
    validate(&input)?;
    let connection = state.db.open().map_err(|e| e.to_string())?;
    let changed = connection.execute("UPDATE credit_cards SET name=?1, closing_day=?2, due_day=?3, limit_cents=?4, updated_at=CURRENT_TIMESTAMP WHERE id=?5", (input.name.trim(), input.closing_day, input.due_day, input.limit_cents, id)).map_err(|e| format!("Não foi possível atualizar o cartão: {e}"))?;
    if changed == 0 {
        return Err("Cartão não encontrado.".into());
    }
    get(&connection, id)
}

#[tauri::command]
pub fn set_card_active(state: State<'_, AppState>, id: i64, active: bool) -> Result<(), String> {
    let connection = state.db.open().map_err(|e| e.to_string())?;
    let changed = connection
        .execute(
            "UPDATE credit_cards SET active=?1, updated_at=CURRENT_TIMESTAMP WHERE id=?2",
            (active, id),
        )
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        Err("Cartão não encontrado.".into())
    } else {
        Ok(())
    }
}
