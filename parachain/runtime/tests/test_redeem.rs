mod mock;

use mock::*;

use primitive_types::H256;

type RedeemCall = redeem::Call<Runtime>;
type RedeemModule = redeem::Module<Runtime>;
type RedeemEvent = redeem::Event<Runtime>;
type RedeemError = redeem::Error<Runtime>;
use vault_registry::types::RichVault;
use vault_registry::types::UpdatableVault;

const USER: [u8; 32] = ALICE;
const VAULT: [u8; 32] = BOB;
const USER_BTC_ADDRESS: BtcAddress = BtcAddress::P2PKH(H160([2u8; 20]));
pub const DEFAULT_USER_FREE_BALANCE: u128 = 1_000_000;
pub const DEFAULT_USER_LOCKED_BALANCE: u128 = 100_000;

// asserts redeem event happen and extracts its id for further testing
fn assert_redeem_request_event() -> H256 {
    let events = SystemModule::events();
    let ids = events
        .iter()
        .filter_map(|r| match r.event {
            Event::redeem(RedeemEvent::RequestRedeem(id, _, _, _, _, _, _)) => Some(id.clone()),
            _ => None,
        })
        .collect::<Vec<H256>>();
    assert_eq!(ids.len(), 1);
    ids[0].clone()
}

#[test]
fn integration_test_redeem_should_fail_if_not_running() {
    ExtBuilder::build().execute_with(|| {
        SecurityModule::set_status(StatusCode::Shutdown);

        assert_noop!(
            Call::Redeem(RedeemCall::request_redeem(
                1000,
                BtcAddress::P2PKH(H160([0u8; 20])),
                account_of(BOB),
            ))
            .dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainNotRunning,
        );
    });
}

#[test]
fn integration_test_redeem_polka_btc_execute() {
    ExtBuilder::build().execute_with(|| {
        let user = ALICE;
        let vault = BOB;
        let collateral_vault = 1_000_000_000_000;
        let polka_btc = 1_000_000_000_000;

        let user_btc_address = BtcAddress::P2PKH(H160([2; 20]));

        SystemModule::set_block_number(1);

        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::checked_from_rational(1, 100_000).unwrap()
        ));

        set_default_thresholds();

        // create tokens for the vault and user
        force_issue_tokens(user, vault, collateral_vault, polka_btc);

        let initial_dot_balance = CollateralModule::get_balance_from_account(&account_of(user));
        let initial_btc_balance = TreasuryModule::get_balance_from_account(account_of(user));
        let initial_btc_issuance = TreasuryModule::get_total_supply();
        assert_eq!(polka_btc, initial_btc_issuance);

        // alice requests to redeem polka_btc from Bob
        assert_ok!(Call::Redeem(RedeemCall::request_redeem(
            polka_btc,
            user_btc_address,
            account_of(vault)
        ))
        .dispatch(origin_of(account_of(user))));

        // assert that request happened and extract the id
        let redeem_id = assert_redeem_request_event();
        let redeem = RedeemModule::get_open_redeem_request_from_id(&redeem_id).unwrap();

        // send the btc from the vault to the user
        let (tx_id, _tx_block_height, merkle_proof, raw_tx) =
            generate_transaction_and_mine(user_btc_address, polka_btc, Some(redeem_id));

        SystemModule::set_block_number(1 + CONFIRMATIONS);

        assert_ok!(Call::Redeem(RedeemCall::execute_redeem(
            redeem_id,
            tx_id,
            merkle_proof,
            raw_tx
        ))
        .dispatch(origin_of(account_of(vault))));

        let final_dot_balance = CollateralModule::get_balance_from_account(&account_of(user));
        let final_btc_balance = TreasuryModule::get_balance_from_account(account_of(user));
        let final_btc_issuance = TreasuryModule::get_total_supply();

        assert_eq!(final_dot_balance, initial_dot_balance);

        // polka_btc burned from user, including fee
        assert_eq!(final_btc_balance, initial_btc_balance - polka_btc);
        // polka_btc burned from issuance
        assert_eq!(final_btc_issuance, initial_btc_issuance - redeem.amount_btc);

        // TODO: check redeem rewards update
    });
}

#[test]
fn integration_test_premium_redeem_polka_btc_execute() {
    ExtBuilder::build().execute_with(|| {
        let user = ALICE;
        let vault = BOB;
        let polka_btc = 1_000_000_000;

        let user_btc_address = BtcAddress::P2PKH(H160([2; 20]));

        SystemModule::set_block_number(1);

        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));

        set_default_thresholds();

        let collateral_vault = required_collateral_for_issue(polka_btc);

        // create tokens for the vault and user
        force_issue_tokens(user, vault, collateral_vault, polka_btc);

        // suddenly require twice as much DOT; we are definitely below premium redeem threshold now
        // (also below liquidation threshold, but as long as we don't call liquidate that's ok)
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::checked_from_integer(2).unwrap()
        ));

        let initial_dot_balance = CollateralModule::get_balance_from_account(&account_of(user));
        let initial_btc_balance = TreasuryModule::get_balance_from_account(account_of(user));
        let initial_vault_collateral =
            CollateralModule::get_collateral_from_account(&account_of(vault));
        let initial_btc_issuance = TreasuryModule::get_total_supply();
        assert_eq!(polka_btc, initial_btc_issuance);

        // alice requests to redeem polka_btc from Bob
        assert_ok!(Call::Redeem(RedeemCall::request_redeem(
            polka_btc,
            user_btc_address,
            account_of(vault)
        ))
        .dispatch(origin_of(account_of(user))));

        // assert that request happened and extract the id
        let redeem_id = assert_redeem_request_event();
        let redeem = RedeemModule::get_open_redeem_request_from_id(&redeem_id).unwrap();

        // send the btc from the vault to the user
        let (tx_id, _tx_block_height, merkle_proof, raw_tx) =
            generate_transaction_and_mine(user_btc_address, polka_btc, Some(redeem_id));

        SystemModule::set_block_number(1 + CONFIRMATIONS);

        assert_ok!(Call::Redeem(RedeemCall::execute_redeem(
            redeem_id,
            tx_id,
            merkle_proof,
            raw_tx
        ))
        .dispatch(origin_of(account_of(vault))));

        let final_dot_balance = CollateralModule::get_balance_from_account(&account_of(user));
        let final_btc_balance = TreasuryModule::get_balance_from_account(account_of(user));
        let final_btc_issuance = TreasuryModule::get_total_supply();

        // user should have received some premium (DOT)
        assert!(final_dot_balance > initial_dot_balance);

        // it should be a zero-sum game; the user's gain is equal to the vault's loss
        assert_eq!(
            initial_vault_collateral + initial_dot_balance,
            CollateralModule::get_collateral_from_account(&account_of(vault)) + final_dot_balance
        );

        // polka_btc burned from user, including fee
        assert_eq!(final_btc_balance, initial_btc_balance - polka_btc);
        // polka_btc burned from issuance
        assert_eq!(final_btc_issuance, initial_btc_issuance - redeem.amount_btc);

        // TODO: check redeem rewards update
    });
}

#[test]
fn integration_test_redeem_polka_btc_liquidation_redeem() {
    ExtBuilder::build().execute_with(|| {
        SystemModule::set_block_number(1);
        set_default_thresholds();
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));

        let issued = 400;
        let to_be_issued = 100;
        let to_be_redeemed = 50;

        UserData::force_to(
            USER,
            UserData {
                free_tokens: 1000,
                ..Default::default()
            },
        );
        CoreVaultData::force_to(
            VAULT,
            CoreVaultData {
                issued,
                to_be_issued,
                to_be_redeemed,
                backing_collateral: 10_000,
                ..Default::default()
            },
        );

        // create tokens for the vault and user
        drop_exchange_rate_and_liquidate(VAULT);

        let slashed_collateral = 10_000 - (10000 * to_be_redeemed) / (issued + to_be_issued);

        assert_eq!(
            CoreVaultData::liquidation_vault(),
            CoreVaultData {
                issued,
                to_be_issued,
                to_be_redeemed,
                backing_collateral: slashed_collateral,
                free_balance: INITIAL_LIQUIDATION_VAULT_BALANCE,
                ..Default::default()
            },
        );

        assert_noop!(
            Call::Redeem(RedeemCall::liquidation_redeem(351)).dispatch(origin_of(account_of(USER))),
            VaultRegistryError::InsufficientTokensCommitted
        );

        assert_ok!(
            Call::Redeem(RedeemCall::liquidation_redeem(325)).dispatch(origin_of(account_of(USER)))
        );

        assert_eq!(
            UserData::get(USER),
            UserData {
                free_balance: (slashed_collateral * 325) / (issued + to_be_issued),
                free_tokens: 1000 - 325,
                ..Default::default()
            },
        );
    });
}

fn setup_cancelable_redeem(
    user: [u8; 32],
    vault: [u8; 32],
    collateral: u128,
    polka_btc: u128,
) -> H256 {
    let redeem_id = setup_redeem(polka_btc, user, vault, collateral);

    // expire request without transferring btc
    SystemModule::set_block_number(RedeemModule::redeem_period() + 1 + 1);

    // bob cannot execute past expiry
    assert_noop!(
        Call::Redeem(RedeemCall::execute_redeem(
            redeem_id,
            H256Le::from_bytes_le(&[0; 32]),
            vec![],
            vec![],
        ))
        .dispatch(origin_of(account_of(vault))),
        RedeemError::CommitPeriodExpired,
    );

    redeem_id
}

fn setup_redeem(polka_btc: u128, user: [u8; 32], vault: [u8; 32], collateral: u128) -> H256 {
    SystemModule::set_block_number(1);

    set_default_thresholds();

    assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
        FixedU128::one()
    ));

    let fee = FeeModule::get_redeem_fee(polka_btc).unwrap();

    // burn surplus free balance to make checking easier
    CollateralModule::transfer(
        account_of(vault),
        account_of(FAUCET),
        CollateralModule::get_balance_from_account(&account_of(vault)) - collateral,
    )
    .unwrap();

    // create tokens for the vault and user
    force_issue_tokens(user, vault, collateral, polka_btc - fee);

    // mint tokens to the user such that he can afford the fee
    TreasuryModule::mint(user.into(), fee);

    // alice requests to redeem polka_btc from Bob
    assert_ok!(Call::Redeem(RedeemCall::request_redeem(
        polka_btc,
        USER_BTC_ADDRESS,
        account_of(vault)
    ))
    .dispatch(origin_of(account_of(user))));

    // assert that request happened and extract the id
    assert_redeem_request_event()
}

#[test]
fn integration_test_redeem_polka_btc_cancel_reimburse() {
    ExtBuilder::build().execute_with(|| {
        let user = ALICE;
        let vault = BOB;
        let amount_btc = 100000;

        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::checked_from_integer(10).unwrap() // 10 planck/satoshi
        ));

        let initial_balance_dot = CollateralModule::get_balance_from_account(&account_of(user));

        let redeem_id = setup_cancelable_redeem(user, vault, 100000000, amount_btc);
        let redeem = RedeemModule::get_open_redeem_request_from_id(&redeem_id).unwrap();
        let amount_without_fee_dot =
            ExchangeRateOracleModule::btc_to_dots(redeem.amount_btc).unwrap();

        let punishment_fee = FeeModule::get_punishment_fee(amount_without_fee_dot).unwrap();
        assert!(punishment_fee > 0);

        // get initial balance - the setup call above will have minted and locked polkabtc
        let initial_balance_btc = TreasuryModule::get_balance_from_account(account_of(user))
            + TreasuryModule::get_locked_balance_from_account(account_of(user));

        let sla_score_before = FixedI128::from(60);
        SlaModule::set_vault_sla(account_of(vault), sla_score_before);

        // alice cancels redeem request and chooses to reimburse
        assert_ok!(Call::Redeem(RedeemCall::cancel_redeem(redeem_id, true))
            .dispatch(origin_of(account_of(user))));

        let new_balance = CollateralModule::get_balance_from_account(&account_of(user));

        // balance should have increased by punishment_fee plus amount_without_fee_dot
        assert_eq!(
            new_balance,
            initial_balance_dot + amount_without_fee_dot + punishment_fee
        );

        // user gets fee back, but loses the rest of the requested btc
        assert_eq!(
            TreasuryModule::get_balance_from_account(account_of(user)),
            initial_balance_btc - (amount_btc - redeem.fee)
        );

        // vault's SLA is reduced by redeem failure amount
        let expected_sla = FixedI128::max(
            FixedI128::zero(),
            sla_score_before + SlaModule::vault_redeem_failure_sla_change(),
        );
        assert_eq!(SlaModule::vault_sla(account_of(vault)), expected_sla);
    });
}

#[test]
fn integration_test_redeem_polka_btc_cancel_no_reimburse() {
    ExtBuilder::build().execute_with(|| {
        let user = ALICE;
        let vault = BOB;
        let amount_btc = 1000;

        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::checked_from_integer(10).unwrap() // 10 planck/satoshi
        ));

        let initial_balance_dot = CollateralModule::get_balance_from_account(&account_of(user));

        let redeem_id = setup_cancelable_redeem(user, vault, 100000000, amount_btc);
        let redeem = RedeemModule::get_open_redeem_request_from_id(&redeem_id).unwrap();
        let punishment_fee = FeeModule::get_punishment_fee(
            ExchangeRateOracleModule::btc_to_dots(redeem.amount_btc).unwrap(),
        )
        .unwrap();
        assert!(punishment_fee > 0);

        // get initial balance - the setup call above will have minted and locked polkabtc
        let initial_balance_btc = TreasuryModule::get_balance_from_account(account_of(user))
            + TreasuryModule::get_locked_balance_from_account(account_of(user));

        let sla_score_before = FixedI128::from(60);
        SlaModule::set_vault_sla(account_of(vault), sla_score_before);

        // alice cancels redeem request, but does not reimburse
        assert_ok!(Call::Redeem(RedeemCall::cancel_redeem(redeem_id, false))
            .dispatch(origin_of(account_of(user))));

        // dot-balance should have increased by punishment_fee
        assert_eq!(
            CollateralModule::get_balance_from_account(&account_of(user)),
            initial_balance_dot + punishment_fee
        );

        // polkabtc balance should not have changed
        assert_eq!(
            TreasuryModule::get_balance_from_account(account_of(user)),
            initial_balance_btc
        );

        // vault's SLA is reduced by redeem failure amount
        let expected_sla = FixedI128::max(
            FixedI128::zero(),
            sla_score_before + SlaModule::vault_redeem_failure_sla_change(),
        );
        assert_eq!(SlaModule::vault_sla(account_of(vault)), expected_sla);
    });
}

fn test_cancel_liquidated(reimburse: bool) {
    let polka_btc = 1_000;
    let fee = FeeModule::get_redeem_fee(polka_btc).unwrap();
    let collateral_vault = 1_000_000;
    let amount_without_fee = polka_btc - fee;

    let redeem_id = setup_cancelable_redeem(USER, VAULT, collateral_vault, polka_btc);

    UserData::force_to(
        USER,
        UserData {
            free_balance: DEFAULT_USER_FREE_BALANCE,
            locked_balance: DEFAULT_USER_LOCKED_BALANCE,
            locked_tokens: polka_btc + 1234,
            free_tokens: 50,
        },
    );

    assert_eq!(
        CoreVaultData::vault(VAULT),
        CoreVaultData {
            issued: amount_without_fee, // assuming fee
            to_be_redeemed: amount_without_fee,
            backing_collateral: collateral_vault,
            ..Default::default()
        },
    );
    CoreVaultData::force_to(
        VAULT,
        CoreVaultData {
            issued: amount_without_fee * 4,
            to_be_redeemed: amount_without_fee * 4,
            backing_collateral: collateral_vault,
            ..Default::default()
        },
    );

    drop_exchange_rate_and_liquidate(VAULT);

    assert_ok!(
        Call::Redeem(RedeemCall::cancel_redeem(redeem_id, reimburse))
            .dispatch(origin_of(account_of(USER)))
    );

    assert_eq!(
        CoreVaultData::vault(VAULT),
        CoreVaultData {
            to_be_redeemed: amount_without_fee * 3,
            backing_collateral: (collateral_vault * 3) / 4,
            ..Default::default()
        },
    );

    if reimburse {
        assert_eq!(
            UserData::get(USER),
            UserData {
                free_balance: DEFAULT_USER_FREE_BALANCE + collateral_vault / 4,
                locked_balance: DEFAULT_USER_LOCKED_BALANCE,
                locked_tokens: 1234,   // polka_btc has been burned
                free_tokens: 50 + fee, // user gets fee back
            },
        );
    } else {
        assert_eq!(
            UserData::get(USER),
            UserData {
                free_balance: DEFAULT_USER_FREE_BALANCE,
                locked_balance: DEFAULT_USER_LOCKED_BALANCE,
                locked_tokens: 1234, // polka_btc has been unlocked
                free_tokens: 50 + polka_btc,
            },
        );
    }
}
#[test]
fn integration_test_redeem_polka_btc_cancel_liquidated_reimburse() {
    ExtBuilder::build().execute_with(|| test_cancel_liquidated(true));
}

#[test]
fn integration_test_redeem_polka_btc_cancel_liquidated_no_reimburse() {
    ExtBuilder::build().execute_with(|| test_cancel_liquidated(false));
}

#[test]
fn integration_test_redeem_polka_btc_execute_liquidated() {
    ExtBuilder::build().execute_with(|| {
        let polka_btc = 1_000;
        let fee = FeeModule::get_redeem_fee(polka_btc).unwrap();
        let collateral_vault = 1_000_000;
        let amount_without_fee = polka_btc - fee;

        let redeem_id = setup_redeem(polka_btc, USER, VAULT, collateral_vault);

        UserData::force_to(
            USER,
            UserData {
                free_balance: DEFAULT_USER_FREE_BALANCE,
                locked_balance: DEFAULT_USER_LOCKED_BALANCE,
                locked_tokens: polka_btc + 1234,
                free_tokens: 50,
            },
        );

        assert_eq!(
            CoreVaultData::vault(VAULT),
            CoreVaultData {
                issued: amount_without_fee, // assuming fee
                to_be_redeemed: amount_without_fee,
                backing_collateral: collateral_vault,
                ..Default::default()
            },
        );
        CoreVaultData::force_to(
            VAULT,
            CoreVaultData {
                issued: amount_without_fee * 4,
                to_be_redeemed: amount_without_fee * 4,
                backing_collateral: collateral_vault,
                ..Default::default()
            },
        );

        drop_exchange_rate_and_liquidate(VAULT);

        assert_redeem_ok(polka_btc, redeem_id);

        assert_eq!(
            CoreVaultData::vault(VAULT),
            CoreVaultData {
                to_be_redeemed: amount_without_fee * 3,
                backing_collateral: (collateral_vault * 3) / 4,
                free_balance: collateral_vault / 4,
                ..Default::default()
            },
        );

        assert_eq!(
            UserData::get(USER),
            UserData {
                free_balance: DEFAULT_USER_FREE_BALANCE,
                locked_balance: DEFAULT_USER_LOCKED_BALANCE,
                locked_tokens: 1234, // most important: check that polka_btc has been burned
                free_tokens: 50,
            },
        );
    });
}

fn assert_redeem_ok(polka_btc: u128, redeem_id: H256) {
    // send the btc from the vault to the user
    let (tx_id, _tx_block_height, merkle_proof, raw_tx) =
        generate_transaction_and_mine(USER_BTC_ADDRESS, polka_btc, Some(redeem_id));

    SystemModule::set_block_number(1 + CONFIRMATIONS);

    assert_ok!(Call::Redeem(RedeemCall::execute_redeem(
        redeem_id,
        tx_id,
        merkle_proof,
        raw_tx
    ))
    .dispatch(origin_of(account_of(VAULT))));
}
