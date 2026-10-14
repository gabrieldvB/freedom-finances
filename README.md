# Freedom Finances Desktop

Aplicativo desktop local e offline para finanças pessoais. A interface é React + TypeScript e o núcleo nativo é Tauri 2 + Rust + SQLite. Não existe API HTTP, login, Docker, VPS, Caddy, domínio ou sincronização remota: a interface chama comandos Rust com `invoke` e o Rust acessa o SQLite diretamente.

## O que já funciona

- Dashboard mensal com receitas, despesas, saldo, valores em aberto/pagos, faturas e gastos por categoria.
- CRUD de lançamentos, alteração de status e filtros por mês/status.
- CRUD e ativação/desativação de categorias, cartões e contas.
- Faturas mensais criadas automaticamente para lançamentos no cartão.
- Compra parcelada com divisão exata dos centavos e faturas futuras.
- Fechar, pagar, reabrir e cancelar fatura.
- Pagamento de fatura marca seus lançamentos abertos como pagos.
- Backup manual consistente pela API de backup do SQLite, validado com `PRAGMA integrity_check`.
- Build Linux em AppImage.

## Dados locais

O aplicativo cria os diretórios na primeira execução:

```text
Banco:   ~/.local/share/freedom-finances/freedom_finances.db
Backups: ~/Backups/freedom-finances/
```

O banco fica fora do código-fonte. Apagar, mover ou atualizar o projeto não apaga seus dados.

Na primeira execução são criados apenas uma conta `Carteira` e as categorias Salário, Mercado, Transporte, Alimentação, Internet, Assinaturas, Saúde, Casa, Lazer e Outros. Nenhum lançamento ou cartão fictício é criado.

## Preparação no Omarchy/Arch Linux

Instale as dependências de desenvolvimento do Tauri:

```bash
sudo pacman -Syu
sudo pacman -S --needed webkit2gtk-4.1 base-devel curl wget file openssl \
  appmenu-gtk-module libappindicator-gtk3 librsvg xdotool
```

Instale o Rust, caso `cargo --version` ainda não funcione:

```bash
curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf | sh
source "$HOME/.cargo/env"
```

Também é necessário Node.js com npm. Omarchy normalmente já oferece os dois; confirme com:

```bash
node --version
npm --version
rustc --version
```

## Desenvolvimento

```bash
cd ~/Projetos/freedom-finances-desktop
npm install
npm run tauri dev
```

O comando abre a janela desktop. O Vite usado durante desenvolvimento serve os arquivos da interface apenas em loopback enquanto o processo está aberto. O aplicativo compilado não inicia servidor HTTP e não expõe porta.

## Build e execução

```bash
npm run tauri build
```

O AppImage é gerado em:

```text
src-tauri/target/release/bundle/appimage/Freedom Finances_0.1.0_amd64.AppImage
```

Execute diretamente:

```bash
chmod +x "src-tauri/target/release/bundle/appimage/Freedom Finances_0.1.0_amd64.AppImage"
"src-tauri/target/release/bundle/appimage/Freedom Finances_0.1.0_amd64.AppImage"
```

## Como as faturas funcionam

Ao salvar um lançamento com cartão, o núcleo procura uma fatura do cartão para o mês fiscal. Se ela não existir, cria automaticamente:

```text
fechamento = mês fiscal + dia de fechamento
vencimento = mês seguinte + dia de vencimento
status inicial = open
```

Dias que não existem no mês são ajustados para o último dia válido. O total da fatura nunca é duplicado em uma coluna: ele é calculado a partir das transações vinculadas, ignorando lançamentos cancelados.

Uma compra de `10000` centavos em três parcelas gera:

```text
3334
3333
3333
```

Cada parcela recebe o mesmo UUID de grupo, sua numeração, o mês fiscal subsequente e a fatura correspondente. Ao pagar uma fatura, o app grava `paid_at` e muda para `paid` todas as transações `open` daquela fatura.

## Backup

Abra **Configurações → Criar backup agora**. O destino segue o padrão:

```text
~/Backups/freedom-finances/freedom_finances_YYYY-MM-DD_HH-MM-SS.db
```

O app usa uma cópia consistente mesmo com o banco em uso e só informa sucesso depois de receber `ok` do `PRAGMA integrity_check`. Os botões da mesma tela abrem as pastas do banco e dos backups no gerenciador de arquivos.

## Testes

```bash
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
```

Os testes Rust cobrem datas de fatura, virada de ano, divisão de centavos e um fluxo integrado: compra em 3x, criação de três faturas, pagamento em cascata e backup íntegro.

## Estrutura

```text
src/                    React, telas, componentes e chamadas invoke
src-tauri/src/db.rs     caminhos, conexão e seed local
src-tauri/src/schema.rs schema SQLite
src-tauri/src/*.rs      regras e comandos nativos por domínio
```

## Limitações atuais

- Não há importação ou sincronização com a versão antiga/VPS.
- Transferências ainda são tratadas como um lançamento único; a contrapartida automática entre duas contas fica para uma próxima versão.
- Não há recorrências, anexos, exportação CSV/OFX, conciliação bancária ou atualização automática.
- O projeto gera AppImage; empacotamento AUR/PKGBUILD pode ser adicionado depois.

## Privacidade

O aplicativo não faz chamadas de rede em produção. Fontes, ícones da interface e código necessários são empacotados localmente. Faça backups periódicos: não existe nuvem para recuperar um banco perdido.
