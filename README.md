# Solana Level 1 Token Starter

Учебный starter для итоговых заданий первого уровня курса Superteam KZ. Он показывает современный минимальный каркас токен-программы без привязки к legacy JavaScript SDK.

> Это исходная точка, а не готовое решение. Не работайте напрямую в ветке `main`: для каждого задания создавайте отдельную ветку.

## Как получить проект через GitHub

Если вы ещё не работали с GitHub:

1. Нажмите **Fork** в правом верхнем углу страницы и создайте копию репозитория в своём аккаунте.
2. На странице своей копии нажмите **Code** и скопируйте HTTPS-ссылку.
3. Выполните в терминале:

   ```bash
   git clone <ссылка-на-ваш-fork>
   cd education
   git checkout -b task/01-tests
   ```

4. После выполнения задания сохраните изменения:

   ```bash
   git add .
   git commit -m "Complete task 01 tests"
   git push -u origin task/01-tests
   ```

5. Отправьте преподавателю ссылку на ветку `task/01-tests` или на последний commit.

Не знаете Git? Для этих заданий достаточно операций `clone`, `checkout -b`, `add`, `commit` и `push`; команды выше можно использовать как готовый сценарий.

## Задание 1 — покрыть токен-программу тестами

В проекте уже есть минимальный LiteSVM-тест `create_token`. Его нужно усилить и добавить тесты остальных реализованных инструкций.

### Что нужно сделать

- В тесте `create_token` проверить `decimals`, mint authority, supply и владельца mint, а не только наличие аккаунта.
- Покрыть `create_token_account`: проверить владельца token account, mint и token program.
- Покрыть `mint_tokens`: проверить изменение баланса получателя и общего supply.
- Покрыть `transfer_tokens`: проверить оба баланса и неизменность общего supply.
- Добавить негативные сценарии: нулевая сумма, неверный authority, другой mint и одинаковые source/destination.
- Обновить README в своём fork: указать версии, команды запуска и кратко описать добавленные тесты.

### Готовность задания

Чистый checkout вашей ветки должен проходить:

```bash
anchor build --ignore-keys
cargo test --workspace --locked
```

Флаг `--ignore-keys` нужен только потому, что локальный program keypair намеренно не хранится в учебном репозитории. Для собственного devnet-деплоя создайте keypair локально и синхронизируйте ID командой `anchor keys sync`, но не добавляйте файл keypair в Git.

Не публикуйте keypair, seed phrase, приватные ключи или `.env` с секретами.

Следующие задания выполняются в ветках `task/02-burn` и `task/03-escrow`. Реализация `burn_tokens` и escrow описана ниже.

## Зафиксированный стек

- Anchor CLI и crates: `1.1.2`
- Solana CLI: `3.1.10`
- Rust: `1.89.0`
- тесты программ: Rust + LiteSVM `0.10.0`
- токены: `anchor_spl::token_interface`, совместимый с Token Program и Token-2022
- рекомендуемый клиент для нового TypeScript-кода: `@solana/kit`

`@solana/web3.js` относится к legacy-стеку. TypeScript-клиент Anchor `@anchor-lang/core` по-прежнему зависит от `@solana/web3.js` v1, поэтому в этом starter тесты написаны на Rust и LiteSVM. Для нового клиентского приложения используйте `@solana/kit`, если задание явно не требует другого.

Оригинальный Token Program остается рабочим и широко используется. Для новых токенов в учебных заданиях используйте Token-2022, а program-код пишите через `token_interface`, чтобы сохранить совместимость с обоими Token Program.

## Задание 1 — что сделано в этой ветке

Ветка `task/01-tests` расширяет LiteSVM-тесты в `programs/solana-level-1-token-starter/tests/create_token.rs`.

### Версии

- Anchor CLI и crates: `1.1.2`
- Solana CLI (Agave): `3.1.10`
- Rust: `1.89.0` (platform-tools `v1.52` для SBF)
- LiteSVM: `0.10.0`
- токены: Token-2022 через `anchor_spl::token_interface`

На WSL `anchor build` может падать из‑за бага rustup (`invalid custom toolchain name: '1.89.0-sbpf-solana-v1.52'`). Обход: собрать программу через `cargo-build-sbf --no-rustup-override` rustc из platform-tools, без `+solana` toolchain.

### Команды

```bash
export PATH="$HOME/.cache/solana/v1.52/platform-tools/rust/bin:$HOME/solana-release/bin:$PATH"
export RUSTC="$HOME/.cache/solana/v1.52/platform-tools/rust/bin/rustc"
cargo-build-sbf --no-rustup-override \
  --manifest-path programs/solana-level-1-token-starter/Cargo.toml \
  --sbf-out-dir target/deploy

unset RUSTC
cargo test --workspace --locked
```

Ожидаемый результат: `test_id`, `test_token_lifecycle_end_to_end`, `test_negative_scenarios_failures` и `test_burn_tokens_reduces_balance` — `ok`. Тесты читают `target/deploy/solana_level_1_token_starter.so`. Отдельный запуск файла тестов: `cargo test --test create_token`.

### Тесты

- `test_token_lifecycle_end_to_end` — create mint (decimals, supply 0, mint authority, owner Token-2022), create ATA Alice/Bob (owner token program, mint, owner аккаунта), mint (баланс получателя и supply), transfer_checked (оба баланса, supply не меняется).
- `test_negative_scenarios_failures` — нулевая сумма mint/transfer, чужой authority, чужой mint, одинаковые source/destination; после ошибки supply и балансы не меняются.

### Архитектура теста

Тест собирает Anchor-инструкции (`CreateToken`, `CreateTokenAccount`, `MintTokens`, `TransferTokens`) и гоняет их в LiteSVM. ATA считаются через `get_associated_token_address_with_program_id` для Token-2022. Состояние mint и token account проверяется через `StateWithExtensions`.

## Задание 2 — сжигание токенов (`burn_tokens`)

Ветка `task/02-burn` добавляет инструкцию `burn_tokens` и LiteSVM-тест, который проверяет уменьшение баланса ATA и mint supply.

### Что сделано

- Инструкция `burn_tokens` в `programs/solana-level-1-token-starter/src/instructions/burn.rs`: CPI `burn_checked` через `token_interface`.
- Проверки: `amount > 0`, `token_account.mint == mint`, `token_account.owner == authority`; authority должен подписать транзакцию.
- Тест `test_burn_tokens_reduces_balance`: создаёт mint, ATA Alice, минтит `1_000_000`, сжигает `400_000` (подписывает Alice как владелец ATA), затем проверяет баланс и supply `600_000`.

### Команды

Сначала соберите программу (тесты читают `target/deploy/solana_level_1_token_starter.so`):

```bash
export PATH="$HOME/.local/bin:$HOME/.cache/solana/v1.52/platform-tools/rust/bin:$HOME/solana-release/bin:$PATH"
anchor build --ignore-keys
```

Запуск всех тестов файла `create_token`:

```bash
cargo test --test create_token
```

Только burn-сценарий:

```bash
cargo test --test create_token test_burn_tokens_reduces_balance -- --nocapture
```

Ожидаемый результат: `test_token_lifecycle_end_to_end`, `test_negative_scenarios_failures` и `test_burn_tokens_reduces_balance` — `ok`.

На WSL, если `anchor build` падает из‑за rustup, используйте сборку из раздела «Задание 1 — что сделано в этой ветке».

### Как LiteSVM проверяет балансы

LiteSVM — in-process Solana VM: тест загружает `.so`, шлёт Anchor-инструкции через `Transaction` и читает аккаунты из памяти SVM, без RPC и локального validator.

1. `svm.get_account(pubkey)` возвращает сырые данные mint или token account после `send()`.
2. `unpack_mint` / `unpack_token` разбирают их через `StateWithExtensions` (Token-2022): у mint берутся `decimals`, `supply`, `mint_authority`; у ATA — `mint`, `owner`, `amount`.
3. После mint тест сравнивает `amount` ATA и `supply` mint с `1_000_000`. После `burn_tokens` оба значения должны стать `1_000_000 - 400_000`. Transfer уменьшает один ATA и увеличивает другой, не меняя supply; burn уменьшает и баланс, и supply.

Так проверка идёт по ончейн-состоянию в SVM, а не по ответу CPI.

## Задание 3 — escrow

Ветка `task/03-escrow` добавляет программу `programs/escrow`: уникальная PDA-сделка, vault на Token-2022 и четыре инструкции с закрытием аккаунтов.

### Архитектура

Два аккаунта на сделку:

- **EscrowState** (PDA программы) — метаданные: `sender`, `receiver`, `mint`, `amount`, `deal_id`, `bump`, `status`.
- **Vault** — ATA Token-2022, authority = PDA. Не общий пул: один vault на одну сделку.

Сиды PDA:

```text
[b"escrow", sender.key().as_ref(), deal_id.to_le_bytes().as_ref()]
```

Один и тот же `deal_id` у другого `sender` даёт другой адрес. Повторный `initialize` с той же парой `(sender, deal_id)` падает на `init`.

| Инструкция | Signer | Эффект |
|---|---|---|
| `initialize(deal_id, amount)` | sender | создаёт PDA + vault, статус `Created` |
| `deposit(deal_id)` | sender | `transfer_checked` ровно `amount` в vault → `Funded` |
| `release(deal_id)` | **только sender** | токены на ATA получателя, close vault и PDA, рента sender |
| `cancel(deal_id)` | sender | из `Created`/`Funded` (возврат токенов если funded), close, рента sender |

Token program только Token-2022 (`token_interface` + `TOKEN_2022_PROGRAM_ID`). Переводы — `transfer_checked` с decimals из mint. `init_if_needed` не используется.

### Конечный автомат

```text
Created  --deposit-->  Funded  --release-->  Released  --> аккаунты закрыты
   |                     |
   +-------cancel--------+------>  Cancelled  --> аккаунты закрыты
```

Недопустимые переходы (`deposit` повторно, `release` не из `Funded`, `cancel` после close) отклоняются кодами `AlreadyProcessed` / `InvalidStatus` или отсутствием аккаунта.

### Модель угроз

| Угроза | Защита в программе |
|---|---|
| Чужой вызывает `release` / `deposit` / `cancel` | `sender: Signer` + PDA seeds от `sender` + `has_one = sender` |
| Подмена receiver при `release` | `has_one = receiver`; токены только на ATA с `token::authority = receiver` |
| Нулевая сумма | `require!(amount > 0)` на `initialize` |
| Повторный `deal_id` у того же sender | `init` PDA; аккаунт уже существует |
| Повторный `deposit` / `release` / `cancel` | статус + закрытие vault/`EscrowState` |
| Другой mint или Token Program | `has_one = mint`, `token::mint`, `token_program == TOKEN_2022` |
| Недостаточный баланс | CPI `transfer_checked` не проходит; статус остаётся `Created` |
| Общий vault / перепутанные сделки | vault = ATA от уникальной PDA, не shared pool |
| Кража ренты | `close = sender` и `close_account` destination = sender |

Клиентские проверки не заменяют эти constraints.

### Команды сборки и тестов

```bash
export PATH="$HOME/.local/bin:$HOME/.cache/solana/v1.52/platform-tools/rust/bin:$HOME/solana-release/bin:$PATH"
export RUSTC="$HOME/.cache/solana/v1.52/platform-tools/rust/bin/rustc"
cargo-build-sbf --no-rustup-override \
  --manifest-path programs/escrow/Cargo.toml \
  --sbf-out-dir target/deploy
cargo-build-sbf --no-rustup-override \
  --manifest-path programs/solana-level-1-token-starter/Cargo.toml \
  --sbf-out-dir target/deploy
unset RUSTC

cargo test -p escrow --test escrow
```

Или после `anchor build --ignore-keys`:

```bash
cargo test --workspace --locked
cargo test -p escrow --test escrow -- --nocapture
```

Ожидаемый результат: позитивные `initialize → deposit → release` / `cancel` и негативные (нулевая сумма, replay, подмена сторон, недостаточный баланс, повторный `deal_id`) — `ok`. Тесты в `programs/escrow/tests/escrow.rs` читают оба `.so` и проверяют балансы ATA, закрытие аккаунтов и возврат ренты sender через `svm.get_account`.

На WSL при ошибке rustup используйте сборку `cargo-build-sbf --no-rustup-override` из раздела задания 1.

## Что уже реализовано

- создание mint с выбранной token-программой;
- создание associated token account;
- выпуск токенов через `mint_to`;
- перевод через `transfer_checked`;
- сжигание через `burn_checked` (`burn_tokens`);
- проверки положительной суммы, полномочий, mint и token program на уровне Anchor accounts constraints;
- LiteSVM-тесты полного цикла Token-2022 (create / ATA / mint / transfer / burn) и негативные сценарии;
- escrow: PDA-сделка, vault ATA, `initialize` / `deposit` / `release` / `cancel`.

## Быстрый старт

1. Установите версии из раздела «Зафиксированный стек» через AVM, rustup и официальный Solana installer.
2. Для локального прохождения заданий выполните `anchor build --ignore-keys`. Для собственного devnet-деплоя создайте локальный program keypair и выполните `anchor keys sync`. Не коммитьте keypair или seed phrase.
3. После первой сборки выполните `cargo test --workspace --locked`.
4. Разрабатывайте каждое задание в отдельной ветке: `task/01-tests`, `task/02-burn`, `task/03-escrow`.

Тесты загружают `target/deploy/solana_level_1_token_starter.so` и `target/deploy/escrow.so`, поэтому перед первым `cargo test` нужна сборка обеих программ.

## Правила сдачи

- сдавайте публичную ссылку на GitHub-репозиторий и указывайте ветку или commit SHA;
- добавьте в README команды сборки и тестирования, ожидаемый результат и краткое описание архитектуры;
- не добавляйте в репозиторий private keys, seed phrases, `.env` с секретами или файлы keypair;
- не используйте `@solana/web3.js` в новом клиентском коде;
- для переводов токенов используйте `transfer_checked`, а не unchecked transfer;
- не подменяйте проверки полномочий только клиентской логикой: все критичные инварианты должны проверяться программой.

## Что считается современным решением

Современность здесь определяется не только номером версии. Решение должно использовать строгие account constraints, проверяемые state transitions, Token-2022 для нового токена, `token_interface` для совместимости, `transfer_checked` для переводов и воспроизводимые LiteSVM-тесты. Если официальные стабильные рекомендации Solana или Anchor изменятся, студент должен зафиксировать выбранные версии и объяснить отклонение в README.
