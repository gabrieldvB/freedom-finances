use tauri::State;

use crate::{
    commands::{CategoryDto, CategoryInput},
    AppState,
};

fn validate(input: &CategoryInput) -> Result<(), String> {
    if input.name.trim().is_empty() {
        return Err("Informe o nome da categoria.".into());
    }
    if !["income", "expense"].contains(&input.category_type.as_str()) {
        return Err("Tipo de categoria inválido.".into());
    }
    if !input.color.starts_with('#') || input.color.len() != 7 {
        return Err("Cor inválida; use #RRGGBB.".into());
    }
    Ok(())
}

#[tauri::command]
pub fn list_categories(state: State<'_, AppState>) -> Result<Vec<CategoryDto>, String> {
    let connection = state.db.open().map_err(|e| e.to_string())?;
    let mut statement = connection.prepare("SELECT id, name, type, color, icon, active FROM categories ORDER BY active DESC, type, name").map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok(CategoryDto {
                id: row.get(0)?,
                name: row.get(1)?,
                category_type: row.get(2)?,
                color: row.get(3)?,
                icon: row.get(4)?,
                active: row.get::<_, i64>(5)? != 0,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

fn get(connection: &rusqlite::Connection, id: i64) -> Result<CategoryDto, String> {
    connection
        .query_row(
            "SELECT id, name, type, color, icon, active FROM categories WHERE id=?1",
            [id],
            |row| {
                Ok(CategoryDto {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    category_type: row.get(2)?,
                    color: row.get(3)?,
                    icon: row.get(4)?,
                    active: row.get::<_, i64>(5)? != 0,
                })
            },
        )
        .map_err(|_| "Categoria não encontrada.".into())
}

#[tauri::command]
pub fn create_category(
    state: State<'_, AppState>,
    input: CategoryInput,
) -> Result<CategoryDto, String> {
    validate(&input)?;
    let connection = state.db.open().map_err(|e| e.to_string())?;
    connection
        .execute(
            "INSERT INTO categories(name, type, color, icon) VALUES (?1, ?2, ?3, ?4)",
            (
                input.name.trim(),
                &input.category_type,
                &input.color,
                &input.icon,
            ),
        )
        .map_err(|e| format!("Não foi possível criar a categoria: {e}"))?;
    get(&connection, connection.last_insert_rowid())
}

#[tauri::command]
pub fn update_category(
    state: State<'_, AppState>,
    id: i64,
    input: CategoryInput,
) -> Result<CategoryDto, String> {
    validate(&input)?;
    let connection = state.db.open().map_err(|e| e.to_string())?;
    let changed = connection.execute("UPDATE categories SET name=?1, type=?2, color=?3, icon=?4, updated_at=CURRENT_TIMESTAMP WHERE id=?5", (input.name.trim(), &input.category_type, &input.color, &input.icon, id)).map_err(|e| format!("Não foi possível atualizar a categoria: {e}"))?;
    if changed == 0 {
        return Err("Categoria não encontrada.".into());
    }
    get(&connection, id)
}

#[tauri::command]
pub fn set_category_active(
    state: State<'_, AppState>,
    id: i64,
    active: bool,
) -> Result<(), String> {
    let connection = state.db.open().map_err(|e| e.to_string())?;
    let changed = connection
        .execute(
            "UPDATE categories SET active=?1, updated_at=CURRENT_TIMESTAMP WHERE id=?2",
            (active, id),
        )
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        Err("Categoria não encontrada.".into())
    } else {
        Ok(())
    }
}
