mod backup;
mod cards;
mod categories;
mod commands;
mod dashboard;
mod db;
mod invoices;
mod recurring;
mod schema;
mod settings;
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
            transactions::delete_installment_transactions,
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
            invoices::get_invoice_groups,
            invoices::get_invoice,
            invoices::get_invoice_transactions,
            invoices::create_declared_invoice,
            invoices::update_invoice_declared_amount,
            invoices::set_invoice_status,
            recurring::list_recurring_items,
            recurring::create_recurring_item,
            recurring::update_recurring_item,
            recurring::set_recurring_item_active,
            recurring::deactivate_recurring_item,
            recurring::delete_recurring_item,
            recurring::delete_recurring_item_with_history,
            recurring::end_recurring_item_from_month,
            recurring::list_recurring_occurrences,
            recurring::ensure_recurring_occurrences_for_month,
            recurring::update_recurring_occurrence,
            recurring::set_recurring_occurrence_status,
            recurring::cancel_recurring_occurrence,
            settings::get_setting,
            settings::set_setting,
            settings::get_default_fiscal_month,
            settings::set_default_fiscal_month,
            settings::get_initial_global_balance,
            settings::set_initial_global_balance,
            backup::get_app_paths,
            backup::create_backup,
            backup::open_directory,
        ])
        .run(tauri::generate_context!())
        .expect("erro ao executar Freedom Finances");
}
