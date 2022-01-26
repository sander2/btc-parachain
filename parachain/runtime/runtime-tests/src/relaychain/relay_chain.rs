mod kintsugi_tests {
	use crate::relaychain::kusama_test_net::*;
	use crate::setup::*;

	use frame_support::{assert_noop, assert_ok};

	use codec::Decode;
	// use module_relaychain::RelayChainCallBuilder;
	// use module_support::CallBuilder;
	use xcm_emulator::TestExt;

	// type KusamaCallBuilder = RelayChainCallBuilder<Runtime, ParachainInfo>;
// 
// 	#[test]
// 	/// Tests the staking_withdraw_unbonded call.
// 	/// Also tests utility_as_derivative call.
// 	fn relaychain_staking_withdraw_unbonded_works() {
// 		let homa_lite_sub_account: AccountId =
// 			hex_literal::hex!["d7b8926b326dd349355a9a7cca6606c1e0eb6fd2b506066b518c7155ff0d8297"].into();
// 		KusamaNet::execute_with(|| {
// 			kusama_runtime::Staking::trigger_new_era(0, vec![]);
// 
// 			// Transfer some KSM into the parachain.
// 			assert_ok!(kusama_runtime::Balances::transfer(
// 				kusama_runtime::Origin::signed(ALICE.into()),
// 				MultiAddress::Id(homa_lite_sub_account.clone()),
// 				1_001_000_000_000_000
// 			));
// 
// 			kusama_runtime::System::set_block_number(100);
// 
// 			// Kusama's unbonding period is 7 days = 7 * 3600 / 6 = 100_800 blocks
// 			kusama_runtime::System::set_block_number(101_000);
// 			// Kusama: 6 hours per era. 7 days = 4 * 7 = 28 eras.
// 			for _i in 0..29 {
// 				kusama_runtime::Staking::trigger_new_era(0, vec![]);
// 			}
// 
// 			assert_eq!(
// 				kusama_runtime::Balances::free_balance(&homa_lite_sub_account.clone()),
// 				1_001_000_000_000_000
// 			);
// 
// 			// Transfer fails because liquidity is locked.
// 			assert_noop!(
// 				kusama_runtime::Balances::transfer(
// 					kusama_runtime::Origin::signed(homa_lite_sub_account.clone()),
// 					MultiAddress::Id(ALICE.into()),
// 					1_000_000_000_000_000
// 				),
// 				pallet_balances::Error::<kusama_runtime::Runtime>::LiquidityRestrictions
// 			);
// 
// 			// Uncomment this to test if withdraw_unbonded and transfer_keep_alive
// 			// work without XCM. Used to isolate error when the test fails.
// 			// assert_ok!(kusama_runtime::Staking::withdraw_unbonded(
// 			// 	kusama_runtime::Origin::signed(homa_lite_sub_account.clone()),
// 			// 	5
// 			// ));
// 		});
// 
// 		Kintsugi::execute_with(|| {
// 			// Call withdraw_unbonded as the homa-lite subaccount
// // 			let xcm_message =
// // 				KusamaCallBuilder::utility_as_derivative_call(KusamaCallBuilder::staking_withdraw_unbonded(5), 0);
// // 
// // 			let msg = KusamaCallBuilder::finalize_call_into_xcm_message(xcm_message, 600_000_000, 10_000_000_000);
// // 
// // 			// Withdraw unbonded
// // 			assert_ok!(pallet_xcm::Pallet::<Runtime>::send_xcm(Here, Parent, msg));
// 		});
// 
// 		KusamaNet::execute_with(|| {
// 			assert_eq!(
// 				kusama_runtime::Balances::free_balance(&homa_lite_sub_account.clone()),
// 				1_001_000_000_000_000
// 			);
// 
// 			// Transfer fails because liquidity is locked.
// 			assert_ok!(
// 				kusama_runtime::Balances::transfer(
// 					kusama_runtime::Origin::signed(homa_lite_sub_account.clone()),
// 					MultiAddress::Id(ALICE.into()),
// 					1_000_000_000_000_000
// 				) //kusama_runtime::Balances::Error::<Runtime>::LiquidityLocked,
// 			);
// 			assert_eq!(
// 				kusama_runtime::Balances::free_balance(&homa_lite_sub_account.clone()),
// 				1_000_000_000_000
// 			);
// 		});
// 	}


}
