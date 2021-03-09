use super::*;
use crate::types::ReplaceRequest;
use crate::Module as Replace;
use bitcoin::formatter::{Formattable, TryFormattable};
use bitcoin::types::{
    BlockBuilder, RawBlockHeader, TransactionBuilder, TransactionInputBuilder, TransactionOutput,
};
use btc_relay::Module as BtcRelay;
use btc_relay::{BtcAddress, BtcPublicKey};
use exchange_rate_oracle::Module as ExchangeRateOracle;
use frame_benchmarking::{account, benchmarks};
use frame_system::Module as System;
use frame_system::RawOrigin;
use sp_core::{H160, H256, U256};
use sp_runtime::FixedPointNumber;
use sp_std::prelude::*;
use vault_registry::types::{Vault, Wallet};
use vault_registry::Module as VaultRegistry;

fn dummy_public_key() -> BtcPublicKey {
    BtcPublicKey([
        2, 205, 114, 218, 156, 16, 235, 172, 106, 37, 18, 153, 202, 140, 176, 91, 207, 51, 187, 55,
        18, 45, 222, 180, 119, 54, 243, 97, 173, 150, 161, 169, 230,
    ])
}

benchmarks! {
    request_replace {
        let vault_id: T::AccountId = account("Vault", 0, 0);
        let amount = Replace::<T>::replace_btc_dust_value() + 1000u32.into();
        // TODO: calculate from exchange rate
        let griefing = 1000u32.into();

        let mut vault = Vault::default();
        vault.id = vault_id.clone();
        vault.wallet = Wallet::new(dummy_public_key());
        vault.issued_tokens = amount;
        VaultRegistry::<T>::insert_vault(
            &vault_id,
            vault
        );

    }: _(RawOrigin::Signed(vault_id), amount, griefing)

    withdraw_replace {
        let vault_id: T::AccountId = account("Vault", 0, 0);

        let mut vault = Vault::default();
        vault.id = vault_id.clone();
        vault.wallet = Wallet::new(dummy_public_key());
        VaultRegistry::<T>::insert_vault(
            &vault_id,
            vault
        );
        let threshold = <T as vault_registry::Config>::UnsignedFixedPoint::one();
        VaultRegistry::<T>::set_auction_collateral_threshold(threshold);

        let replace_id = H256::zero();
        let mut replace_request = ReplaceRequest::default();
        replace_request.old_vault = vault_id.clone();
        Replace::<T>::insert_replace_request(replace_id, replace_request);

    }: _(RawOrigin::Signed(vault_id), replace_id)

    accept_replace {
        let old_vault_id: T::AccountId = account("Origin", 0, 0);
        let new_vault_id: T::AccountId = account("Vault", 0, 0);
        let amount: u32 = 100;
        let collateral: u32 = 1000;

        let new_vault_btc_address = BtcAddress::P2SH(H160([0; 20]));

        VaultRegistry::<T>::set_secure_collateral_threshold(<T as vault_registry::Config>::UnsignedFixedPoint::checked_from_rational(1, 100000).unwrap());
        ExchangeRateOracle::<T>::_set_exchange_rate(<T as exchange_rate_oracle::Config>::UnsignedFixedPoint::one()).unwrap();

        VaultRegistry::<T>::_register_vault(&old_vault_id, 100000000u32.into(), dummy_public_key()).unwrap();

        VaultRegistry::<T>::try_increase_to_be_issued_tokens(&old_vault_id, amount.into()).unwrap();
        VaultRegistry::<T>::issue_tokens(&old_vault_id, amount.into()).unwrap();
        VaultRegistry::<T>::try_increase_to_be_replaced_tokens(&old_vault_id, amount.into()).unwrap();

        VaultRegistry::<T>::_register_vault(&new_vault_id, 100000000u32.into(), dummy_public_key()).unwrap();

        let replace_id = H256::zero();
        let mut replace_request = ReplaceRequest::default();
        replace_request.old_vault = old_vault_id.clone();
        replace_request.amount = amount.into();
        Replace::<T>::insert_replace_request(replace_id, replace_request);

    }: _(RawOrigin::Signed(new_vault_id), replace_id, collateral.into(), new_vault_btc_address)

    auction_replace {
        let old_vault_id: T::AccountId = account("Origin", 0, 0);
        let new_vault_id: T::AccountId = account("Vault", 0, 0);
        let btc_amount: u32 = 100;
        let collateral: u32 = 1000;

        let new_vault_btc_address = BtcAddress::P2SH(H160([0; 20]));

        ExchangeRateOracle::<T>::_set_exchange_rate(<T as exchange_rate_oracle::Config>::UnsignedFixedPoint::one()).unwrap();

        VaultRegistry::<T>::_register_vault(&old_vault_id, 50u32.into(), dummy_public_key()).unwrap();
        VaultRegistry::<T>::_register_vault(&new_vault_id, 50u32.into(), dummy_public_key()).unwrap();

        VaultRegistry::<T>::set_secure_collateral_threshold(<T as vault_registry::Config>::UnsignedFixedPoint::checked_from_rational(1, 100000).unwrap()); // 0.001%
        VaultRegistry::<T>::set_auction_collateral_threshold(<T as vault_registry::Config>::UnsignedFixedPoint::checked_from_rational(10000, 100).unwrap()); // 10000%

        VaultRegistry::<T>::try_increase_to_be_issued_tokens(&old_vault_id, btc_amount.into()).unwrap();
        VaultRegistry::<T>::issue_tokens(&old_vault_id, btc_amount.into()).unwrap();
    }: _(RawOrigin::Signed(new_vault_id), old_vault_id, btc_amount.into(), collateral.into(), new_vault_btc_address)

    execute_replace {
        let new_vault_id: T::AccountId = account("Origin", 0, 0);
        let old_vault_id: T::AccountId = account("Vault", 0, 0);
        let relayer_id: T::AccountId = account("Relayer", 0, 0);

        BtcRelay::<T>::register_authorized_relayer(relayer_id.clone());

        let new_vault_btc_address = BtcAddress::P2SH(H160([0; 20]));
        let old_vault_btc_address = BtcAddress::P2SH(H160([1; 20]));

        let replace_id = H256::zero();
        let mut replace_request = ReplaceRequest::default();
        replace_request.old_vault = old_vault_id.clone();
        replace_request.new_vault = Some(new_vault_id.clone());
        replace_request.btc_address = Some(old_vault_btc_address);
        Replace::<T>::insert_replace_request(replace_id, replace_request);

        let mut old_vault = Vault::default();
        old_vault.id = old_vault_id.clone();
        old_vault.wallet = Wallet::new(dummy_public_key());
        VaultRegistry::<T>::insert_vault(
            &old_vault_id,
            old_vault
        );

        let mut new_vault = Vault::default();
        new_vault.id = new_vault_id.clone();
        new_vault.wallet = Wallet::new(dummy_public_key());
        VaultRegistry::<T>::insert_vault(
            &new_vault_id,
            new_vault
        );

        let height = 0;
        let block = BlockBuilder::new()
            .with_version(2)
            .with_coinbase(&new_vault_btc_address, 50, 3)
            .with_timestamp(1588813835)
            .mine(U256::from(2).pow(254.into())).unwrap();

        let block_hash = block.header.hash().unwrap();
        let block_header = RawBlockHeader::from_bytes(&block.header.try_format().unwrap()).unwrap();
        BtcRelay::<T>::_initialize(relayer_id.clone(), block_header, height).unwrap();

        let value = 0;
        let transaction = TransactionBuilder::new()
            .with_version(2)
            .add_input(
                TransactionInputBuilder::new()
                    .with_coinbase(false)
                    .with_previous_hash(block.transactions[0].hash())
                    .with_script(&[
                        0, 71, 48, 68, 2, 32, 91, 128, 41, 150, 96, 53, 187, 63, 230, 129, 53, 234,
                        210, 186, 21, 187, 98, 38, 255, 112, 30, 27, 228, 29, 132, 140, 155, 62, 123,
                        216, 232, 168, 2, 32, 72, 126, 179, 207, 142, 8, 99, 8, 32, 78, 244, 166, 106,
                        160, 207, 227, 61, 210, 172, 234, 234, 93, 59, 159, 79, 12, 194, 240, 212, 3,
                        120, 50, 1, 71, 81, 33, 3, 113, 209, 131, 177, 9, 29, 242, 229, 15, 217, 247,
                        165, 78, 111, 80, 79, 50, 200, 117, 80, 30, 233, 210, 167, 133, 175, 62, 253,
                        134, 127, 212, 51, 33, 2, 128, 200, 184, 235, 148, 25, 43, 34, 28, 173, 55, 54,
                        189, 164, 187, 243, 243, 152, 7, 84, 210, 85, 156, 238, 77, 97, 188, 240, 162,
                        197, 105, 62, 82, 174,
                    ])
                    .build(),
            )
            .add_output(TransactionOutput::payment(value.into(), &old_vault_btc_address))
            .add_output(TransactionOutput::op_return(0, H256::zero().as_bytes()))
            .build();

        let block = BlockBuilder::new()
            .with_previous_hash(block_hash)
            .with_version(2)
            .with_coinbase(&new_vault_btc_address, 50, 3)
            .with_timestamp(1588813835)
            .add_transaction(transaction.clone())
            .mine(U256::from(2).pow(254.into())).unwrap();

        let tx_id = transaction.tx_id();
        let proof = block.merkle_proof(&vec![tx_id]).unwrap().try_format().unwrap();
        let raw_tx = transaction.format_with(true);

        let block_header = RawBlockHeader::from_bytes(&block.header.try_format().unwrap()).unwrap();
        BtcRelay::<T>::_store_block_header(relayer_id, block_header).unwrap();

    }: _(RawOrigin::Signed(old_vault_id), replace_id, tx_id, proof, raw_tx)

    cancel_replace {
        let new_vault_id: T::AccountId = account("Origin", 0, 0);
        let old_vault_id: T::AccountId = account("Vault", 0, 0);
        let amount:u32 = 100;

        let replace_id = H256::zero();
        let mut replace_request = ReplaceRequest::default();
        replace_request.old_vault = old_vault_id.clone();
        replace_request.new_vault = Some(new_vault_id.clone());
        replace_request.amount = amount.into();
        Replace::<T>::insert_replace_request(replace_id, replace_request);
        System::<T>::set_block_number(System::<T>::block_number() + Replace::<T>::replace_period() + 10u32.into());

        VaultRegistry::<T>::set_secure_collateral_threshold(<T as vault_registry::Config>::UnsignedFixedPoint::checked_from_rational(1, 100000).unwrap());
        ExchangeRateOracle::<T>::_set_exchange_rate(<T as exchange_rate_oracle::Config>::UnsignedFixedPoint::one()).unwrap();

        VaultRegistry::<T>::_register_vault(&old_vault_id, 100000000u32.into(), dummy_public_key()).unwrap();
        VaultRegistry::<T>::try_increase_to_be_issued_tokens(&old_vault_id, amount.into()).unwrap();
        VaultRegistry::<T>::issue_tokens(&old_vault_id, amount.into()).unwrap();
        VaultRegistry::<T>::try_increase_to_be_redeemed_tokens(&old_vault_id, amount.into()).unwrap();

        VaultRegistry::<T>::_register_vault(&new_vault_id, 100000000u32.into(), dummy_public_key()).unwrap();
        VaultRegistry::<T>::try_increase_to_be_issued_tokens(&new_vault_id, amount.into()).unwrap();

    }: _(RawOrigin::Signed(new_vault_id), replace_id)

    set_replace_period {
    }: _(RawOrigin::Root, 1u32.into())

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{ExtBuilder, Test};
    use frame_support::assert_ok;

    #[test]
    fn test_benchmarks() {
        ExtBuilder::build_with(
            pallet_balances::GenesisConfig::<Test, pallet_balances::Instance1> {
                balances: vec![
                    (account("Origin", 0, 0), 1 << 32),
                    (account("Vault", 0, 0), 1 << 32),
                ],
            },
        )
        .execute_with(|| {
            assert_ok!(test_benchmark_request_replace::<Test>());
            assert_ok!(test_benchmark_withdraw_replace::<Test>());
            assert_ok!(test_benchmark_accept_replace::<Test>());
            assert_ok!(test_benchmark_auction_replace::<Test>());
            assert_ok!(test_benchmark_execute_replace::<Test>());
            assert_ok!(test_benchmark_cancel_replace::<Test>());
            assert_ok!(test_benchmark_set_replace_period::<Test>());
        });
    }
}
