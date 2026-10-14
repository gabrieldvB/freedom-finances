# Freedom Finances Desktop

Aplicativo desktop local e offline para finanças pessoais. A interface é React + TypeScript e o núcleo nativo é Tauri 2 + Rust + SQLite. Não existe API HTTP, login, Docker, VPS, Caddy, domínio ou sincronização remota: a interface chama comandos Rust com `invoke` e o Rust acessa o SQLite diretamente.

## O que já funciona

- Dashboard mensal com receitas, despesas, saldo, valores em aberto/pagos, faturas e gastos por categoria.
- CRUD de lançamentos, alteração de status e filtros por mês/status.
- CRUD e ativação/desativação de categorias, cartões e contas.
- Faturas mensais criadas automaticamente para lançamentos no cartão.
- Declaração manual de faturas preexistentes, mesmo sem lançamentos cadastrados.
- Mês fiscal padrão persistido localmente para o Dashboard e novos formulários.
- Saldo projetado sem duplicar compras individuais já incluídas nas faturas.
- Receitas e contas fixas com modelo recorrente, ocorrência mensal e lançamento vinculado.
- Cartões e faturas agrupados pelo mês fiscal selecionado, incluindo faturas declaradas sem lançamentos.
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

Dias que não existem no mês são ajustados para o último dia válido. O total da fatura é calculado exclusivamente em centavos inteiros:

```text
total = saldo declarado + lançamentos vinculados não cancelados
```

O saldo declarado permite registrar uma fatura que já existia antes do início do uso do app. Na tela **Cartões e faturas**, use **Declarar fatura em aberto** e informe cartão, mês fiscal, valor, vencimento, fechamento opcional, status e observação. Se a fatura do mesmo cartão e mês já existir, a declaração a atualiza sem apagar seus lançamentos.

Bancos criados por versões anteriores recebem as colunas de declaração por uma migração aditiva e idempotente durante a inicialização.

## Mês fiscal padrão e projeção

No Dashboard, escolha o mês desejado e use **Definir como padrão**. A preferência é salva na tabela `settings` com a chave `default_fiscal_month`; quando ela não existe, o aplicativo usa o mês atual do sistema. O valor também é mostrado em **Configurações** e sugerido ao criar lançamentos, receitas, compras parceladas e faturas declaradas.

O saldo projetado evita contar compras de cartão duas vezes:

```text
saldo projetado = receitas do mês
                 - despesas diretas do mês
                 - total das faturas com vencimento no mês
```

Faturas abertas, fechadas e pagas entram pela data de vencimento. Faturas canceladas e lançamentos cancelados não entram no cálculo.

## Fixos mensais

A aba **Fixos** separa o modelo recorrente de cada ocorrência mensal. Cadastre salário, água, internet, DAS ou outra receita/despesa com valor padrão, conta, categoria, dia previsto e intervalo de validade. Em seguida, selecione o mês e use **Gerar fixos do mês**.

A geração é idempotente: executar novamente para o mesmo mês não duplica ocorrências nem lançamentos. Cada ocorrência cria uma transação normal sem cartão, por isso entra automaticamente nas receitas, despesas diretas e no saldo projetado. Editar o valor de um mês atualiza a ocorrência e sua transação, mas preserva o valor padrão dos meses futuros. Pagar, receber, cancelar ou reabrir também sincroniza o status da transação.

As tabelas `recurring_items` e `recurring_occurrences` são criadas de forma aditiva com `CREATE TABLE IF NOT EXISTS`, preservando bancos existentes.

## Cartões por mês fiscal

A tela **Cartões e faturas** possui seu próprio seletor integrado ao mês selecionado no aplicativo. A fatura atual corresponde ao `fiscal_month` escolhido e pode estar aberta, fechada ou paga. A seção de faturas abertas inclui todos os registros `open` e `closed`, ordenados pelo vencimento, inclusive faturas declaradas sem transações. Faturas canceladas não são exibidas.

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

Os testes Rust cobrem datas e agrupamento de faturas, virada de ano, divisão de centavos, migração de banco antigo, faturas declaradas, dashboard/projeção, fixos mensais, sincronização entre ocorrência e transação, geração idempotente, pagamento e o fluxo integrado de compra em 3x com backup íntegro.

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
