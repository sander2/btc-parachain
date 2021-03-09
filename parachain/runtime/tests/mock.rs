extern crate hex;

pub use bitcoin::formatter::{Formattable, TryFormattable};
pub use bitcoin::types::*;
pub use btc_parachain_runtime::{AccountId, Call, Event, Runtime};
pub use btc_relay::{BtcAddress, BtcPublicKey};
pub use frame_support::{assert_noop, assert_ok};
pub use mocktopus::mocking::*;
use primitive_types::{H256, U256};
pub use security::{ErrorCode, StatusCode};
pub use sp_arithmetic::{FixedI128, FixedPointNumber, FixedU128};
pub use sp_core::H160;
pub use sp_runtime::traits::Dispatchable;
pub use sp_std::convert::TryInto;
pub use vault_registry::CurrencySource;

pub const ALICE: [u8; 32] = [0u8; 32];
pub const BOB: [u8; 32] = [1u8; 32];
pub const CAROL: [u8; 32] = [2u8; 32];

pub const LIQUIDATION_VAULT: [u8; 32] = [3u8; 32];
pub const FEE_POOL: [u8; 32] = [4u8; 32];
pub const MAINTAINER: [u8; 32] = [5u8; 32];

pub const FAUCET: [u8; 32] = [128u8; 32];

pub const INITIAL_BALANCE: u128 = 1_000_000_000_000;
pub const INITIAL_LIQUIDATION_VAULT_BALANCE: u128 = 1_000;

pub const CONFIRMATIONS: u32 = 6;

pub type BTCRelayCall = btc_relay::Call<Runtime>;
pub type BTCRelayModule = btc_relay::Module<Runtime>;
pub type BTCRelayError = btc_relay::Error<Runtime>;
pub type BTCRelayEvent = btc_relay::Event<Runtime>;

pub fn origin_of(account_id: AccountId) -> <Runtime as frame_system::Config>::Origin {
    <Runtime as frame_system::Config>::Origin::signed(account_id)
}

pub fn account_of(address: [u8; 32]) -> AccountId {
    AccountId::from(address)
}

#[derive(Debug, PartialEq, Default)]
pub struct UserData {
    pub free_balance: u128,
    pub locked_balance: u128,
    pub locked_tokens: u128,
    pub free_tokens: u128,
}

impl UserData {
    #[allow(dead_code)]
    pub fn get(id: [u8; 32]) -> Self {
        let account_id = account_of(id);
        Self {
            free_balance: CollateralModule::get_balance_from_account(&account_id),
            locked_balance: CollateralModule::get_collateral_from_account(&account_id),
            locked_tokens: TreasuryModule::get_locked_balance_from_account(account_id.clone()),
            free_tokens: TreasuryModule::get_balance_from_account(account_id.clone()),
        }
    }
    #[allow(dead_code)]
    pub fn force_to(id: [u8; 32], new: Self) -> Self {
        let old = Self::get(id.clone());
        let account_id = account_of(id);

        if old.free_tokens > new.free_tokens || old.locked_tokens > new.locked_tokens {
            unimplemented!()
        }

        // set free balance:
        CollateralModule::transfer(account_id.clone(), account_of(FAUCET), old.free_balance)
            .unwrap();
        CollateralModule::transfer(account_of(FAUCET), account_id.clone(), new.free_balance)
            .unwrap();

        // set locked balance:
        CollateralModule::slash_collateral(
            account_id.clone(),
            account_of(FAUCET),
            old.locked_balance,
        )
        .unwrap();
        CollateralModule::transfer(account_of(FAUCET), account_id.clone(), new.locked_balance)
            .unwrap();
        CollateralModule::lock_collateral(&account_id, new.locked_balance).unwrap();

        // set free_tokens
        TreasuryModule::mint(account_id.clone(), new.free_tokens - old.free_tokens);

        // set locked_tokens
        let locked_tokens_to_add = new.locked_tokens - old.locked_tokens;
        TreasuryModule::mint(account_id.clone(), locked_tokens_to_add);
        TreasuryModule::lock(account_id.clone(), locked_tokens_to_add).unwrap();

        // sanity check:
        assert_eq!(Self::get(id), new);

        new
    }
}

#[derive(Debug, PartialEq, Default)]
pub struct CoreVaultData {
    pub to_be_issued: u128,
    pub issued: u128,
    pub to_be_redeemed: u128,
    pub backing_collateral: u128,
    pub griefing_collateral: u128,
    pub free_balance: u128,
}

impl CoreVaultData {
    #[allow(dead_code)]
    pub fn vault(vault: [u8; 32]) -> Self {
        let account_id = account_of(vault);
        let vault = VaultRegistryModule::get_vault_from_id(&account_id).unwrap();
        Self {
            to_be_issued: vault.to_be_issued_tokens,
            issued: vault.issued_tokens,
            to_be_redeemed: vault.to_be_redeemed_tokens,
            backing_collateral: CurrencySource::<Runtime>::Backing(account_id.clone())
                .current_balance()
                .unwrap(),
            griefing_collateral: CurrencySource::<Runtime>::Griefing(account_id.clone())
                .current_balance()
                .unwrap(),
            free_balance: CollateralModule::get_balance_from_account(&account_id),
        }
    }
    #[allow(dead_code)]
    pub fn liquidation_vault() -> Self {
        let account_id = account_of(LIQUIDATION_VAULT);
        let vault = VaultRegistryModule::get_liquidation_vault();
        Self {
            to_be_issued: vault.to_be_issued_tokens,
            issued: vault.issued_tokens,
            to_be_redeemed: vault.to_be_redeemed_tokens,
            backing_collateral: CurrencySource::<Runtime>::LiquidationVault
                .current_balance()
                .unwrap(),
            griefing_collateral: 0,
            free_balance: CollateralModule::get_balance_from_account(&account_id),
        }
    }
    #[allow(dead_code)]
    pub fn force_to(vault: [u8; 32], state: CoreVaultData) {
        // register vault if not yet registered
        if let Err(_) = VaultRegistryModule::get_vault_from_id(&account_of(vault)) {
            assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
                100,
                dummy_public_key()
            ))
            .dispatch(origin_of(account_of(vault))));
        };

        // temporarily give vault a lot of backing collateral so we can set issued & to-be-issued to whatever we want
        VaultRegistryModule::slash_collateral(
            CurrencySource::FreeBalance(account_of(FAUCET)),
            CurrencySource::Backing(account_of(vault)),
            CollateralModule::get_balance_from_account(&account_of(FAUCET)),
        )
        .unwrap();

        let current = CoreVaultData::vault(vault);
        if current.to_be_issued < state.to_be_issued {
            assert_ok!(VaultRegistryModule::try_increase_to_be_issued_tokens(
                &account_of(vault),
                state.to_be_issued - current.to_be_issued
            ));
        }

        if current.issued < state.issued {
            assert_ok!(VaultRegistryModule::try_increase_to_be_issued_tokens(
                &account_of(vault),
                state.issued - current.issued
            ));

            assert_ok!(VaultRegistryModule::issue_tokens(
                &account_of(vault),
                state.issued - current.issued
            ));
        }
        if current.to_be_redeemed < state.to_be_redeemed {
            assert_ok!(VaultRegistryModule::try_increase_to_be_redeemed_tokens(
                &account_of(vault),
                state.to_be_redeemed - current.to_be_redeemed
            ));
        }

        // clear all balances
        VaultRegistryModule::slash_collateral(
            CurrencySource::Backing(account_of(vault)),
            CurrencySource::FreeBalance(account_of(FAUCET)),
            CurrencySource::<Runtime>::Backing(account_of(vault))
                .current_balance()
                .unwrap(),
        )
        .unwrap();
        VaultRegistryModule::slash_collateral(
            CurrencySource::Griefing(account_of(vault)),
            CurrencySource::FreeBalance(account_of(FAUCET)),
            CurrencySource::<Runtime>::Griefing(account_of(vault))
                .current_balance()
                .unwrap(),
        )
        .unwrap();
        VaultRegistryModule::slash_collateral(
            CurrencySource::FreeBalance(account_of(vault)),
            CurrencySource::FreeBalance(account_of(FAUCET)),
            CurrencySource::<Runtime>::FreeBalance(account_of(vault))
                .current_balance()
                .unwrap(),
        )
        .unwrap();

        // now set balances to desired values
        VaultRegistryModule::slash_collateral(
            CurrencySource::FreeBalance(account_of(FAUCET)),
            CurrencySource::Backing(account_of(vault)),
            state.backing_collateral,
        )
        .unwrap();
        VaultRegistryModule::slash_collateral(
            CurrencySource::FreeBalance(account_of(FAUCET)),
            CurrencySource::Griefing(account_of(vault)),
            state.griefing_collateral,
        )
        .unwrap();
        VaultRegistryModule::slash_collateral(
            CurrencySource::FreeBalance(account_of(FAUCET)),
            CurrencySource::FreeBalance(account_of(vault)),
            state.free_balance,
        )
        .unwrap();

        // since the function is only partially implemented, check that we achieved the
        // desired stae
        assert_eq!(CoreVaultData::vault(vault), state);
    }
}
#[allow(dead_code)]
pub fn drop_exchange_rate_and_liquidate(vault: [u8; 32]) {
    assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
        FixedU128::checked_from_integer(1_0000_000_000).unwrap()
    ));
    assert_ok!(VaultRegistryModule::liquidate_vault(&account_of(vault)));
}

#[allow(dead_code)]
pub fn set_default_thresholds() {
    let secure = FixedU128::checked_from_rational(150, 100).unwrap();
    let auction = FixedU128::checked_from_rational(120, 100).unwrap();
    let premium = FixedU128::checked_from_rational(135, 100).unwrap();
    let liquidation = FixedU128::checked_from_rational(110, 100).unwrap();

    VaultRegistryModule::set_secure_collateral_threshold(secure);
    VaultRegistryModule::set_auction_collateral_threshold(auction);
    VaultRegistryModule::set_premium_redeem_threshold(premium);
    VaultRegistryModule::set_liquidation_collateral_threshold(liquidation);
}

pub fn dummy_public_key() -> BtcPublicKey {
    BtcPublicKey([
        2, 205, 114, 218, 156, 16, 235, 172, 106, 37, 18, 153, 202, 140, 176, 91, 207, 51, 187, 55,
        18, 45, 222, 180, 119, 54, 243, 97, 173, 150, 161, 169, 230,
    ])
}

#[allow(dead_code)]
pub fn increase_issued(user: [u8; 32], vault: [u8; 32], collateral: u128, tokens: u128) {
    // register the vault
    assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
        collateral,
        dummy_public_key()
    ))
    .dispatch(origin_of(account_of(vault))));

    // increase to be issued tokens
    assert_ok!(VaultRegistryModule::try_increase_to_be_issued_tokens(
        &account_of(vault),
        tokens
    ));

    // issue tokens
    assert_ok!(VaultRegistryModule::issue_tokens(
        &account_of(vault),
        tokens
    ));

    // mint tokens to the user
    treasury::Module::<Runtime>::mint(user.into(), tokens);
}

#[allow(dead_code)]
pub fn required_collateral_for_issue(issue_btc: u128) -> u128 {
    let fee_amount_btc = FeeModule::get_issue_fee(issue_btc).unwrap();
    let total_amount_btc = issue_btc + fee_amount_btc;
    let collateral_vault =
        VaultRegistryModule::get_required_collateral_for_polkabtc(total_amount_btc).unwrap();
    collateral_vault
}

pub fn assert_store_main_chain_header_event(height: u32, hash: H256Le, relayer: AccountId) {
    let store_event = Event::btc_relay(BTCRelayEvent::StoreMainChainHeader(height, hash, relayer));
    let events = SystemModule::events();

    // store only main chain header
    assert!(events.iter().any(|a| a.event == store_event));
}

#[allow(dead_code)]
pub fn generate_transaction_and_mine(
    address: BtcAddress,
    amount: u128,
    return_data: Option<H256>,
) -> (H256Le, u32, Vec<u8>, Vec<u8>) {
    generate_transaction_and_mine_with_script_sig(
        address,
        amount,
        return_data,
        &[
            0, 71, 48, 68, 2, 32, 91, 128, 41, 150, 96, 53, 187, 63, 230, 129, 53, 234, 210, 186,
            21, 187, 98, 38, 255, 112, 30, 27, 228, 29, 132, 140, 155, 62, 123, 216, 232, 168, 2,
            32, 72, 126, 179, 207, 142, 8, 99, 8, 32, 78, 244, 166, 106, 160, 207, 227, 61, 210,
            172, 234, 234, 93, 59, 159, 79, 12, 194, 240, 212, 3, 120, 50, 1, 71, 81, 33, 3, 113,
            209, 131, 177, 9, 29, 242, 229, 15, 217, 247, 165, 78, 111, 80, 79, 50, 200, 117, 80,
            30, 233, 210, 167, 133, 175, 62, 253, 134, 127, 212, 51, 33, 2, 128, 200, 184, 235,
            148, 25, 43, 34, 28, 173, 55, 54, 189, 164, 187, 243, 243, 152, 7, 84, 210, 85, 156,
            238, 77, 97, 188, 240, 162, 197, 105, 62, 82, 174,
        ],
    )
}

#[allow(dead_code)]
pub fn generate_transaction_and_mine_with_script_sig(
    address: BtcAddress,
    amount: u128,
    return_data: Option<H256>,
    script: &[u8],
) -> (H256Le, u32, Vec<u8>, Vec<u8>) {
    let mut height = 1;
    let confirmations = 6;

    // initialize BTC Relay with one block
    let init_block = BlockBuilder::new()
        .with_version(2)
        .with_coinbase(&address, 50, 3)
        .with_timestamp(1588813835)
        .mine(U256::from(2).pow(254.into()))
        .unwrap();

    let raw_init_block_header =
        RawBlockHeader::from_bytes(&init_block.header.try_format().unwrap())
            .expect("could not serialize block header");

    match Call::BTCRelay(BTCRelayCall::initialize(
        raw_init_block_header.try_into().expect("bad block header"),
        height,
    ))
    .dispatch(origin_of(account_of(ALICE)))
    {
        Ok(_) => {}
        Err(e) if e == BTCRelayError::AlreadyInitialized.into() => {}
        _ => panic!("Failed to initialize btc relay"),
    }

    height = BTCRelayModule::get_best_block_height() + 1;

    let value = amount as i64;
    let mut transaction_builder = TransactionBuilder::new();
    transaction_builder.with_version(2);
    transaction_builder.add_input(
        TransactionInputBuilder::new()
            .with_coinbase(false)
            .with_script(script)
            .with_previous_hash(init_block.transactions[0].hash())
            .build(),
    );

    transaction_builder.add_output(TransactionOutput::payment(value.into(), &address));
    if let Some(op_return_data) = return_data {
        transaction_builder.add_output(TransactionOutput::op_return(0, op_return_data.as_bytes()));
    }

    let transaction = transaction_builder.build();

    let prev_hash = BTCRelayModule::get_best_block();
    let block = BlockBuilder::new()
        .with_previous_hash(prev_hash)
        .with_version(2)
        .with_coinbase(&address, 50, 3)
        .with_timestamp(1588814835)
        .add_transaction(transaction.clone())
        .mine(U256::from(2).pow(254.into()))
        .unwrap();

    let raw_block_header = RawBlockHeader::from_bytes(&block.header.try_format().unwrap())
        .expect("could not serialize block header");

    let tx_id = transaction.tx_id();
    let tx_block_height = height;
    let proof = block.merkle_proof(&vec![tx_id]).unwrap();
    let bytes_proof = proof.try_format().unwrap();
    let raw_tx = transaction.format_with(true);

    assert_ok!(Call::BTCRelay(BTCRelayCall::store_block_header(
        raw_block_header.try_into().expect("bad block header")
    ))
    .dispatch(origin_of(account_of(ALICE))));
    assert_store_main_chain_header_event(height, block.header.hash().unwrap(), account_of(ALICE));

    // Mine six new blocks to get over required confirmations
    let mut prev_block_hash = block.header.hash().unwrap();
    let mut timestamp = 1588814835;
    for _ in 0..confirmations {
        height += 1;
        timestamp += 1000;
        let conf_block = BlockBuilder::new()
            .with_previous_hash(prev_block_hash)
            .with_version(2)
            .with_coinbase(&address, 50, 3)
            .with_timestamp(timestamp)
            .mine(U256::from(2).pow(254.into()))
            .unwrap();

        let raw_conf_block_header =
            RawBlockHeader::from_bytes(&conf_block.header.try_format().unwrap())
                .expect("could not serialize block header");
        assert_ok!(Call::BTCRelay(BTCRelayCall::store_block_header(
            raw_conf_block_header.try_into().expect("bad block header"),
        ))
        .dispatch(origin_of(account_of(ALICE))));

        assert_store_main_chain_header_event(
            height,
            conf_block.header.hash().unwrap(),
            account_of(ALICE),
        );

        prev_block_hash = conf_block.header.hash().unwrap();
    }

    (tx_id, tx_block_height, bytes_proof, raw_tx)
}

#[allow(dead_code)]
pub type SystemModule = frame_system::Module<Runtime>;

#[allow(dead_code)]
pub type SecurityModule = security::Module<Runtime>;
#[allow(dead_code)]
pub type SecurityError = security::Error<Runtime>;

#[allow(dead_code)]
pub type VaultRegistryCall = vault_registry::Call<Runtime>;
#[allow(dead_code)]
pub type VaultRegistryModule = vault_registry::Module<Runtime>;
#[allow(dead_code)]
pub type VaultRegistryError = vault_registry::Error<Runtime>;

#[allow(dead_code)]
pub type ExchangeRateOracleCall = exchange_rate_oracle::Call<Runtime>;
#[allow(dead_code)]
pub type ExchangeRateOracleModule = exchange_rate_oracle::Module<Runtime>;

#[allow(dead_code)]
pub type SlaModule = sla::Module<Runtime>;

#[allow(dead_code)]
pub type FeeModule = fee::Module<Runtime>;
#[allow(dead_code)]
pub type FeeCall = fee::Call<Runtime>;

#[allow(dead_code)]
pub type CollateralModule = collateral::Module<Runtime>;

#[allow(dead_code)]
pub type TreasuryModule = treasury::Module<Runtime>;

pub struct ExtBuilder;

impl ExtBuilder {
    pub fn build() -> sp_io::TestExternalities {
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<Runtime>()
            .unwrap();

        pallet_balances::GenesisConfig::<Runtime, pallet_balances::Instance1> {
            balances: vec![
                (account_of(ALICE), INITIAL_BALANCE),
                (account_of(BOB), INITIAL_BALANCE),
                (account_of(CAROL), INITIAL_BALANCE),
                (account_of(FAUCET), 1 << 60),
                // create accounts for vault & fee pool; this needs a minimum amount because
                // the parachain refuses to create accounts with a balance below `ExistentialDeposit`
                (
                    account_of(LIQUIDATION_VAULT),
                    INITIAL_LIQUIDATION_VAULT_BALANCE,
                ),
                (account_of(FEE_POOL), 1000),
            ],
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        pallet_balances::GenesisConfig::<Runtime, pallet_balances::Instance2> { balances: vec![] }
            .assimilate_storage(&mut storage)
            .unwrap();

        exchange_rate_oracle::GenesisConfig::<Runtime> {
            authorized_oracles: vec![(account_of(BOB), BOB.to_vec())],
            max_delay: 3600000, // one hour
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        btc_relay::GenesisConfig::<Runtime> {
            bitcoin_confirmations: CONFIRMATIONS,
            parachain_confirmations: CONFIRMATIONS,
            disable_difficulty_check: false,
            disable_inclusion_check: false,
            disable_op_return_check: false,
            disable_relayer_auth: true,
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        vault_registry::GenesisConfig::<Runtime> {
            minimum_collateral_vault: 0,
            punishment_delay: 8,
            secure_collateral_threshold: FixedU128::checked_from_rational(150, 100).unwrap(),
            auction_collateral_threshold: FixedU128::checked_from_rational(120, 100).unwrap(),
            premium_redeem_threshold: FixedU128::checked_from_rational(135, 100).unwrap(),
            liquidation_collateral_threshold: FixedU128::checked_from_rational(110, 100).unwrap(),
            liquidation_vault_account_id: account_of(LIQUIDATION_VAULT),
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        issue::GenesisConfig::<Runtime> { issue_period: 10 }
            .assimilate_storage(&mut storage)
            .unwrap();

        redeem::GenesisConfig::<Runtime> {
            redeem_period: 10,
            redeem_btc_dust_value: 1,
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        replace::GenesisConfig::<Runtime> {
            replace_period: 10,
            replace_btc_dust_value: 1,
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        fee::GenesisConfig::<Runtime> {
            issue_fee: FixedU128::checked_from_rational(5, 1000).unwrap(), // 0.5%
            issue_griefing_collateral: FixedU128::checked_from_rational(5, 100000).unwrap(), // 0.005%
            refund_fee: FixedU128::checked_from_rational(5, 1000).unwrap(),                  // 0.5%
            redeem_fee: FixedU128::checked_from_rational(5, 1000).unwrap(),                  // 0.5%
            premium_redeem_fee: FixedU128::checked_from_rational(5, 100).unwrap(),           // 5%
            auction_redeem_fee: FixedU128::checked_from_rational(5, 100).unwrap(),           // 5%
            punishment_fee: FixedU128::checked_from_rational(1, 10).unwrap(),                // 10%
            replace_griefing_collateral: FixedU128::checked_from_rational(1, 10).unwrap(),   // 10%
            fee_pool_account_id: account_of(FEE_POOL),
            maintainer_account_id: account_of(MAINTAINER),
            epoch_period: 5,
            // give 90% of the rewards to vaults in order for withdrawal to work
            // since we cannot transfer below `ExistentialDeposit`
            vault_rewards: FixedU128::checked_from_rational(90, 100).unwrap(), // 90%
            vault_rewards_issued: FixedU128::checked_from_rational(90, 100).unwrap(), // 90%
            vault_rewards_locked: FixedU128::checked_from_rational(10, 100).unwrap(), // 10%
            relayer_rewards: FixedU128::checked_from_rational(10, 100).unwrap(), // 10%
            maintainer_rewards: FixedU128::from(0),                            // 0%
            collator_rewards: FixedU128::from(0),                              // 0%
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        sla::GenesisConfig::<Runtime> {
            vault_target_sla: FixedI128::from(100),
            vault_redeem_failure_sla_change: FixedI128::from(-100),
            vault_executed_issue_max_sla_change: FixedI128::from(4),
            vault_submitted_issue_proof: FixedI128::from(1),
            vault_refunded: FixedI128::from(1),
            relayer_target_sla: FixedI128::from(100),
            relayer_block_submission: FixedI128::from(1),
            relayer_correct_no_data_vote_or_report: FixedI128::from(1),
            relayer_correct_invalid_vote_or_report: FixedI128::from(10),
            relayer_correct_liquidation_report: FixedI128::from(1),
            relayer_correct_theft_report: FixedI128::from(1),
            relayer_correct_oracle_offline_report: FixedI128::from(1),
            relayer_false_no_data_vote_or_report: FixedI128::from(-10),
            relayer_false_invalid_vote_or_report: FixedI128::from(-100),
            relayer_ignored_vote: FixedI128::from(-10),
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        sp_io::TestExternalities::from(storage)
    }
}
