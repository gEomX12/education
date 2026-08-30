use anchor_lang::{InstructionData, ToAccountMetas};
use litesvm::LiteSVM;
use solana_keypair::Keypair;
use solana_message::{AccountMeta, Instruction, Message};
use solana_signer::Signer;
use solana_transaction::Transaction;
use std::{fs, path::PathBuf};

const DECIMALS: u8 = 6;
const MINT_AMOUNT: u64 = 1_000_000;
const TRANSFER_AMOUNT: u64 = 250_000;
const BURN_AMOUNT: u64 = 400_000;

fn program_bytes() -> Vec<u8> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        manifest_dir.join("../../target/deploy/solana_level_1_token_starter.so"),
        manifest_dir.join("target/deploy/solana_level_1_token_starter.so"),
        PathBuf::from("target/deploy/solana_level_1_token_starter.so"),
    ];
    let path = candidates
        .into_iter()
        .find(|path| path.exists())
        .unwrap_or_else(|| {
            panic!(
                "Build the program with `anchor build` before running tests. Could not find solana_level_1_token_starter.so"
            )
        });
    fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "Build the program with `anchor build` before running tests. Could not read {}: {error}",
            path.display()
        )
    })
}

fn metas(accounts: impl ToAccountMetas) -> Vec<AccountMeta> {
    accounts
        .to_account_metas(None)
        .into_iter()
        .map(|meta| AccountMeta {
            pubkey: meta.pubkey,
            is_signer: meta.is_signer,
            is_writable: meta.is_writable,
        })
        .collect()
}

fn send(
    svm: &mut LiteSVM,
    payer: &Keypair,
    extra_signers: &[&Keypair],
    instruction: Instruction,
) -> Result<(), String> {
    let mut signers = vec![payer];
    for signer in extra_signers {
        if signer.pubkey() != payer.pubkey() {
            signers.push(signer);
        }
    }
    let transaction = Transaction::new(
        &signers,
        Message::new(&[instruction], Some(&payer.pubkey())),
        svm.latest_blockhash(),
    );
    svm.send_transaction(transaction)
        .map(|_| ())
        .map_err(|error| format!("{error:?}"))
}

fn ata(owner: &solana_keypair::Address, mint: &solana_keypair::Address) -> solana_keypair::Address {
    anchor_spl::associated_token::get_associated_token_address_with_program_id(
        owner,
        mint,
        &anchor_spl::token_2022::ID,
    )
}

fn create_token_ix(
    program_id: solana_keypair::Address,
    token_program: solana_keypair::Address,
    payer: &solana_keypair::Address,
    authority: &solana_keypair::Address,
    mint: &solana_keypair::Address,
) -> Instruction {
    Instruction {
        program_id,
        accounts: metas(solana_level_1_token_starter::accounts::CreateToken {
            payer: *payer,
            authority: *authority,
            mint: *mint,
            token_program,
            system_program: anchor_lang::system_program::ID,
        }),
        data: solana_level_1_token_starter::instruction::CreateToken { decimals: DECIMALS }
            .data(),
    }
}

fn create_token_account_ix(
    program_id: solana_keypair::Address,
    token_program: solana_keypair::Address,
    payer: &solana_keypair::Address,
    owner: &solana_keypair::Address,
    mint: &solana_keypair::Address,
) -> Instruction {
    Instruction {
        program_id,
        accounts: metas(solana_level_1_token_starter::accounts::CreateTokenAccount {
            payer: *payer,
            owner: *owner,
            mint: *mint,
            token_account: ata(owner, mint),
            token_program,
            associated_token_program: anchor_spl::associated_token::ID,
            system_program: anchor_lang::system_program::ID,
        }),
        data: solana_level_1_token_starter::instruction::CreateTokenAccount {}.data(),
    }
}

fn mint_tokens_ix(
    program_id: solana_keypair::Address,
    token_program: solana_keypair::Address,
    authority: &solana_keypair::Address,
    mint: &solana_keypair::Address,
    destination: &solana_keypair::Address,
    amount: u64,
) -> Instruction {
    Instruction {
        program_id,
        accounts: metas(solana_level_1_token_starter::accounts::MintTokens {
            authority: *authority,
            mint: *mint,
            destination: *destination,
            token_program,
        }),
        data: solana_level_1_token_starter::instruction::MintTokens { amount }.data(),
    }
}

fn transfer_tokens_ix(
    program_id: solana_keypair::Address,
    token_program: solana_keypair::Address,
    authority: &solana_keypair::Address,
    mint: &solana_keypair::Address,
    source: &solana_keypair::Address,
    destination: &solana_keypair::Address,
    amount: u64,
) -> Instruction {
    Instruction {
        program_id,
        accounts: metas(solana_level_1_token_starter::accounts::TransferTokens {
            authority: *authority,
            mint: *mint,
            source: *source,
            destination: *destination,
            token_program,
        }),
        data: solana_level_1_token_starter::instruction::TransferTokens { amount }.data(),
    }
}

fn burn_tokens_ix(
    program_id: solana_keypair::Address,
    token_program: solana_keypair::Address,
    authority: &solana_keypair::Address,
    mint: &solana_keypair::Address,
    token_account: &solana_keypair::Address,
    amount: u64,
) -> Instruction {
    Instruction {
        program_id,
        accounts: metas(solana_level_1_token_starter::accounts::BurnTokens {
            token_account: *token_account,
            mint: *mint,
            authority: *authority,
            token_program,
        }),
        data: solana_level_1_token_starter::instruction::BurnTokens { amount }.data(),
    }
}

fn unpack_mint(svm: &LiteSVM, mint: &solana_keypair::Address) -> (u8, u64, solana_keypair::Address) {
    use anchor_spl::token_interface::spl_token_2022::extension::StateWithExtensions;
    use anchor_spl::token_interface::spl_token_2022::state::Mint;

    let account = svm.get_account(mint).expect("mint must exist");
    let mint_data = StateWithExtensions::<Mint>::unpack(&account.data).expect("unpack mint");
    let authority = Option::<solana_keypair::Address>::from(mint_data.base.mint_authority)
        .expect("mint authority should be set");
    (mint_data.base.decimals, mint_data.base.supply, authority)
}

fn unpack_token(
    svm: &LiteSVM,
    token_account: &solana_keypair::Address,
) -> (solana_keypair::Address, solana_keypair::Address, u64) {
    use anchor_spl::token_interface::spl_token_2022::extension::StateWithExtensions;
    use anchor_spl::token_interface::spl_token_2022::state::Account as TokenAccount;

    let account = svm.get_account(token_account).expect("token account must exist");
    let data = StateWithExtensions::<TokenAccount>::unpack(&account.data).expect("unpack token");
    (data.base.mint, data.base.owner, data.base.amount)
}

struct Harness {
    svm: LiteSVM,
    program_id: solana_keypair::Address,
    token_program: solana_keypair::Address,
    payer: Keypair,
    authority: Keypair,
    mint: Keypair,
    alice: Keypair,
    bob: Keypair,
    alice_ata: solana_keypair::Address,
    bob_ata: solana_keypair::Address,
}

fn setup() -> Harness {
    let program_id = solana_level_1_token_starter::ID;
    let token_program = anchor_spl::token_2022::ID;
    let mut svm = LiteSVM::new();
    svm.add_program(program_id, &program_bytes())
        .expect("program must load");

    let payer = Keypair::new();
    let authority = Keypair::new();
    let mint = Keypair::new();
    let alice = Keypair::new();
    let bob = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000)
        .expect("airdrop payer");
    svm.airdrop(&alice.pubkey(), 1_000_000_000)
        .expect("airdrop alice");
    svm.airdrop(&bob.pubkey(), 1_000_000_000)
        .expect("airdrop bob");

    let alice_ata = ata(&alice.pubkey(), &mint.pubkey());
    let bob_ata = ata(&bob.pubkey(), &mint.pubkey());

    Harness {
        svm,
        program_id,
        token_program,
        payer,
        authority,
        mint,
        alice,
        bob,
        alice_ata,
        bob_ata,
    }
}

#[test]
fn test_token_lifecycle_end_to_end() {
    let mut h = setup();

    send(
        &mut h.svm,
        &h.payer,
        &[&h.authority, &h.mint],
        create_token_ix(
            h.program_id,
            h.token_program,
            &h.payer.pubkey(),
            &h.authority.pubkey(),
            &h.mint.pubkey(),
        ),
    )
    .expect("create_token failed");

    let mint_account = h.svm.get_account(&h.mint.pubkey()).expect("mint missing");
    assert_eq!(mint_account.owner, h.token_program);
    let (decimals, supply, mint_authority) = unpack_mint(&h.svm, &h.mint.pubkey());
    assert_eq!(decimals, DECIMALS);
    assert_eq!(supply, 0);
    assert_eq!(mint_authority, h.authority.pubkey());

    send(
        &mut h.svm,
        &h.payer,
        &[],
        create_token_account_ix(
            h.program_id,
            h.token_program,
            &h.payer.pubkey(),
            &h.alice.pubkey(),
            &h.mint.pubkey(),
        ),
    )
    .expect("create_token_account alice failed");
    send(
        &mut h.svm,
        &h.payer,
        &[],
        create_token_account_ix(
            h.program_id,
            h.token_program,
            &h.payer.pubkey(),
            &h.bob.pubkey(),
            &h.mint.pubkey(),
        ),
    )
    .expect("create_token_account bob failed");

    let alice_token_account = h.svm.get_account(&h.alice_ata).expect("alice ata missing");
    assert_eq!(alice_token_account.owner, h.token_program);
    let (alice_mint, alice_owner, alice_amount) = unpack_token(&h.svm, &h.alice_ata);
    assert_eq!(alice_mint, h.mint.pubkey());
    assert_eq!(alice_owner, h.alice.pubkey());
    assert_eq!(alice_amount, 0);

    let bob_token_account = h.svm.get_account(&h.bob_ata).expect("bob ata missing");
    assert_eq!(bob_token_account.owner, h.token_program);
    let (bob_mint, bob_owner, bob_amount) = unpack_token(&h.svm, &h.bob_ata);
    assert_eq!(bob_mint, h.mint.pubkey());
    assert_eq!(bob_owner, h.bob.pubkey());
    assert_eq!(bob_amount, 0);

    send(
        &mut h.svm,
        &h.payer,
        &[&h.authority],
        mint_tokens_ix(
            h.program_id,
            h.token_program,
            &h.authority.pubkey(),
            &h.mint.pubkey(),
            &h.alice_ata,
            MINT_AMOUNT,
        ),
    )
    .expect("mint_tokens failed");

    let (_, supply_after_mint, _) = unpack_mint(&h.svm, &h.mint.pubkey());
    let (_, _, alice_after_mint) = unpack_token(&h.svm, &h.alice_ata);
    assert_eq!(supply_after_mint, MINT_AMOUNT);
    assert_eq!(alice_after_mint, MINT_AMOUNT);

    send(
        &mut h.svm,
        &h.payer,
        &[&h.alice],
        transfer_tokens_ix(
            h.program_id,
            h.token_program,
            &h.alice.pubkey(),
            &h.mint.pubkey(),
            &h.alice_ata,
            &h.bob_ata,
            TRANSFER_AMOUNT,
        ),
    )
    .expect("transfer_tokens failed");

    let (_, supply_after_transfer, _) = unpack_mint(&h.svm, &h.mint.pubkey());
    let (_, _, alice_after_transfer) = unpack_token(&h.svm, &h.alice_ata);
    let (_, _, bob_after_transfer) = unpack_token(&h.svm, &h.bob_ata);
    assert_eq!(supply_after_transfer, MINT_AMOUNT);
    assert_eq!(alice_after_transfer, MINT_AMOUNT - TRANSFER_AMOUNT);
    assert_eq!(bob_after_transfer, TRANSFER_AMOUNT);
}

#[test]
fn test_negative_scenarios_failures() {
    let mut h = setup();

    send(
        &mut h.svm,
        &h.payer,
        &[&h.authority, &h.mint],
        create_token_ix(
            h.program_id,
            h.token_program,
            &h.payer.pubkey(),
            &h.authority.pubkey(),
            &h.mint.pubkey(),
        ),
    )
    .expect("create_token failed");
    send(
        &mut h.svm,
        &h.payer,
        &[],
        create_token_account_ix(
            h.program_id,
            h.token_program,
            &h.payer.pubkey(),
            &h.alice.pubkey(),
            &h.mint.pubkey(),
        ),
    )
    .expect("create alice ata failed");
    send(
        &mut h.svm,
        &h.payer,
        &[],
        create_token_account_ix(
            h.program_id,
            h.token_program,
            &h.payer.pubkey(),
            &h.bob.pubkey(),
            &h.mint.pubkey(),
        ),
    )
    .expect("create bob ata failed");

    let zero_mint = send(
        &mut h.svm,
        &h.payer,
        &[&h.authority],
        mint_tokens_ix(
            h.program_id,
            h.token_program,
            &h.authority.pubkey(),
            &h.mint.pubkey(),
            &h.alice_ata,
            0,
        ),
    );
    assert_eq!(zero_mint.is_ok(), false, "zero mint amount must fail");
    assert_eq!(unpack_mint(&h.svm, &h.mint.pubkey()).1, 0);
    assert_eq!(unpack_token(&h.svm, &h.alice_ata).2, 0);

    send(
        &mut h.svm,
        &h.payer,
        &[&h.authority],
        mint_tokens_ix(
            h.program_id,
            h.token_program,
            &h.authority.pubkey(),
            &h.mint.pubkey(),
            &h.alice_ata,
            MINT_AMOUNT,
        ),
    )
    .expect("mint setup for failures");
    assert_eq!(unpack_mint(&h.svm, &h.mint.pubkey()).1, MINT_AMOUNT);
    assert_eq!(unpack_token(&h.svm, &h.alice_ata).2, MINT_AMOUNT);
    assert_eq!(unpack_token(&h.svm, &h.bob_ata).2, 0);

    let snapshot_supply = unpack_mint(&h.svm, &h.mint.pubkey()).1;
    let snapshot_alice = unpack_token(&h.svm, &h.alice_ata).2;
    let snapshot_bob = unpack_token(&h.svm, &h.bob_ata).2;

    let zero_transfer = send(
        &mut h.svm,
        &h.payer,
        &[&h.alice],
        transfer_tokens_ix(
            h.program_id,
            h.token_program,
            &h.alice.pubkey(),
            &h.mint.pubkey(),
            &h.alice_ata,
            &h.bob_ata,
            0,
        ),
    );
    assert_eq!(zero_transfer.is_ok(), false, "zero transfer amount must fail");
    assert_eq!(unpack_mint(&h.svm, &h.mint.pubkey()).1, snapshot_supply);
    assert_eq!(unpack_token(&h.svm, &h.alice_ata).2, snapshot_alice);
    assert_eq!(unpack_token(&h.svm, &h.bob_ata).2, snapshot_bob);

    let stranger = Keypair::new();
    let wrong_mint_authority = send(
        &mut h.svm,
        &h.payer,
        &[&stranger],
        mint_tokens_ix(
            h.program_id,
            h.token_program,
            &stranger.pubkey(),
            &h.mint.pubkey(),
            &h.alice_ata,
            1,
        ),
    );
    assert_eq!(
        wrong_mint_authority.is_ok(),
        false,
        "wrong mint authority must fail"
    );
    assert_eq!(unpack_mint(&h.svm, &h.mint.pubkey()).1, snapshot_supply);
    assert_eq!(unpack_token(&h.svm, &h.alice_ata).2, snapshot_alice);

    let wrong_transfer_authority = send(
        &mut h.svm,
        &h.payer,
        &[&h.bob],
        transfer_tokens_ix(
            h.program_id,
            h.token_program,
            &h.bob.pubkey(),
            &h.mint.pubkey(),
            &h.alice_ata,
            &h.bob_ata,
            1,
        ),
    );
    assert_eq!(
        wrong_transfer_authority.is_ok(),
        false,
        "wrong transfer authority must fail"
    );
    assert_eq!(unpack_mint(&h.svm, &h.mint.pubkey()).1, snapshot_supply);
    assert_eq!(unpack_token(&h.svm, &h.alice_ata).2, snapshot_alice);
    assert_eq!(unpack_token(&h.svm, &h.bob_ata).2, snapshot_bob);

    let other_mint = Keypair::new();
    send(
        &mut h.svm,
        &h.payer,
        &[&h.authority, &other_mint],
        create_token_ix(
            h.program_id,
            h.token_program,
            &h.payer.pubkey(),
            &h.authority.pubkey(),
            &other_mint.pubkey(),
        ),
    )
    .expect("create other mint");
    let wrong_mint_destination = send(
        &mut h.svm,
        &h.payer,
        &[&h.authority],
        mint_tokens_ix(
            h.program_id,
            h.token_program,
            &h.authority.pubkey(),
            &other_mint.pubkey(),
            &h.alice_ata,
            1,
        ),
    );
    assert_eq!(
        wrong_mint_destination.is_ok(),
        false,
        "wrong mint for destination must fail"
    );
    let wrong_mint_transfer = send(
        &mut h.svm,
        &h.payer,
        &[&h.alice],
        transfer_tokens_ix(
            h.program_id,
            h.token_program,
            &h.alice.pubkey(),
            &other_mint.pubkey(),
            &h.alice_ata,
            &h.bob_ata,
            1,
        ),
    );
    assert_eq!(
        wrong_mint_transfer.is_ok(),
        false,
        "wrong mint for transfer must fail"
    );
    assert_eq!(unpack_mint(&h.svm, &h.mint.pubkey()).1, snapshot_supply);
    assert_eq!(unpack_token(&h.svm, &h.alice_ata).2, snapshot_alice);
    assert_eq!(unpack_token(&h.svm, &h.bob_ata).2, snapshot_bob);

    let same_accounts = send(
        &mut h.svm,
        &h.payer,
        &[&h.alice],
        transfer_tokens_ix(
            h.program_id,
            h.token_program,
            &h.alice.pubkey(),
            &h.mint.pubkey(),
            &h.alice_ata,
            &h.alice_ata,
            1,
        ),
    );
    assert_eq!(
        same_accounts.is_ok(),
        false,
        "same source and destination must fail"
    );
    assert_eq!(unpack_mint(&h.svm, &h.mint.pubkey()).1, snapshot_supply);
    assert_eq!(unpack_token(&h.svm, &h.alice_ata).2, snapshot_alice);
    assert_eq!(unpack_token(&h.svm, &h.bob_ata).2, snapshot_bob);
}

#[test]
fn test_burn_tokens_reduces_balance() {
    let mut h = setup();

    send(
        &mut h.svm,
        &h.payer,
        &[&h.authority, &h.mint],
        create_token_ix(
            h.program_id,
            h.token_program,
            &h.payer.pubkey(),
            &h.authority.pubkey(),
            &h.mint.pubkey(),
        ),
    )
    .expect("create_token failed");
    send(
        &mut h.svm,
        &h.payer,
        &[],
        create_token_account_ix(
            h.program_id,
            h.token_program,
            &h.payer.pubkey(),
            &h.alice.pubkey(),
            &h.mint.pubkey(),
        ),
    )
    .expect("create alice ata failed");
    send(
        &mut h.svm,
        &h.payer,
        &[&h.authority],
        mint_tokens_ix(
            h.program_id,
            h.token_program,
            &h.authority.pubkey(),
            &h.mint.pubkey(),
            &h.alice_ata,
            MINT_AMOUNT,
        ),
    )
    .expect("mint_tokens failed");

    assert_eq!(unpack_token(&h.svm, &h.alice_ata).2, MINT_AMOUNT);
    assert_eq!(unpack_mint(&h.svm, &h.mint.pubkey()).1, MINT_AMOUNT);

    send(
        &mut h.svm,
        &h.payer,
        &[&h.alice],
        burn_tokens_ix(
            h.program_id,
            h.token_program,
            &h.alice.pubkey(),
            &h.mint.pubkey(),
            &h.alice_ata,
            BURN_AMOUNT,
        ),
    )
    .expect("burn_tokens failed");

    assert_eq!(
        unpack_token(&h.svm, &h.alice_ata).2,
        MINT_AMOUNT - BURN_AMOUNT
    );
    assert_eq!(
        unpack_mint(&h.svm, &h.mint.pubkey()).1,
        MINT_AMOUNT - BURN_AMOUNT
    );
}
