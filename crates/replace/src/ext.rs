#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub(crate) mod btc_relay {
    use bitcoin::types::H256Le;
    use btc_relay::BtcAddress;
    use frame_support::dispatch::{DispatchError, DispatchResult};
    use sp_std::vec::Vec;

    pub fn verify_transaction_inclusion<T: btc_relay::Config>(
        tx_id: H256Le,
        merkle_proof: Vec<u8>,
    ) -> DispatchResult {
        <btc_relay::Module<T>>::_verify_transaction_inclusion(tx_id, merkle_proof, None)
    }

    pub fn validate_transaction<T: btc_relay::Config>(
        raw_tx: Vec<u8>,
        amount: i64,
        btc_address: BtcAddress,
        replace_id: Option<Vec<u8>>,
    ) -> Result<(BtcAddress, i64), DispatchError> {
        <btc_relay::Module<T>>::_validate_transaction(raw_tx, amount, btc_address, replace_id)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod vault_registry {
    use crate::{PolkaBTC, DOT};
    use btc_relay::BtcAddress;
    use frame_support::dispatch::{DispatchError, DispatchResult};

    pub fn replace_tokens<T: vault_registry::Config>(
        old_vault_id: T::AccountId,
        new_vault_id: T::AccountId,
        tokens: PolkaBTC<T>,
        collateral: DOT<T>,
    ) -> DispatchResult {
        <vault_registry::Module<T>>::replace_tokens(
            &old_vault_id,
            &new_vault_id,
            tokens,
            collateral,
        )
    }

    pub fn decrease_to_be_redeemed_tokens<T: vault_registry::Config>(
        vault_id: T::AccountId,
        tokens: PolkaBTC<T>,
    ) -> DispatchResult {
        <vault_registry::Module<T>>::decrease_to_be_redeemed_tokens(&vault_id, tokens)
    }

    pub fn get_active_vault_from_id<T: vault_registry::Config>(
        vault_id: &T::AccountId,
    ) -> Result<
        vault_registry::types::Vault<T::AccountId, T::BlockNumber, PolkaBTC<T>>,
        DispatchError,
    > {
        <vault_registry::Module<T>>::get_active_vault_from_id(vault_id)
    }

    pub fn increase_to_be_redeemed_tokens<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        tokens: PolkaBTC<T>,
    ) -> DispatchResult {
        <vault_registry::Module<T>>::increase_to_be_redeemed_tokens(vault_id, tokens)
    }

    pub fn is_over_minimum_collateral<T: vault_registry::Config>(collateral: DOT<T>) -> bool {
        <vault_registry::Module<T>>::is_over_minimum_collateral(collateral)
    }

    pub fn is_vault_below_auction_threshold<T: vault_registry::Config>(
        vault_id: T::AccountId,
    ) -> Result<bool, DispatchError> {
        <vault_registry::Module<T>>::is_vault_below_auction_threshold(&vault_id)
    }

    pub fn is_collateral_below_secure_threshold<T: vault_registry::Config>(
        collateral: DOT<T>,
        btc_amount: PolkaBTC<T>,
    ) -> Result<bool, DispatchError> {
        <vault_registry::Module<T>>::is_collateral_below_secure_threshold(collateral, btc_amount)
    }

    pub fn ensure_not_banned<T: vault_registry::Config>(
        vault: &T::AccountId,
        height: T::BlockNumber,
    ) -> DispatchResult {
        <vault_registry::Module<T>>::_ensure_not_banned(vault, height)
    }

    pub fn insert_vault_deposit_address<T: vault_registry::Config>(
        vault_id: &T::AccountId,
        btc_address: BtcAddress,
    ) -> DispatchResult {
        <vault_registry::Module<T>>::insert_vault_deposit_address(vault_id, btc_address)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod collateral {
    use crate::DOT;
    use frame_support::dispatch::DispatchResult;

    pub fn get_collateral_from_account<T: collateral::Config>(account: T::AccountId) -> DOT<T> {
        <collateral::Module<T>>::get_collateral_from_account(&account)
    }

    pub fn release_collateral<T: collateral::Config>(
        sender: T::AccountId,
        amount: DOT<T>,
    ) -> DispatchResult {
        <collateral::Module<T>>::release_collateral(&sender, amount)
    }

    pub fn slash_collateral<T: collateral::Config>(
        old_vault_id: T::AccountId,
        new_vault_id: T::AccountId,
        collateral: DOT<T>,
    ) -> DispatchResult {
        <collateral::Module<T>>::slash_collateral(old_vault_id, new_vault_id, collateral)
    }

    pub fn lock_collateral<T: collateral::Config>(
        sender: T::AccountId,
        amount: DOT<T>,
    ) -> DispatchResult {
        <collateral::Module<T>>::lock_collateral(&sender, amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod security {
    use frame_support::dispatch::DispatchResult;
    use primitive_types::H256;

    pub fn get_secure_id<T: security::Config>(id: &T::AccountId) -> H256 {
        <security::Module<T>>::get_secure_id(id)
    }

    pub fn ensure_parachain_status_running<T: security::Config>() -> DispatchResult {
        <security::Module<T>>::ensure_parachain_status_running()
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod oracle {
    use crate::types::{PolkaBTC, DOT};
    use frame_support::dispatch::DispatchError;

    pub fn btc_to_dots<T: exchange_rate_oracle::Config>(
        amount: PolkaBTC<T>,
    ) -> Result<DOT<T>, DispatchError> {
        <exchange_rate_oracle::Module<T>>::btc_to_dots(amount)
    }
}

#[cfg_attr(test, mockable)]
pub(crate) mod fee {
    use crate::types::DOT;
    use frame_support::dispatch::DispatchError;

    pub fn get_replace_griefing_collateral<T: fee::Config>(
        amount: DOT<T>,
    ) -> Result<DOT<T>, DispatchError> {
        <fee::Module<T>>::get_replace_griefing_collateral(amount)
    }

    pub fn get_auction_redeem_fee<T: fee::Config>(amount: DOT<T>) -> Result<DOT<T>, DispatchError> {
        <fee::Module<T>>::get_auction_redeem_fee(amount)
    }
}
