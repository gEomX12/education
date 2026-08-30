use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use escrow::{EscrowState, EscrowStatus, ESCROW_SEED};
use litesvm::LiteSVM;
use solana_keypair::Keypair;
use solana_message::{AccountMeta, Instruction, Message};
use solana_signer::Signer;
use solana_transaction::Transaction;
use std::{fs, path::PathBuf};

const DECIMALS: u8 = 6;
const MINT_AMOUNT: u64 = 1_000_000;
const DEAL_AMOUNT: u64 = 400_000;
const DEAL_ID: u64 = 1;

fn read_so(name: &str) -> Vec<u8> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        manifest_dir.join(format!("../../target/deploy/{name}")),
        manifest_dir.join(format!("target/deploy/{name}")),
        PathBuf::from(format!("target/deploy/{name}")),
    ];
    let path = candidates.into_iter().find(|path| path.exists()).unwrap_or_else(|| {
        panic!("Build programs before running tests. Could not find {name}")
    });
    fs::read(&path).unwrap_or_else(|error| {
        panic!("Could not read {}: {error}", path.display())
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

fn escrow_pda(
    sender: &solana_keypair::Address,
    deal_id: u64,
) -> solana_keypair::Address {
    let (pda, _) = solana_keypair::Address::find_program_address(
        &[ESCROW_SEED, sender.as_ref(), &deal_id.to_le_bytes()],
        &escrow::ID,
    );
    pda
}

fn vault_ata(escrow: &solana_keypair::Address, mint: &solana_keypair::Address) -> solana_keypair::Address {
    ata(escrow, mint)
}

fn unpack_token(svm: &LiteSVM, token_account: &solana_keypair::Address) -> u64 {
    use anchor_spl::token_interface::spl_token_2022::extension::StateWithExtensions;
    use anchor_spl::token_interface::spl_token_2022::state::Account as TokenAccount;

    let account = svm.get_account(token_account).expect("token account must exist");
    let data = StateWithExtensions::<TokenAccount>::unpack(&account.data).expect("unpack token");
    data.base.amount
}

fn unpack_escrow(svm: &LiteSVM, pda: &solana_keypair::Address) -> EscrowState {
    let account = svm.get_account(pda).expect("escrow must exist");
    EscrowState::try_deserialize(&mut account.data.as_slice()).expect("deserialize escrow")
}

fn is_closed(svm: &LiteSVM, key: &solana_keypair::Address) -> bool {
    match svm.get_account(key) {
        None => true,
        Some(account) => account.lamports == 0,
    }
}

fn lamports(svm: &LiteSVM, key: &solana_keypair::Address) -> u64 {
    svm.get_account(key).map(|account| account.lamports).unwrap_or(0)
}

fn create_token_ix(h: &Harness) -> Instruction {
    Instruction {
        program_id: h.token_starter,
        accounts: metas(solana_level_1_token_starter::accounts::CreateToken {
            payer: h.payer.pubkey(),
            authority: h.mint_authority.pubkey(),
            mint: h.mint.pubkey(),
            token_program: h.token_program,
            system_program: anchor_lang::system_program::ID,
        }),
        data: solana_level_1_token_starter::instruction::CreateToken { decimals: DECIMALS }
            .data(),
    }
}

fn create_ata_ix(h: &Harness, owner: &solana_keypair::Address) -> Instruction {
    Instruction {
        program_id: h.token_starter,
        accounts: metas(solana_level_1_token_starter::accounts::CreateTokenAccount {
            payer: h.payer.pubkey(),
            owner: *owner,
            mint: h.mint.pubkey(),
            token_account: ata(owner, &h.mint.pubkey()),
            token_program: h.token_program,
            associated_token_program: anchor_spl::associated_token::ID,
            system_program: anchor_lang::system_program::ID,
        }),
        data: solana_level_1_token_starter::instruction::CreateTokenAccount {}.data(),
    }
}

fn mint_tokens_ix(h: &Harness, destination: &solana_keypair::Address, amount: u64) -> Instruction {
    Instruction {
        program_id: h.token_starter,
        accounts: metas(solana_level_1_token_starter::accounts::MintTokens {
            authority: h.mint_authority.pubkey(),
            mint: h.mint.pubkey(),
            destination: *destination,
            token_program: h.token_program,
        }),
        data: solana_level_1_token_starter::instruction::MintTokens { amount }.data(),
    }
}

fn initialize_ix(h: &Harness, deal_id: u64, amount: u64) -> Instruction {
    let escrow = escrow_pda(&h.sender.pubkey(), deal_id);
    Instruction {
        program_id: h.escrow_program,
        accounts: metas(escrow::accounts::Initialize {
            sender: h.sender.pubkey(),
            receiver: h.receiver.pubkey(),
            mint: h.mint.pubkey(),
            escrow,
            vault: vault_ata(&escrow, &h.mint.pubkey()),
            token_program: h.token_program,
            associated_token_program: anchor_spl::associated_token::ID,
            system_program: anchor_lang::system_program::ID,
        }),
        data: escrow::instruction::Initialize { deal_id, amount }.data(),
    }
}

fn deposit_ix(h: &Harness, deal_id: u64) -> Instruction {
    let escrow = escrow_pda(&h.sender.pubkey(), deal_id);
    Instruction {
        program_id: h.escrow_program,
        accounts: metas(escrow::accounts::Deposit {
            sender: h.sender.pubkey(),
            escrow,
            mint: h.mint.pubkey(),
            sender_token: h.sender_ata,
            vault: vault_ata(&escrow, &h.mint.pubkey()),
            token_program: h.token_program,
        }),
        data: escrow::instruction::Deposit { deal_id }.data(),
    }
}

fn release_ix(h: &Harness, deal_id: u64) -> Instruction {
    let escrow = escrow_pda(&h.sender.pubkey(), deal_id);
    Instruction {
        program_id: h.escrow_program,
        accounts: metas(escrow::accounts::Release {
            sender: h.sender.pubkey(),
            receiver: h.receiver.pubkey(),
            escrow,
            mint: h.mint.pubkey(),
            vault: vault_ata(&escrow, &h.mint.pubkey()),
            receiver_token: h.receiver_ata,
            token_program: h.token_program,
        }),
        data: escrow::instruction::Release { deal_id }.data(),
    }
}

fn cancel_ix(h: &Harness, deal_id: u64) -> Instruction {
    let escrow = escrow_pda(&h.sender.pubkey(), deal_id);
    Instruction {
        program_id: h.escrow_program,
        accounts: metas(escrow::accounts::Cancel {
            sender: h.sender.pubkey(),
            escrow,
            mint: h.mint.pubkey(),
            sender_token: h.sender_ata,
            vault: vault_ata(&escrow, &h.mint.pubkey()),
            token_program: h.token_program,
        }),
        data: escrow::instruction::Cancel { deal_id }.data(),
    }
}

struct Harness {
    svm: LiteSVM,
    escrow_program: solana_keypair::Address,
    token_starter: solana_keypair::Address,
    token_program: solana_keypair::Address,
    payer: Keypair,
    mint_authority: Keypair,
    mint: Keypair,
    sender: Keypair,
    receiver: Keypair,
    attacker: Keypair,
    sender_ata: solana_keypair::Address,
    receiver_ata: solana_keypair::Address,
}

fn setup_with_mint_amount(mint_amount: u64) -> Harness {
    let escrow_program = escrow::ID;
    let token_starter = solana_level_1_token_starter::ID;
    let token_program = anchor_spl::token_2022::ID;
    let mut svm = LiteSVM::new();
    svm.add_program(escrow_program, &read_so("escrow.so"))
        .expect("escrow program must load");
    svm.add_program(token_starter, &read_so("solana_level_1_token_starter.so"))
        .expect("token starter must load");

    let payer = Keypair::new();
    let mint_authority = Keypair::new();
    let mint = Keypair::new();
    let sender = Keypair::new();
    let receiver = Keypair::new();
    let attacker = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).expect("airdrop payer");
    svm.airdrop(&sender.pubkey(), 2_000_000_000).expect("airdrop sender");
    svm.airdrop(&receiver.pubkey(), 1_000_000_000).expect("airdrop receiver");
    svm.airdrop(&attacker.pubkey(), 1_000_000_000).expect("airdrop attacker");

    let sender_ata = ata(&sender.pubkey(), &mint.pubkey());
    let receiver_ata = ata(&receiver.pubkey(), &mint.pubkey());

    let mut h = Harness {
        svm,
        escrow_program,
        token_starter,
        token_program,
        payer,
        mint_authority,
        mint,
        sender,
        receiver,
        attacker,
        sender_ata,
        receiver_ata,
    };

    let create_mint = create_token_ix(&h);
    send(&mut h.svm, &h.payer, &[&h.mint_authority, &h.mint], create_mint).expect("create mint");
    let sender_ata_ix = create_ata_ix(&h, &h.sender.pubkey());
    send(&mut h.svm, &h.payer, &[], sender_ata_ix).expect("sender ata");
    let receiver_ata_ix = create_ata_ix(&h, &h.receiver.pubkey());
    send(&mut h.svm, &h.payer, &[], receiver_ata_ix).expect("receiver ata");
    if mint_amount > 0 {
        let mint_ix = mint_tokens_ix(&h, &h.sender_ata, mint_amount);
        send(&mut h.svm, &h.payer, &[&h.mint_authority], mint_ix).expect("mint to sender");
    }
    h
}

fn setup() -> Harness {
    setup_with_mint_amount(MINT_AMOUNT)
}

fn initialize_deal(h: &mut Harness, deal_id: u64, amount: u64) -> Result<(), String> {
    let ix = initialize_ix(h, deal_id, amount);
    send(&mut h.svm, &h.payer, &[&h.sender], ix)
}

fn deposit_deal(h: &mut Harness, deal_id: u64) -> Result<(), String> {
    let ix = deposit_ix(h, deal_id);
    send(&mut h.svm, &h.payer, &[&h.sender], ix)
}

fn release_deal(h: &mut Harness, deal_id: u64) -> Result<(), String> {
    let ix = release_ix(h, deal_id);
    send(&mut h.svm, &h.payer, &[&h.sender], ix)
}

fn cancel_deal(h: &mut Harness, deal_id: u64) -> Result<(), String> {
    let ix = cancel_ix(h, deal_id);
    send(&mut h.svm, &h.payer, &[&h.sender], ix)
}

fn fund_deal(h: &mut Harness, deal_id: u64, amount: u64) {
    initialize_deal(h, deal_id, amount).expect("initialize");
    deposit_deal(h, deal_id).expect("deposit");
}

#[test]
fn test_initialize_deposit_release_transfers_tokens_and_refunds_rent() {
    let mut h = setup();
    let pda = escrow_pda(&h.sender.pubkey(), DEAL_ID);
    let vault = vault_ata(&pda, &h.mint.pubkey());

    initialize_deal(&mut h, DEAL_ID, DEAL_AMOUNT).expect("initialize");
    let state = unpack_escrow(&h.svm, &pda);
    assert_eq!(state.sender, h.sender.pubkey());
    assert_eq!(state.receiver, h.receiver.pubkey());
    assert_eq!(state.mint, h.mint.pubkey());
    assert_eq!(state.amount, DEAL_AMOUNT);
    assert_eq!(state.deal_id, DEAL_ID);
    assert_eq!(state.status, EscrowStatus::Created);
    assert_eq!(unpack_token(&h.svm, &vault), 0);

    deposit_deal(&mut h, DEAL_ID).expect("deposit");
    assert_eq!(unpack_escrow(&h.svm, &pda).status, EscrowStatus::Funded);
    assert_eq!(unpack_token(&h.svm, &h.sender_ata), MINT_AMOUNT - DEAL_AMOUNT);
    assert_eq!(unpack_token(&h.svm, &vault), DEAL_AMOUNT);
    assert_eq!(unpack_token(&h.svm, &h.receiver_ata), 0);

    let escrow_rent = lamports(&h.svm, &pda);
    let vault_rent = lamports(&h.svm, &vault);
    let sender_before = lamports(&h.svm, &h.sender.pubkey());
    assert!(escrow_rent > 0);
    assert!(vault_rent > 0);

    release_deal(&mut h, DEAL_ID).expect("release");

    assert!(is_closed(&h.svm, &pda), "escrow state must be closed");
    assert!(is_closed(&h.svm, &vault), "vault must be closed");
    assert_eq!(unpack_token(&h.svm, &h.sender_ata), MINT_AMOUNT - DEAL_AMOUNT);
    assert_eq!(unpack_token(&h.svm, &h.receiver_ata), DEAL_AMOUNT);
    assert_eq!(
        lamports(&h.svm, &h.sender.pubkey()),
        sender_before + escrow_rent + vault_rent,
        "release must return vault and EscrowState rent to sender"
    );
}

#[test]
fn test_initialize_deposit_cancel_returns_tokens_to_sender() {
    let mut h = setup();
    let pda = escrow_pda(&h.sender.pubkey(), DEAL_ID);
    let vault = vault_ata(&pda, &h.mint.pubkey());

    fund_deal(&mut h, DEAL_ID, DEAL_AMOUNT);
    assert_eq!(unpack_token(&h.svm, &h.sender_ata), MINT_AMOUNT - DEAL_AMOUNT);
    assert_eq!(unpack_token(&h.svm, &vault), DEAL_AMOUNT);

    let escrow_rent = lamports(&h.svm, &pda);
    let vault_rent = lamports(&h.svm, &vault);
    let sender_before = lamports(&h.svm, &h.sender.pubkey());

    cancel_deal(&mut h, DEAL_ID).expect("cancel");

    assert!(is_closed(&h.svm, &pda), "escrow state must be closed");
    assert!(is_closed(&h.svm, &vault), "vault must be closed");
    assert_eq!(unpack_token(&h.svm, &h.sender_ata), MINT_AMOUNT);
    assert_eq!(unpack_token(&h.svm, &h.receiver_ata), 0);
    assert_eq!(
        lamports(&h.svm, &h.sender.pubkey()),
        sender_before + escrow_rent + vault_rent,
        "cancel must return vault and EscrowState rent to sender"
    );
}

#[test]
fn test_initialize_rejects_zero_amount() {
    let mut h = setup();
    let pda = escrow_pda(&h.sender.pubkey(), DEAL_ID);

    let result = initialize_deal(&mut h, DEAL_ID, 0);
    assert!(result.is_err(), "zero amount initialize must fail");
    assert!(
        h.svm.get_account(&pda).is_none(),
        "failed initialize must not leave escrow account"
    );
}

#[test]
fn test_release_and_cancel_cannot_replay() {
    let mut h = setup();
    let pda = escrow_pda(&h.sender.pubkey(), DEAL_ID);
    let vault = vault_ata(&pda, &h.mint.pubkey());

    fund_deal(&mut h, DEAL_ID, DEAL_AMOUNT);
    let sender_after_fund = unpack_token(&h.svm, &h.sender_ata);
    let receiver_after_fund = unpack_token(&h.svm, &h.receiver_ata);

    release_deal(&mut h, DEAL_ID).expect("first release");
    assert!(
        release_deal(&mut h, DEAL_ID).is_err(),
        "second release must fail"
    );
    assert!(
        cancel_deal(&mut h, DEAL_ID).is_err(),
        "cancel after release must fail"
    );
    assert_eq!(unpack_token(&h.svm, &h.sender_ata), sender_after_fund);
    assert_eq!(unpack_token(&h.svm, &h.receiver_ata), DEAL_AMOUNT);
    assert!(is_closed(&h.svm, &pda));
    assert!(is_closed(&h.svm, &vault));

    let mut h = setup();
    let second_id = DEAL_ID + 1;
    let pda = escrow_pda(&h.sender.pubkey(), second_id);
    fund_deal(&mut h, second_id, DEAL_AMOUNT);
    cancel_deal(&mut h, second_id).expect("first cancel");
    assert!(
        cancel_deal(&mut h, second_id).is_err(),
        "second cancel must fail"
    );
    assert!(
        release_deal(&mut h, second_id).is_err(),
        "release after cancel must fail"
    );
    assert_eq!(unpack_token(&h.svm, &h.sender_ata), MINT_AMOUNT);
    assert_eq!(unpack_token(&h.svm, &h.receiver_ata), 0);
    assert!(is_closed(&h.svm, &pda));
    let _ = receiver_after_fund;
}

#[test]
fn test_rejects_swapped_sender_or_receiver() {
    let mut h = setup();
    fund_deal(&mut h, DEAL_ID, DEAL_AMOUNT);
    let sender_before = unpack_token(&h.svm, &h.sender_ata);
    let receiver_before = unpack_token(&h.svm, &h.receiver_ata);
    let pda = escrow_pda(&h.sender.pubkey(), DEAL_ID);
    let vault = vault_ata(&pda, &h.mint.pubkey());
    let vault_before = unpack_token(&h.svm, &vault);

    let attacker_as_sender = {
        let mut ix = deposit_ix(&h, DEAL_ID);
        if let Some(meta) = ix.accounts.first_mut() {
            meta.pubkey = h.attacker.pubkey();
            meta.is_signer = true;
        }
        ix
    };
    assert!(
        send(&mut h.svm, &h.payer, &[&h.attacker], attacker_as_sender).is_err(),
        "deposit with swapped sender must fail"
    );

    let attacker_as_release_sender = {
        let mut ix = release_ix(&h, DEAL_ID);
        if let Some(meta) = ix.accounts.first_mut() {
            meta.pubkey = h.attacker.pubkey();
            meta.is_signer = true;
        }
        ix
    };
    assert!(
        send(&mut h.svm, &h.payer, &[&h.attacker], attacker_as_release_sender).is_err(),
        "release signed by attacker as sender must fail"
    );

    let swapped_receiver = {
        let mut ix = release_ix(&h, DEAL_ID);
        ix.accounts[1].pubkey = h.attacker.pubkey();
        ix
    };
    assert!(
        send(&mut h.svm, &h.payer, &[&h.sender], swapped_receiver).is_err(),
        "release with swapped receiver must fail"
    );

    let mut unsigned_sender = release_ix(&h, DEAL_ID);
    unsigned_sender.accounts[0].is_signer = false;
    assert!(
        send(&mut h.svm, &h.payer, &[], unsigned_sender).is_err(),
        "release must require sender signature, not receiver"
    );

    assert_eq!(unpack_token(&h.svm, &h.sender_ata), sender_before);
    assert_eq!(unpack_token(&h.svm, &h.receiver_ata), receiver_before);
    assert_eq!(unpack_token(&h.svm, &vault), vault_before);
    assert_eq!(unpack_escrow(&h.svm, &pda).status, EscrowStatus::Funded);
}

#[test]
fn test_deposit_rejects_insufficient_balance() {
    let mut h = setup_with_mint_amount(1);
    initialize_deal(&mut h, DEAL_ID, DEAL_AMOUNT).expect("initialize");
    let pda = escrow_pda(&h.sender.pubkey(), DEAL_ID);
    let vault = vault_ata(&pda, &h.mint.pubkey());

    assert!(
        deposit_deal(&mut h, DEAL_ID).is_err(),
        "deposit larger than sender balance must fail"
    );
    assert_eq!(unpack_token(&h.svm, &h.sender_ata), 1);
    assert_eq!(unpack_token(&h.svm, &vault), 0);
    assert_eq!(unpack_escrow(&h.svm, &pda).status, EscrowStatus::Created);
}

#[test]
fn test_initialize_rejects_duplicate_deal_id() {
    let mut h = setup();
    initialize_deal(&mut h, DEAL_ID, DEAL_AMOUNT).expect("first initialize");
    let pda = escrow_pda(&h.sender.pubkey(), DEAL_ID);
    let first = unpack_escrow(&h.svm, &pda);

    assert!(
        initialize_deal(&mut h, DEAL_ID, DEAL_AMOUNT + 1).is_err(),
        "same sender and deal_id must not reinitialize"
    );

    let second = unpack_escrow(&h.svm, &pda);
    assert_eq!(second.amount, first.amount);
    assert_eq!(second.status, EscrowStatus::Created);
    assert_eq!(second.receiver, first.receiver);
}

#[test]
fn test_initialize_rejects_sender_equals_receiver() {
    let mut h = setup();
    let pda = escrow_pda(&h.sender.pubkey(), DEAL_ID);
    let mut ix = initialize_ix(&h, DEAL_ID, DEAL_AMOUNT);
    ix.accounts[1].pubkey = h.sender.pubkey();

    assert!(
        send(&mut h.svm, &h.payer, &[&h.sender], ix).is_err(),
        "sender and receiver must differ"
    );
    assert!(h.svm.get_account(&pda).is_none());
}

#[test]
fn test_deposit_cannot_run_twice() {
    let mut h = setup();
    fund_deal(&mut h, DEAL_ID, DEAL_AMOUNT);
    let pda = escrow_pda(&h.sender.pubkey(), DEAL_ID);
    let vault = vault_ata(&pda, &h.mint.pubkey());

    assert!(
        deposit_deal(&mut h, DEAL_ID).is_err(),
        "second deposit must fail"
    );
    assert_eq!(unpack_token(&h.svm, &h.sender_ata), MINT_AMOUNT - DEAL_AMOUNT);
    assert_eq!(unpack_token(&h.svm, &vault), DEAL_AMOUNT);
    assert_eq!(unpack_escrow(&h.svm, &pda).status, EscrowStatus::Funded);
}
