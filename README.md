# Freedom Finances Desktop

Freedom Finances Desktop is an open source, local-first, offline desktop application for personal finance management.

It is built with **React**, **TypeScript**, **Tauri 2**, **Rust**, and **SQLite**. There is no HTTP API, login system, Docker dependency, VPS, Caddy, domain, or remote synchronization. The interface calls native Rust commands through `invoke`, and Rust accesses SQLite directly.

## About

Freedom Finances Desktop is an open source project created and maintained by **Gabriel Balduino**.

For contact, suggestions, or contributions:

**Email:** gabrielbalduino@proton.me

## What already works

- Monthly dashboard with income, expenses, balances, open/paid values, invoices, and category totals.
- Transaction CRUD, status changes, and filters by fiscal month/status.
- CRUD and activation/deactivation of categories, credit cards, and accounts.
- Monthly credit card invoices automatically created from card transactions.
- Manual declaration of pre-existing invoices, even without registered transactions.
- Locally persisted default fiscal month for the Dashboard and new forms.
- Projected balance without duplicating individual card purchases already included in invoices.
- Fixed income and recurring bills with recurring models, monthly occurrences, and linked transactions.
- Credit cards and invoices grouped by the selected fiscal month, including declared invoices without transactions.
- Installment purchases with exact cent distribution and future invoices.
- Invoice close, pay, reopen, and cancel actions.
- Paying an invoice marks its open linked transactions as paid.
- Safe deletion flows for recurring items, recurring occurrences, and installment purchases.
- Projected, realized, and accumulated cash flow.
- Manual SQLite backup validated with `PRAGMA integrity_check`.
- Modern dark interface with icons.
- Linux AppImage build.

## Local data

The application creates the required directories on first run:

```text
Database: ~/.local/share/freedom-finances/freedom_finances.db
Backups:  ~/Backups/freedom-finances/
```

The database is stored outside the source code directory. Deleting, moving, or updating the project does not delete your financial data.

On the first run, the app creates only one default account, `Wallet`, and a default set of categories such as Salary, Groceries, Transportation, Food, Internet, Subscriptions, Health, Home, Leisure, and Others. No fake transactions or fake credit cards are created.

## Preparation on Omarchy / Arch Linux

Install the Tauri development dependencies:

```bash
sudo pacman -Syu
sudo pacman -S --needed webkit2gtk-4.1 base-devel curl wget file openssl \
  appmenu-gtk-module libappindicator-gtk3 librsvg xdotool
```

Install Rust if `cargo --version` does not work yet:

```bash
curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf | sh
source "$HOME/.cargo/env"
```

Node.js and npm are also required. Omarchy usually provides both, but you can confirm with:

```bash
node --version
npm --version
rustc --version
cargo --version
```

## Development

```bash
cd ~/Projetos/freedom-finances-desktop
npm install
npm run tauri dev
```

This command opens the desktop window. During development, Vite serves the interface locally while the process is running. The compiled application does not start an HTTP server and does not expose a port.

## Build and run

```bash
npm run tauri build
```

The AppImage is generated at:

```text
src-tauri/target/release/bundle/appimage/Freedom Finances_0.1.0_amd64.AppImage
```

Run it directly:

```bash
chmod +x "src-tauri/target/release/bundle/appimage/Freedom Finances_0.1.0_amd64.AppImage"
"src-tauri/target/release/bundle/appimage/Freedom Finances_0.1.0_amd64.AppImage"
```

## How invoices work

When saving a transaction with a credit card, the native core searches for an invoice for that card and fiscal month. If it does not exist, it is created automatically:

```text
closing date = fiscal month + card closing day
due date     = following month + card due day
initial status = open
```

Invalid dates are adjusted to the last valid day of the month.

Invoice totals are calculated using integer cents only:

```text
total = declared invoice amount + non-cancelled linked transactions
```

The declared amount allows you to register an invoice that already existed before you started using the app.

In **Credit Cards and Invoices**, use **Declare open invoice** and provide the card, fiscal month, amount, due date, optional closing date, status, and notes. If an invoice for the same card and fiscal month already exists, the declaration updates it without deleting its linked transactions.

Databases created by previous versions receive the declaration columns through an additive and idempotent migration during startup.

## Default fiscal month and projection

On the Dashboard, select the desired month and use **Set as default**. The preference is saved in the `settings` table using the `default_fiscal_month` key. If the setting does not exist, the application uses the current system month.

The selected value is also shown in **Settings** and suggested when creating transactions, income, installment purchases, and declared invoices.

The projected balance avoids counting card purchases twice:

```text
projected balance = monthly income
                  - monthly direct expenses
                  - invoices due in the month
```

Open, closed, and paid invoices are included based on their due date. Cancelled invoices and cancelled transactions are excluded.

## Monthly recurring items

The **Recurring** page separates the recurring model from each monthly occurrence.

You can register salary, water bills, internet bills, taxes, subscriptions, or any other recurring income/expense with a default amount, account, category, expected day, and validity interval.

Each generated occurrence creates a normal non-card transaction. Because of that, recurring items automatically affect income, direct expenses, projected values, realized values, and accumulated balances.

Generation is idempotent: running it again for the same month does not duplicate occurrences or transactions.

Editing the value of a month updates the occurrence and its linked transaction, while preserving the default amount for future months. Paying, receiving, cancelling, or reopening an occurrence also synchronizes the linked transaction status.

The `recurring_items` and `recurring_occurrences` tables are created additively with `CREATE TABLE IF NOT EXISTS`, preserving existing databases.

## Recurring item deletion and ending

Recurring items support different actions depending on the intended result:

- **Remove this month**: removes only one monthly occurrence.
- **End from this month**: keeps previous history and prevents future generation.
- **Delete if no history exists**: physically deletes only clean recurring models.
- **Delete recurring item and history**: destructive action that removes the recurring model, its occurrences, and linked transactions.

The destructive action requires explicit confirmation by typing the recurring item name.

## Credit cards by fiscal month

The **Credit Cards and Invoices** page has its own month selector integrated with the app's selected fiscal month.

The current invoice corresponds to the selected `fiscal_month` and may be open, closed, or paid. The open invoices section includes all `open` and `closed` records ordered by due date, including manually declared invoices without transactions. Cancelled invoices are hidden.

An installment purchase of `10000` cents in three installments generates:

```text
3334
3333
3333
```

Each installment receives the same group UUID, its installment number, the following fiscal month, and the corresponding invoice.

When an invoice is paid, the app records `paid_at` and changes all `open` transactions linked to that invoice to `paid`.

## Installment purchase deletion

When deleting an installment transaction, the app asks which scope should be removed:

- only this installment
- this and the following installments
- the entire installment purchase

This prevents future installments from remaining as orphaned charges after deleting a test or incorrect installment purchase.

## Backup

Open **Settings → Create backup now**.

The destination follows this pattern:

```text
~/Backups/freedom-finances/freedom_finances_YYYY-MM-DD_HH-MM-SS.db
```

The app creates a consistent copy even while the database is in use and only reports success after receiving `ok` from `PRAGMA integrity_check`.

The same Settings page also provides buttons to open the database and backup folders in the file manager.

## Tests

```bash
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
```

Rust tests cover:

- date handling
- invoice grouping
- year transitions
- cent distribution
- legacy database migration
- declared invoices
- dashboard and projection
- recurring items
- recurring occurrence and transaction synchronization
- idempotent generation
- payment flows
- installment deletion scopes
- integrated three-installment purchase flow
- backup integrity

## Project structure

```text
src/                    React screens, components, and invoke calls
src-tauri/src/db.rs     local paths, SQLite connection, and local seed
src-tauri/src/schema.rs SQLite schema
src-tauri/src/*.rs      native commands and domain rules
```

## Current limitations

- There is no import or synchronization with the old web/VPS version.
- Transfers are still handled as a single transaction; automatic counterpart entries between accounts are planned for a future version.
- There is no CSV/OFX import yet.
- There are no attachments, automatic bank reconciliation, or auto-updates yet.
- The project currently builds an AppImage; AUR/PKGBUILD packaging may be added later.

## Privacy

The production application does not make network calls. The required interface assets, icons, and code are bundled locally.

All financial data remains on the user's machine.

Create periodic backups. There is no cloud recovery if the local database is lost.

## Repository hygiene

The following files should not be committed:

- local SQLite databases
- backups
- AppImage builds
- Tauri target directory
- Node modules
- environment files
- temporary files

## Roadmap

Possible future improvements:

- CSV import for bank statements and credit card invoices
- OFX import
- automatic category rules based on transaction descriptions
- CSV export
- charts and monthly reports
- multi-account balance projection
- release automation with GitHub Actions
- AUR package
- optional migration from previous versions

## License

This project is licensed under the MIT License.

See the `LICENSE` file for details.
