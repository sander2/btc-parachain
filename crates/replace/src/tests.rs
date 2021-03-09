use crate::ext;
use crate::mock::*;
use crate::{has_request_expired, PolkaBTC, ReplaceRequest, DOT};
use bitcoin::types::H256Le;
use btc_relay::{BtcAddress, BtcPublicKey};
use frame_support::{
    assert_err, assert_noop, assert_ok,
    dispatch::{DispatchError, DispatchResult},
};
use mocktopus::mocking::*;
use primitive_types::H256;
use sp_core::H160;
use vault_registry::{Vault, VaultStatus, Wallet};

type Event = crate::Event<Test>;

// use macro to avoid messing up stack trace
macro_rules! assert_emitted {
    ($event:expr) => {
        let test_event = TestEvent::replace($event);
        assert!(System::events().iter().any(|a| a.event == test_event));
    };
    ($event:expr, $times:expr) => {
        let test_event = TestEvent::replace($event);
        assert_eq!(
            System::events()
                .iter()
                .filter(|a| a.event == test_event)
                .count(),
            $times
        );
    };
}

fn test_request() -> ReplaceRequest<u64, u64, u64, u64> {
    ReplaceRequest {
        new_vault: None,
        old_vault: ALICE,
        open_time: 0,
        accept_time: None,
        amount: 10,
        griefing_collateral: 0,
        btc_address: Some(BtcAddress::default()),
        collateral: 20,
        completed: false,
        cancelled: false,
    }
}

fn dummy_public_key() -> BtcPublicKey {
    BtcPublicKey([
        2, 205, 114, 218, 156, 16, 235, 172, 106, 37, 18, 153, 202, 140, 176, 91, 207, 51, 187, 55,
        18, 45, 222, 180, 119, 54, 243, 97, 173, 150, 161, 169, 230,
    ])
}

fn test_vault() -> Vault<u64, u64, u64, u64> {
    Vault {
        id: BOB,
        banned_until: None,
        issued_tokens: 5,
        wallet: Wallet::new(dummy_public_key()),
        to_be_replaced_tokens: 0,
        to_be_issued_tokens: 0,
        to_be_redeemed_tokens: 0,
        backing_collateral: 0,
        status: VaultStatus::Active,
    }
}

fn request_replace(
    vault: AccountId,
    amount: Balance,
    griefing_collateral: DOT<Test>,
) -> DispatchResult {
    Replace::_request_replace(vault, amount, griefing_collateral)
}

fn withdraw_replace(vault_id: AccountId, replace_id: H256) -> Result<(), DispatchError> {
    Replace::_withdraw_replace_request(vault_id, replace_id)
}

fn accept_replace(
    vault_id: AccountId,
    replace_id: H256,
    collateral: DOT<Test>,
) -> Result<(), DispatchError> {
    Replace::_accept_replace(vault_id, replace_id, collateral, BtcAddress::default())
}

fn auction_replace(
    old_vault_id: AccountId,
    new_vault_id: AccountId,
    btc_amount: PolkaBTC<Test>,
    collateral: DOT<Test>,
) -> Result<(), DispatchError> {
    Replace::_auction_replace(
        old_vault_id,
        new_vault_id,
        btc_amount,
        collateral,
        BtcAddress::default(),
    )
}

fn execute_replace(
    replace_id: H256,
    tx_id: H256Le,
    merkle_proof: Vec<u8>,
    raw_tx: Vec<u8>,
) -> Result<(), DispatchError> {
    Replace::_execute_replace(replace_id, tx_id, merkle_proof, raw_tx)
}

fn cancel_replace(new_vault_id: AccountId, replace_id: H256) -> Result<(), DispatchError> {
    Replace::_cancel_replace(new_vault_id, replace_id)
}

#[test]
fn test_request_replace_transfer_zero_fails() {
    run_test(|| {
        ext::vault_registry::ensure_not_banned::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::vault_registry::get_active_vault_from_id::<Test>.mock_safe(|_| {
            MockResult::Return(Ok(Vault {
                id: BOB,
                to_be_replaced_tokens: 0,
                to_be_issued_tokens: 0,
                issued_tokens: 100,
                to_be_redeemed_tokens: 0,
                backing_collateral: 0,
                wallet: Wallet::new(dummy_public_key()),
                banned_until: None,
                status: VaultStatus::Active,
            }))
        });
        assert_noop!(request_replace(BOB, 0, 0), TestError::AmountBelowDustAmount);
    })
}

#[test]
fn test_request_replace_vault_not_found_fails() {
    run_test(|| {
        assert_noop!(
            request_replace(10_000, 5, 0),
            VaultRegistryError::VaultNotFound
        );
    })
}

#[test]
fn test_request_replace_vault_banned_fails() {
    run_test(|| {
        ext::vault_registry::ensure_not_banned::<Test>
            .mock_safe(|_, _| MockResult::Return(Err(VaultRegistryError::VaultBanned.into())));
        ext::vault_registry::get_active_vault_from_id::<Test>.mock_safe(|_| {
            MockResult::Return(Ok(Vault {
                id: BOB,
                to_be_replaced_tokens: 0,
                to_be_issued_tokens: 0,
                issued_tokens: 0,
                to_be_redeemed_tokens: 0,
                backing_collateral: 0,
                wallet: Wallet::new(dummy_public_key()),
                banned_until: Some(1),
                status: VaultStatus::Active,
            }))
        });
        assert_noop!(
            Replace::_request_replace(BOB, 5, 0),
            VaultRegistryError::VaultBanned
        );
    })
}
#[test]
fn test_request_replace_amount_below_dust_value_fails() {
    run_test(|| {
        let old_vault = BOB;
        let griefing_collateral = 0;
        let desired_griefing_collateral = 2;

        let amount = 1;

        ext::vault_registry::ensure_not_banned::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::vault_registry::get_active_vault_from_id::<Test>.mock_safe(|_| {
            MockResult::Return(Ok(Vault {
                id: BOB,
                to_be_replaced_tokens: 0,
                to_be_issued_tokens: 0,
                issued_tokens: 10,
                to_be_redeemed_tokens: 0,
                backing_collateral: 0,
                wallet: Wallet::new(dummy_public_key()),
                banned_until: None,
                status: VaultStatus::Active,
            }))
        });
        ext::vault_registry::is_over_minimum_collateral::<Test>
            .mock_safe(|_| MockResult::Return(true));
        ext::oracle::btc_to_dots::<Test>.mock_safe(|_| MockResult::Return(Ok(0)));
        ext::fee::get_replace_griefing_collateral::<Test>
            .mock_safe(move |_| MockResult::Return(Ok(desired_griefing_collateral)));

        assert_noop!(
            Replace::_request_replace(old_vault, amount, griefing_collateral),
            TestError::AmountBelowDustAmount
        );
    })
}

#[test]
fn test_request_replace_insufficient_griefing_collateral_fails() {
    run_test(|| {
        let old_vault = BOB;
        let griefing_collateral = 0;
        let desired_griefing_collateral = 2;

        let amount = 3;

        ext::vault_registry::ensure_not_banned::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::vault_registry::get_active_vault_from_id::<Test>.mock_safe(|_| {
            MockResult::Return(Ok(Vault {
                id: BOB,
                to_be_replaced_tokens: 0,
                to_be_issued_tokens: 0,
                issued_tokens: 10,
                to_be_redeemed_tokens: 0,
                backing_collateral: 0,
                wallet: Wallet::new(dummy_public_key()),
                banned_until: None,
                status: VaultStatus::Active,
            }))
        });
        ext::vault_registry::is_over_minimum_collateral::<Test>
            .mock_safe(|_| MockResult::Return(true));
        ext::oracle::btc_to_dots::<Test>.mock_safe(|_| MockResult::Return(Ok(0)));
        ext::fee::get_replace_griefing_collateral::<Test>
            .mock_safe(move |_| MockResult::Return(Ok(desired_griefing_collateral)));
        ext::vault_registry::get_backing_collateral::<Test>
            .mock_safe(move |_| MockResult::Return(Ok(desired_griefing_collateral)));
        assert_noop!(
            Replace::_request_replace(old_vault, amount, griefing_collateral),
            TestError::InsufficientCollateral
        );
    })
}

#[test]
fn test_withdraw_replace_request_invalid_replace_id_fails() {
    run_test(|| {
        Replace::get_open_replace_request
            .mock_safe(|_| MockResult::Return(Err(TestError::ReplaceIdNotFound.into())));
        assert_noop!(
            Replace::_withdraw_replace_request(ALICE, H256([0u8; 32])),
            TestError::ReplaceIdNotFound
        );
    })
}

#[test]
fn test_withdraw_replace_request_invalid_vault_id_fails() {
    run_test(|| {
        Replace::get_open_replace_request.mock_safe(|_| MockResult::Return(Ok(test_request())));
        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Err(VaultRegistryError::VaultNotFound.into())));
        assert_noop!(
            withdraw_replace(ALICE, H256([0u8; 32])),
            VaultRegistryError::VaultNotFound
        );
    })
}

#[test]
fn test_withdraw_replace_req_vault_id_mismatch_fails() {
    run_test(|| {
        Replace::get_open_replace_request.mock_safe(|_| MockResult::Return(Ok(test_request())));
        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(|_id| MockResult::Return(Ok(test_vault())));
        assert_noop!(
            withdraw_replace(BOB, H256([0u8; 32])),
            TestError::UnauthorizedVault
        );
    })
}

#[test]
fn test_withdraw_replace_req_under_secure_threshold_fails() {
    run_test(|| {
        Replace::get_open_replace_request.mock_safe(|_| MockResult::Return(Ok(test_request())));
        ext::vault_registry::get_active_vault_from_id::<Test>.mock_safe(|_id| {
            MockResult::Return(Ok({
                let mut v = test_vault();
                v.id = ALICE;
                v
            }))
        });
        assert_noop!(
            withdraw_replace(BOB, H256([0u8; 32])),
            TestError::UnauthorizedVault
        );
    })
}

#[test]
fn test_withdraw_replace_req_has_new_owner_fails() {
    run_test(|| {
        Replace::get_open_replace_request.mock_safe(|_| {
            let mut r = test_request();
            r.old_vault = ALICE;
            r.new_vault = Some(3);
            MockResult::Return(Ok(r))
        });
        ext::vault_registry::get_active_vault_from_id::<Test>.mock_safe(|_id| {
            MockResult::Return(Ok({
                let mut v = test_vault();
                v.id = ALICE;
                v
            }))
        });
        ext::vault_registry::is_vault_below_auction_threshold::<Test>
            .mock_safe(|_| MockResult::Return(Ok(false)));
        assert_noop!(
            withdraw_replace(ALICE, H256([0u8; 32])),
            TestError::CancelAcceptedRequest
        );
    })
}

#[test]
fn test_accept_replace_bad_replace_id_fails() {
    run_test(|| {
        Replace::get_open_replace_request.mock_safe(|_| {
            let mut r = test_request();
            r.old_vault = ALICE;
            r.new_vault = Some(3);
            MockResult::Return(Err(TestError::ReplaceIdNotFound.into()))
        });
        ext::vault_registry::get_active_vault_from_id::<Test>.mock_safe(|_id| {
            MockResult::Return(Ok({
                let mut v = test_vault();
                v.id = ALICE;
                v
            }))
        });
        ext::vault_registry::is_vault_below_auction_threshold::<Test>
            .mock_safe(|_| MockResult::Return(Ok(true)));
        let collateral = 100_000;
        assert_noop!(
            accept_replace(ALICE, H256([0u8; 32]), collateral),
            TestError::ReplaceIdNotFound
        );
    })
}

#[test]
fn test_accept_replace_bad_vault_id_fails() {
    run_test(|| {
        Replace::get_open_replace_request.mock_safe(|_| {
            let mut r = test_request();
            r.old_vault = ALICE;
            r.new_vault = Some(3);
            MockResult::Return(Ok(r))
        });
        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(|_id| MockResult::Return(Err(VaultRegistryError::VaultNotFound.into())));
        ext::vault_registry::is_vault_below_auction_threshold::<Test>
            .mock_safe(|_| MockResult::Return(Ok(false)));
        let collateral = 100_000;
        assert_noop!(
            accept_replace(ALICE, H256([0u8; 32]), collateral),
            VaultRegistryError::VaultNotFound
        );
    })
}

#[test]
fn test_accept_replace_vault_banned_fails() {
    run_test(|| {
        Replace::get_open_replace_request.mock_safe(|_| {
            let mut r = test_request();
            r.old_vault = ALICE;
            r.new_vault = Some(3);
            MockResult::Return(Ok(r))
        });
        ext::vault_registry::ensure_not_banned::<Test>
            .mock_safe(|_, _| MockResult::Return(Err(VaultRegistryError::VaultBanned.into())));
        ext::vault_registry::get_active_vault_from_id::<Test>.mock_safe(|_id| {
            let mut vault = test_vault();
            vault.banned_until = Some(100);
            MockResult::Return(Ok(vault))
        });
        ext::vault_registry::insert_vault_deposit_address::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::vault_registry::is_vault_below_auction_threshold::<Test>
            .mock_safe(|_| MockResult::Return(Ok(false)));
        let collateral = 100_000;
        assert_noop!(
            accept_replace(ALICE, H256([0u8; 32]), collateral),
            VaultRegistryError::VaultBanned
        );
    })
}

#[test]
fn test_accept_replace_insufficient_collateral_fails() {
    run_test(|| {
        Replace::get_open_replace_request.mock_safe(|_| {
            let mut r = test_request();
            r.old_vault = ALICE;
            r.new_vault = Some(3);
            MockResult::Return(Ok(r))
        });
        ext::vault_registry::ensure_not_banned::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::vault_registry::get_active_vault_from_id::<Test>.mock_safe(|_id| {
            let mut vault = test_vault();
            vault.banned_until = None;
            MockResult::Return(Ok(vault))
        });
        ext::vault_registry::try_lock_additional_collateral::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::vault_registry::insert_vault_deposit_address::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::vault_registry::decrease_to_be_replaced_tokens::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::vault_registry::try_increase_to_be_redeemed_tokens::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::vault_registry::try_increase_to_be_issued_tokens::<Test>.mock_safe(|_, _| {
            MockResult::Return(Err(VaultRegistryError::ExceedingVaultLimit.into()))
        });

        let collateral = 100_000;
        assert_err!(
            accept_replace(ALICE, H256([0u8; 32]), collateral),
            VaultRegistryError::ExceedingVaultLimit
        );
    })
}

#[test]
fn test_auction_replace_bad_old_vault_id_fails() {
    run_test(|| {
        ext::vault_registry::get_active_vault_from_id::<Test>.mock_safe(|id| {
            MockResult::Return(if *id == ALICE {
                Err(VaultRegistryError::VaultNotFound.into())
            } else {
                Ok(test_vault())
            })
        });
        let collateral = 100_000;
        let btc_amount = 100;
        assert_noop!(
            auction_replace(ALICE, BOB, btc_amount, collateral),
            VaultRegistryError::VaultNotFound
        );
    })
}

#[test]
fn test_auction_replace_bad_new_vault_id_fails() {
    run_test(|| {
        ext::vault_registry::get_active_vault_from_id::<Test>.mock_safe(|id| {
            MockResult::Return(if *id == ALICE {
                Ok(test_vault())
            } else {
                Err(VaultRegistryError::VaultNotFound.into())
            })
        });
        let collateral = 100_000;
        let btc_amount = 100;
        assert_noop!(
            auction_replace(ALICE, BOB, btc_amount, collateral),
            VaultRegistryError::VaultNotFound
        );
    })
}

#[test]
fn test_auction_replace_insufficient_collateral_fails() {
    run_test(|| {
        ext::vault_registry::get_active_vault_from_id::<Test>.mock_safe(|id| {
            MockResult::Return(if *id == ALICE {
                Ok(test_vault())
            } else {
                Ok(test_vault())
            })
        });
        ext::vault_registry::insert_vault_deposit_address::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::vault_registry::is_vault_below_auction_threshold::<Test>
            .mock_safe(|_| MockResult::Return(Ok(false)));
        let collateral = 100_000;
        let btc_amount = 100;
        assert_noop!(
            auction_replace(ALICE, BOB, btc_amount, collateral),
            TestError::VaultOverAuctionThreshold
        );
    })
}

#[test]
fn test_execute_replace_bad_replace_id_fails() {
    run_test(|| {
        Replace::get_open_replace_request
            .mock_safe(|_| MockResult::Return(Err(TestError::ReplaceIdNotFound.into())));

        let replace_id = H256::zero();
        let tx_id = H256Le::zero();
        let merkle_proof = Vec::new();
        let raw_tx = Vec::new();
        assert_err!(
            execute_replace(replace_id, tx_id, merkle_proof, raw_tx),
            TestError::ReplaceIdNotFound
        );
    })
}

#[test]
fn test_execute_replace_replace_period_expired_fails() {
    run_test(|| {
        let new_vault_id = BOB;
        let replace_id = H256::zero();
        let tx_id = H256Le::zero();
        let merkle_proof = Vec::new();
        let raw_tx = Vec::new();

        Replace::get_open_replace_request.mock_safe(move |_| {
            let mut req = test_request();
            req.open_time = 100_000;
            req.new_vault = Some(new_vault_id);
            MockResult::Return(Ok(req))
        });

        System::set_block_number(110_000);
        assert_err!(
            execute_replace(replace_id, tx_id, merkle_proof, raw_tx),
            TestError::ReplacePeriodExpired
        );
    })
}

#[test]
fn test_cancel_replace_invalid_replace_id_fails() {
    run_test(|| {
        Replace::get_open_replace_request
            .mock_safe(|_| MockResult::Return(Err(TestError::ReplaceIdNotFound.into())));

        let new_vault_id = ALICE;
        let replace_id = H256::zero();

        assert_err!(
            cancel_replace(new_vault_id, replace_id),
            TestError::ReplaceIdNotFound
        );
    })
}

#[test]
fn test_cancel_replace_period_not_expired_fails() {
    run_test(|| {
        Replace::get_open_replace_request.mock_safe(|_| MockResult::Return(Ok(test_request())));
        Replace::current_height.mock_safe(|| MockResult::Return(1));
        Replace::replace_period.mock_safe(|| MockResult::Return(2));
        let new_vault_id = ALICE;
        let replace_id = H256::zero();

        assert_err!(
            cancel_replace(new_vault_id, replace_id),
            TestError::ReplacePeriodNotExpired
        );
    })
}

#[test]
fn test_cancel_replace_period_not_expired_current_height_0_fails() {
    run_test(|| {
        Replace::get_open_replace_request.mock_safe(|_| MockResult::Return(Ok(test_request())));
        Replace::current_height.mock_safe(|| MockResult::Return(0));
        Replace::replace_period.mock_safe(|| MockResult::Return(2));
        let new_vault_id = ALICE;
        let replace_id = H256::zero();

        assert_err!(
            cancel_replace(new_vault_id, replace_id),
            TestError::ReplacePeriodNotExpired
        );
    })
}

#[test]
fn test_request_replace_with_amount_exceed_vault_issued_tokens_succeeds() {
    run_test(|| {
        let vault_id = BOB;
        let amount = 6;
        let replace_id = H256::zero();
        let griefing_collateral = 10_000;

        let vault = test_vault();
        let replace_amount = vault.issued_tokens;

        ext::vault_registry::ensure_not_banned::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));

        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(move |_| MockResult::Return(Ok(vault.clone())));

        ext::vault_registry::is_over_minimum_collateral::<Test>
            .mock_safe(|_| MockResult::Return(true));
        ext::collateral::lock_collateral::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::vault_registry::try_increase_to_be_replaced_tokens::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::security::get_secure_id::<Test>.mock_safe(|_| MockResult::Return(H256::zero()));
        ext::vault_registry::get_backing_collateral::<Test>
            .mock_safe(move |_| MockResult::Return(Ok(griefing_collateral)));

        assert_ok!(request_replace(vault_id, amount, griefing_collateral));

        let event =
            Event::RequestReplace(replace_id, vault_id, replace_amount, griefing_collateral);
        assert_emitted!(event);
    })
}

#[test]
fn test_request_replace_with_amount_less_than_vault_issued_tokens_succeeds() {
    run_test(|| {
        let vault_id = BOB;
        let amount = 3;
        let replace_id = H256::zero();
        let griefing_collateral = 10_000;

        let vault = test_vault();
        let replace_amount = amount;

        ext::vault_registry::ensure_not_banned::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));

        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(move |_| MockResult::Return(Ok(vault.clone())));

        ext::vault_registry::is_over_minimum_collateral::<Test>
            .mock_safe(|_| MockResult::Return(true));
        ext::collateral::lock_collateral::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));

        ext::vault_registry::try_increase_to_be_replaced_tokens::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::security::get_secure_id::<Test>.mock_safe(|_| MockResult::Return(H256::zero()));
        ext::vault_registry::get_backing_collateral::<Test>
            .mock_safe(move |_| MockResult::Return(Ok(griefing_collateral)));

        assert_ok!(request_replace(vault_id, amount, griefing_collateral));

        let event =
            Event::RequestReplace(replace_id, vault_id, replace_amount, griefing_collateral);
        assert_emitted!(event);
    })
}
#[test]
fn test_withdraw_replace_succeeds() {
    run_test(|| {
        let vault_id = BOB;
        let replace_id = H256::zero();

        Replace::get_open_replace_request.mock_safe(|_| {
            let mut replace = test_request();
            replace.old_vault = BOB;
            MockResult::Return(Ok(replace))
        });

        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(test_vault())));
        ext::vault_registry::is_vault_below_auction_threshold::<Test>
            .mock_safe(|_| MockResult::Return(Ok(false)));
        ext::collateral::release_collateral::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::vault_registry::decrease_to_be_replaced_tokens::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(())));

        Replace::remove_replace_request.mock_safe(|_, _| MockResult::Return(()));

        assert_eq!(withdraw_replace(vault_id, replace_id), Ok(()));

        let event = Event::WithdrawReplace(replace_id, vault_id);
        assert_emitted!(event);
    })
}

#[test]
fn test_accept_replace_succeeds() {
    run_test(|| {
        let old_vault_id = ALICE;
        let new_vault_id = BOB;
        let replace_id = H256::zero();
        let collateral = 20_000;
        let btc_amount = 100;

        Replace::get_open_replace_request.mock_safe(move |_| {
            let mut replace = test_request();
            replace.old_vault = old_vault_id;
            replace.amount = btc_amount;
            MockResult::Return(Ok(replace))
        });

        ext::vault_registry::ensure_not_banned::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));

        ext::vault_registry::try_lock_additional_collateral::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(())));

        ext::vault_registry::decrease_to_be_replaced_tokens::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(())));

        ext::vault_registry::try_increase_to_be_redeemed_tokens::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(())));

        ext::vault_registry::try_increase_to_be_issued_tokens::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(())));

        ext::vault_registry::insert_vault_deposit_address::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(())));

        ext::collateral::lock_collateral::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));

        assert_eq!(accept_replace(new_vault_id, replace_id, collateral), Ok(()));

        let event = Event::AcceptReplace(
            replace_id,
            old_vault_id,
            new_vault_id,
            btc_amount,
            collateral,
            BtcAddress::default(),
        );
        assert_emitted!(event);
    })
}

#[test]
fn test_auction_replace_succeeds() {
    run_test(|| {
        let old_vault_id = ALICE;
        let new_vault_id = BOB;
        let btc_amount = 1000;
        let collateral = 20_000;
        let height = 10;
        let replace_id = H256::zero();
        let reward = 50;
        let griefing_collateral = 0;

        // NOTE: we don't use the old_vault in the code - should be changed to just
        // check if it exists in storage
        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(test_vault())));

        ext::vault_registry::insert_vault_deposit_address::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(())));

        ext::vault_registry::is_vault_below_auction_threshold::<Test>
            .mock_safe(|_| MockResult::Return(Ok(true)));

        ext::vault_registry::slash_collateral::<Test>.mock_safe(|_, _, fee| {
            assert_eq!(fee, 50); // 5% of dot equivalent of the btc_amount
            MockResult::Return(Ok(()))
        });

        ext::vault_registry::try_lock_additional_collateral::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(())));

        ext::vault_registry::try_increase_to_be_redeemed_tokens::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(())));

        ext::security::get_secure_id::<Test>.mock_safe(|_| MockResult::Return(H256::zero()));

        ext::fee::get_auction_redeem_fee::<Test>.mock_safe(move |_| MockResult::Return(Ok(reward)));

        ext::fee::get_replace_griefing_collateral::<Test>
            .mock_safe(move |_| MockResult::Return(Ok(griefing_collateral)));

        ext::vault_registry::try_increase_to_be_issued_tokens::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(())));

        Replace::current_height.mock_safe(move || MockResult::Return(height.clone()));

        assert_eq!(
            auction_replace(old_vault_id, new_vault_id, btc_amount, collateral),
            Ok(())
        );

        assert_emitted!(Event::AuctionReplace(
            replace_id,
            old_vault_id,
            new_vault_id,
            btc_amount,
            collateral,
            reward,
            griefing_collateral,
            height,
            BtcAddress::default(),
        ));
    })
}

#[test]
fn test_execute_replace_succeeds() {
    run_test(|| {
        let old_vault_id = ALICE;
        let new_vault_id = BOB;
        let replace_id = H256::zero();
        let tx_id = H256Le::zero();
        let merkle_proof = Vec::new();
        let raw_tx = Vec::new();

        Replace::get_open_replace_request.mock_safe(move |_| {
            let mut replace = test_request();
            replace.old_vault = old_vault_id.clone();
            replace.new_vault = Some(new_vault_id.clone());
            replace.open_time = 5;
            MockResult::Return(Ok(replace))
        });

        Replace::current_height.mock_safe(|| MockResult::Return(10));
        Replace::replace_period.mock_safe(|| MockResult::Return(20));

        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(test_vault())));

        ext::btc_relay::verify_transaction_inclusion::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::btc_relay::validate_transaction::<Test>
            .mock_safe(|_, _, _, _| MockResult::Return(Ok((BtcAddress::P2SH(H160::zero()), 0))));

        ext::vault_registry::replace_tokens::<Test>
            .mock_safe(|_, _, _, _| MockResult::Return(Ok(())));

        ext::collateral::release_collateral::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));

        ext::vault_registry::is_vault_liquidated::<Test>
            .mock_safe(|_| MockResult::Return(Ok(false)));

        Replace::remove_replace_request.mock_safe(|_, _| MockResult::Return(()));

        assert_eq!(
            execute_replace(replace_id, tx_id, merkle_proof, raw_tx),
            Ok(())
        );

        let event = Event::ExecuteReplace(replace_id, old_vault_id, new_vault_id);
        assert_emitted!(event);
    })
}

#[test]
fn test_cancel_replace_succeeds() {
    run_test(|| {
        let new_vault_id = BOB;
        let old_vault_id = ALICE;
        let replace_id = H256::zero();
        let griefing_collateral = 0;

        System::set_block_number(45);
        Replace::get_open_replace_request.mock_safe(move |_| {
            let mut replace = test_request();
            replace.old_vault = old_vault_id.clone();
            replace.new_vault = Some(new_vault_id.clone());
            replace.open_time = 2;
            MockResult::Return(Ok(replace))
        });
        Replace::current_height.mock_safe(|| MockResult::Return(15));
        Replace::replace_period.mock_safe(|| MockResult::Return(2));
        ext::vault_registry::cancel_replace_tokens::<Test>
            .mock_safe(|_, _, _| MockResult::Return(Ok(())));
        ext::vault_registry::is_vault_liquidated::<Test>
            .mock_safe(|_| MockResult::Return(Ok(false)));

        Replace::remove_replace_request.mock_safe(|_, _| MockResult::Return(()));
        ext::vault_registry::slash_collateral::<Test>
            .mock_safe(|_, _, _| MockResult::Return(Ok(())));
        ext::vault_registry::is_allowed_to_withdraw_collateral::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(false)));
        assert_eq!(cancel_replace(new_vault_id, replace_id), Ok(()));

        let event =
            Event::CancelReplace(replace_id, new_vault_id, old_vault_id, griefing_collateral);
        assert_emitted!(event);
    })
}

#[test]
fn test_cancel_replace_as_third_party_fails() {
    run_test(|| {
        let new_vault_id = BOB;
        let old_vault_id = ALICE;
        let canceller = CAROL;
        let replace_id = H256::zero();

        System::set_block_number(45);
        Replace::get_open_replace_request.mock_safe(move |_| {
            let mut replace = test_request();
            replace.old_vault = old_vault_id.clone();
            replace.new_vault = Some(new_vault_id.clone());
            replace.open_time = 2;
            MockResult::Return(Ok(replace))
        });
        Replace::current_height.mock_safe(|| MockResult::Return(15));
        Replace::replace_period.mock_safe(|| MockResult::Return(2));
        Replace::remove_replace_request.mock_safe(|_, _| MockResult::Return(()));

        assert_noop!(
            cancel_replace(canceller, replace_id),
            TestError::UnauthorizedVault
        );
    })
}

// Security module integration tests
#[test]
fn test_request_replace_parachain_not_running_fails() {
    run_test(|| {
        ext::security::ensure_parachain_status_running::<Test>
            .mock_safe(|| MockResult::Return(Err(SecurityError::ParachainNotRunning.into())));
        assert_noop!(
            request_replace(10_000, 1, 0),
            SecurityError::ParachainNotRunning
        );
    })
}

#[test]
fn test_accept_replace_parachain_not_running_fails() {
    run_test(|| {
        ext::security::ensure_parachain_status_running::<Test>
            .mock_safe(|| MockResult::Return(Err(SecurityError::ParachainNotRunning.into())));
        assert_noop!(
            accept_replace(BOB, H256::zero(), 1),
            SecurityError::ParachainNotRunning
        );
    })
}

#[test]
fn test_auction_replace_parachain_not_running_fails() {
    run_test(|| {
        ext::security::ensure_parachain_status_running::<Test>
            .mock_safe(|| MockResult::Return(Err(SecurityError::ParachainNotRunning.into())));
        assert_noop!(
            auction_replace(ALICE, BOB, 1, 1),
            SecurityError::ParachainNotRunning
        );
    })
}

#[test]
fn test_execute_replace_parachain_not_running_fails() {
    run_test(|| {
        ext::security::ensure_parachain_status_running::<Test>
            .mock_safe(|| MockResult::Return(Err(SecurityError::ParachainNotRunning.into())));
        assert_noop!(
            execute_replace(H256::zero(), H256Le::zero(), Vec::new(), Vec::new()),
            SecurityError::ParachainNotRunning
        );
    })
}

#[test]
fn test_cancel_replace_parachain_not_running_fails() {
    run_test(|| {
        ext::security::ensure_parachain_status_running::<Test>
            .mock_safe(|| MockResult::Return(Err(SecurityError::ParachainNotRunning.into())));
        assert_noop!(
            cancel_replace(ALICE, H256::zero()),
            SecurityError::ParachainNotRunning
        );
    })
}

#[test]
fn test_withdraw_replace_parachain_not_running_succeeds() {
    run_test(|| {
        let vault_id = BOB;
        let replace_id = H256::zero();

        Replace::get_open_replace_request.mock_safe(|_| {
            let mut replace = test_request();
            replace.old_vault = BOB;
            MockResult::Return(Ok(replace))
        });

        ext::vault_registry::get_active_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(test_vault())));
        ext::vault_registry::is_vault_below_auction_threshold::<Test>
            .mock_safe(|_| MockResult::Return(Ok(false)));
        ext::vault_registry::try_increase_to_be_redeemed_tokens::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::collateral::release_collateral::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::vault_registry::decrease_to_be_replaced_tokens::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::security::ensure_parachain_status_running::<Test>
            .mock_safe(|| MockResult::Return(Err(SecurityError::ParachainNotRunning.into())));

        Replace::remove_replace_request.mock_safe(|_, _| MockResult::Return(()));

        assert_eq!(withdraw_replace(vault_id, replace_id), Ok(()));

        let event = Event::WithdrawReplace(replace_id, vault_id);
        assert_emitted!(event);
    })
}

#[test]
fn test_set_replace_period_only_root() {
    run_test(|| {
        assert_noop!(
            Replace::set_replace_period(Origin::signed(ALICE), 1),
            DispatchError::BadOrigin
        );
        assert_ok!(Replace::set_replace_period(Origin::root(), 1));
    })
}

#[test]
fn test_has_request_expired() {
    run_test(|| {
        System::set_block_number(4525);
        assert!(has_request_expired::<Test>(11, 300));
        assert!(!has_request_expired::<Test>(2758, 5000));
    })
}
