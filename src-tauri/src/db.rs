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
