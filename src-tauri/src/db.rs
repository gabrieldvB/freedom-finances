use std::{
    fs,
    path::{Path, PathBuf},
};

use rusqlite::{Connection, Result as SqlResult};

use crate::schema::SCHEMA;

#[derive(Clone)]
pub struct Database {
    path: PathBuf,
    backup_dir: PathBuf,
}

impl Database {
    pub fn initialize() -> Result<Self, String> {
        let home = dirs::home_dir().ok_or("Não foi possível localizar a pasta pessoal.")?;
        let data_root = dirs::data_local_dir()
            .unwrap_or_else(|| home.join(".local/share"))
            .join("freedom-finances");
        let backup_dir = home.join("Backups/freedom-finances");
        fs::create_dir_all(&data_root).map_err(|e| format!("Erro ao criar pasta de dados: {e}"))?;
        fs::create_dir_all(&backup_dir)
            .map_err(|e| format!("Erro ao criar pasta de backup: {e}"))?;
        let database = Self {
            path: data_root.join("freedom_finances.db"),
            backup_dir,
        };
        let connection = database
            .open()
            .map_err(|e| format!("Erro ao abrir banco: {e}"))?;
        connection
            .execute_batch(SCHEMA)
            .map_err(|e| format!("Erro ao criar schema: {e}"))?;
        migrate(&connection).map_err(|e| format!("Erro ao atualizar banco: {e}"))?;
        seed(&connection).map_err(|e| format!("Erro ao criar dados iniciais: {e}"))?;
        Ok(database)
    }

    #[cfg(test)]
    pub fn at(path: PathBuf) -> Result<Self, String> {
        let backup_dir = path.parent().unwrap_or(Path::new(".")).join("backups");
        fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;
        let database = Self { path, backup_dir };
        let connection = database.open().map_err(|e| e.to_string())?;
        connection
            .execute_batch(SCHEMA)
            .map_err(|e| e.to_string())?;
        migrate(&connection).map_err(|e| e.to_string())?;
        seed(&connection).map_err(|e| e.to_string())?;
        Ok(database)
    }

    pub fn open(&self) -> SqlResult<Connection> {
        let connection = Connection::open(&self.path)?;
        connection.execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;",
        )?;
        Ok(connection)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn data_dir(&self) -> &Path {
        self.path.parent().expect("database always has parent")
    }
    pub fn backup_dir(&self) -> &Path {
        &self.backup_dir
    }
}

fn has_column(connection: &Connection, table: &str, column: &str) -> SqlResult<bool> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
    for current in columns {
        if current? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn migrate(connection: &Connection) -> SqlResult<()> {
    if !has_column(connection, "credit_card_invoices", "declared_amount_cents")? {
        connection.execute(
            "ALTER TABLE credit_card_invoices ADD COLUMN declared_amount_cents INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    if !has_column(connection, "credit_card_invoices", "declared_at")? {
        connection.execute(
            "ALTER TABLE credit_card_invoices ADD COLUMN declared_at TEXT",
            [],
        )?;
    }
    if !has_column(connection, "credit_card_invoices", "declared_note")? {
        connection.execute(
            "ALTER TABLE credit_card_invoices ADD COLUMN declared_note TEXT",
            [],
        )?;
    }
    Ok(())
}

fn seed(connection: &Connection) -> SqlResult<()> {
    let account_count: i64 =
        connection.query_row("SELECT COUNT(*) FROM accounts", [], |row| row.get(0))?;
    if account_count == 0 {
        connection.execute("INSERT INTO accounts(name, type, initial_balance_cents) VALUES ('Carteira', 'cash', 0)", [])?;
    }
    let category_count: i64 =
        connection.query_row("SELECT COUNT(*) FROM categories", [], |row| row.get(0))?;
    if category_count == 0 {
        let categories = [
            ("Salário", "income", "#55d69d", "wallet"),
            ("Mercado", "expense", "#f2c56b", "shopping-cart"),
            ("Transporte", "expense", "#81b5ff", "car"),
            ("Alimentação", "expense", "#ff9b73", "utensils"),
            ("Internet", "expense", "#9b8cff", "wifi"),
            ("Assinaturas", "expense", "#ec7faa", "repeat"),
            ("Saúde", "expense", "#ff857f", "heart-pulse"),
            ("Casa", "expense", "#79d6c5", "house"),
            ("Lazer", "expense", "#c3a6ff", "gamepad"),
            ("Outros", "expense", "#91a69e", "tag"),
        ];
        for (name, kind, color, icon) in categories {
            connection.execute(
                "INSERT INTO categories(name, type, color, icon) VALUES (?1, ?2, ?3, ?4)",
                (name, kind, color, icon),
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn migrates_legacy_invoice_table_without_losing_data_and_is_idempotent() {
        let root =
            std::env::temp_dir().join(format!("freedom-finances-migration-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("legacy.db");
        let legacy = Connection::open(&path).unwrap();
        legacy.execute_batch(
            r#"
            CREATE TABLE credit_cards (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                closing_day INTEGER NOT NULL,
                due_day INTEGER NOT NULL,
                limit_cents INTEGER NOT NULL DEFAULT 0,
                active INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE credit_card_invoices (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                credit_card_id INTEGER NOT NULL REFERENCES credit_cards(id),
                fiscal_month TEXT NOT NULL,
                closing_date TEXT NOT NULL,
                due_date TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'open',
                paid_at TEXT,
                notes TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(credit_card_id, fiscal_month)
            );
            INSERT INTO credit_cards(id, name, closing_day, due_day) VALUES (1, 'Legado', 20, 15);
            INSERT INTO credit_card_invoices(id, credit_card_id, fiscal_month, closing_date, due_date, notes)
            VALUES (7, 1, '2026-11', '2026-10-20', '2026-11-15', 'preservar');
            "#,
        ).unwrap();
        drop(legacy);

        let db = Database::at(path).unwrap();
        let connection = db.open().unwrap();
        migrate(&connection).unwrap();
        let row: (i64, String, i64, Option<String>, Option<String>) = connection
            .query_row(
                "SELECT id, notes, declared_amount_cents, declared_at, declared_note FROM credit_card_invoices WHERE id=7",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .unwrap();
        assert_eq!(row, (7, "preservar".into(), 0, None, None));
        drop(connection);
        std::fs::remove_dir_all(root).unwrap();
    }
}
