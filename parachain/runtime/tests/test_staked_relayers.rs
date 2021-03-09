mod mock;

use mock::*;
use primitive_types::H256;
use sp_runtime::traits::CheckedMul;
use vault_registry::Vault;

type StakedRelayersCall = staked_relayers::Call<Runtime>;
type StakedRelayersModule = staked_relayers::Module<Runtime>;

#[test]
fn integration_test_report_vault_theft() {
    ExtBuilder::build().execute_with(|| {
        let user = ALICE;
        let vault = BOB;
        let amount = 100;
        let collateral_vault = 1000000;

        let vault_btc_address = BtcAddress::P2SH(H160([
            215, 255, 109, 96, 235, 244, 10, 155, 24, 134, 172, 206, 6, 101, 59, 162, 34, 77, 143,
            234,
        ]));
        let other_btc_address = BtcAddress::P2SH(H160([1; 20]));

        SystemModule::set_block_number(1);

        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));
        VaultRegistryModule::insert_vault(&account_of(LIQUIDATION_VAULT), Vault::default());
        // assert_ok!(CollateralModule::lock_collateral(&account_of(vault), collateral_vault));
        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            collateral_vault,
            dummy_public_key()
        ))
        .dispatch(origin_of(account_of(vault))));
        assert_ok!(VaultRegistryModule::insert_vault_deposit_address(
            &account_of(vault),
            vault_btc_address
        ));

        // register as staked relayer
        assert_ok!(
            Call::StakedRelayers(StakedRelayersCall::register_staked_relayer(100))
                .dispatch(origin_of(account_of(user)))
        );

        SystemModule::set_block_number(StakedRelayersModule::get_maturity_period() + 100);

        // manually activate
        assert_ok!(StakedRelayersModule::activate_staked_relayer(&account_of(
            user
        )));

        let initial_sla = SlaModule::relayer_sla(account_of(ALICE));

        let (tx_id, _height, proof, raw_tx) = generate_transaction_and_mine_with_script_sig(
            other_btc_address,
            amount,
            Some(H256::zero()),
            &[
                0, 71, 48, 68, 2, 32, 91, 128, 41, 150, 96, 53, 187, 63, 230, 129, 53, 234, 210,
                186, 21, 187, 98, 38, 255, 112, 30, 27, 228, 29, 132, 140, 155, 62, 123, 216, 232,
                168, 2, 32, 72, 126, 179, 207, 142, 8, 99, 8, 32, 78, 244, 166, 106, 160, 207, 227,
                61, 210, 172, 234, 234, 93, 59, 159, 79, 12, 194, 240, 212, 3, 120, 50, 1, 71, 81,
                33, 3, 113, 209, 131, 177, 9, 29, 242, 229, 15, 217, 247, 165, 78, 111, 80, 79, 50,
                200, 117, 80, 30, 233, 210, 167, 133, 175, 62, 253, 134, 127, 212, 51, 33, 2, 128,
                200, 184, 235, 148, 25, 43, 34, 28, 173, 55, 54, 189, 164, 187, 243, 243, 152, 7,
                84, 210, 85, 156, 238, 77, 97, 188, 240, 162, 197, 105, 62, 82, 174,
            ],
        );

        // check sla increase for the block submission. The call above will have submitted 7 blocks
        // (the actual transaction, plus 6 confirmations)
        let mut expected_sla = initial_sla
            + FixedI128::checked_from_integer(7)
                .unwrap()
                .checked_mul(&SlaModule::relayer_block_submission())
                .unwrap();
        assert_eq!(SlaModule::relayer_sla(account_of(ALICE)), expected_sla);

        SystemModule::set_block_number(1000);

        assert_ok!(Call::StakedRelayers(StakedRelayersCall::report_vault_theft(
            account_of(vault),
            tx_id,
            proof,
            raw_tx
        ))
        .dispatch(origin_of(account_of(user))));

        // check sla increase for the theft report
        expected_sla = expected_sla + SlaModule::relayer_correct_theft_report();
        assert_eq!(SlaModule::relayer_sla(account_of(ALICE)), expected_sla);
    });
}

#[test]
fn integration_test_report_vault_under_liquidation_threshold() {
    ExtBuilder::build().execute_with(|| {
        let relayer = ALICE;
        let vault = BOB;
        let user = CAROL;
        let amount = 100;
        let collateral_vault = 1000;

        SystemModule::set_block_number(1);

        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));
        VaultRegistryModule::insert_vault(&account_of(LIQUIDATION_VAULT), Vault::default());

        increase_issued(user, vault, collateral_vault, amount);

        // register as staked relayer
        assert_ok!(
            Call::StakedRelayers(StakedRelayersCall::register_staked_relayer(100))
                .dispatch(origin_of(account_of(relayer)))
        );

        SystemModule::set_block_number(StakedRelayersModule::get_maturity_period() + 100);

        // manually activate
        assert_ok!(StakedRelayersModule::activate_staked_relayer(&account_of(
            relayer
        )));

        let initial_sla = SlaModule::relayer_sla(account_of(relayer));

        // make vault to be undercollateralized
        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::checked_from_integer(100000).unwrap()
        ));

        assert_ok!(Call::StakedRelayers(
            StakedRelayersCall::report_vault_under_liquidation_threshold(account_of(vault))
        )
        .dispatch(origin_of(account_of(relayer))));

        // check sla increase for the theft report
        let expected_sla = initial_sla + SlaModule::relayer_correct_liquidation_report();
        assert_eq!(SlaModule::relayer_sla(account_of(relayer)), expected_sla);
    });
}
