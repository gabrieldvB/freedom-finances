mod backup;
mod cards;
mod categories;
mod commands;
mod dashboard;
mod db;
mod invoices;
mod schema;
mod transactions;

use db::Database;
use tauri::Manager;

pub struct AppState {
    db: Database,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let db = Database::initialize().map_err(std::io::Error::other)?;
            app.manage(AppState { db });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            dashboard::get_dashboard,
            transactions::list_transactions,
            transactions::create_transaction,
            transactions::update_transaction,
            transactions::set_transaction_status,
            transactions::delete_transaction,
            transactions::create_installments,
            commands::list_accounts,
            commands::create_account,
            commands::set_account_active,
            categories::list_categories,
            categories::create_category,
            categories::update_category,
            categories::set_category_active,
            cards::list_cards,
            cards::create_card,
            cards::update_card,
            cards::set_card_active,
            invoices::list_invoices,
            invoices::get_invoice,
            invoices::get_invoice_transactions,
            invoices::set_invoice_status,
            backup::get_app_paths,
            backup::create_backup,
            backup::open_directory,
        ])
        .run(tauri::generate_context!())
        .expect("erro ao executar Freedom Finances");
}
