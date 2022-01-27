// This file is part of Acala.

// Copyright (C) 2020-2022 Acala Foundation.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Cross-chain transfer tests within Kusama network.

use crate::{relaychain::kusama_test_net::*, setup::*};

use frame_support::assert_ok;

use orml_traits::MultiCurrency;
use primitives::{CurrencyId::Token, CurrencyInfo};
use xcm_emulator::TestExt;

#[test]
fn transfer_from_relay_chain() {
    KusamaNet::execute_with(|| {
        assert_ok!(kusama_runtime::XcmPallet::reserve_transfer_assets(
            kusama_runtime::Origin::signed(ALICE.into()),
            Box::new(Parachain(2092).into().into()),
            Box::new(
                Junction::AccountId32 {
                    id: BOB,
                    network: NetworkId::Any
                }
                .into()
                .into()
            ),
            Box::new((Here, KSM.one()).into()),
            0
        ));
    });

    Kintsugi::execute_with(|| {
        assert_eq!(Tokens::free_balance(Token(KSM), &AccountId::from(BOB)), 999_999_360_000);
    });
}

#[test]
fn transfer_to_relay_chain() {
    Kintsugi::execute_with(|| {
        assert_ok!(XTokens::transfer(
            Origin::signed(ALICE.into()),
            Token(KSM),
            KSM.one(),
            Box::new(
                MultiLocation::new(
                    1,
                    X1(Junction::AccountId32 {
                        id: BOB,
                        network: NetworkId::Any,
                    })
                )
                .into()
            ),
            4_000_000_000
        ));
    });

    KusamaNet::execute_with(|| {
        assert_eq!(
            kusama_runtime::Balances::free_balance(&AccountId::from(BOB)),
            999_893_333_340
        );
    });
}

#[test]
fn transfer_to_sibling() {
    TestNet::reset();

    fn sibling_sovereign_account() -> AccountId {
        use sp_runtime::traits::AccountIdConversion;
        polkadot_parachain::primitives::Sibling::from(2001).into_account()
    }

    Kintsugi::execute_with(|| {
        assert_ok!(Tokens::deposit(
            Token(KINT),
            &AccountId::from(ALICE),
            100_000_000_000_000
        ));
    });

    Kintsugi::execute_with(|| {
        assert_ok!(XTokens::transfer(
            Origin::signed(ALICE.into()),
            Token(KINT),
            10_000_000_000_000,
            Box::new(
                MultiLocation::new(
                    1,
                    X2(
                        Parachain(2001),
                        Junction::AccountId32 {
                            network: NetworkId::Any,
                            id: BOB.into(),
                        }
                    )
                )
                .into()
            ),
            1_000_000_000,
        ));

        assert_eq!(
            Tokens::free_balance(Token(KINT), &AccountId::from(ALICE)),
            90_000_000_000_000
        );

        assert_eq!(
            Tokens::free_balance(Token(KINT), &sibling_sovereign_account()),
            10_000_000_000_000
        );
    });

    Sibling::execute_with(|| {
        assert_ok!(XTokens::transfer_multiasset(
            Origin::signed(BOB.into()),
            Box::new(
                MultiAsset {
                    id: Concrete(MultiLocation::new(1, X2(Parachain(2092), GeneralKey(Token(KINT).encode()))).into()),
                    fun: Fungibility::Fungible(5_000_000_000_000),
                }
                .into()
            ),
            Box::new(
                MultiLocation::new(
                    1,
                    X2(
                        Parachain(2092),
                        Junction::AccountId32 {
                            network: NetworkId::Any,
                            id: ALICE.into(),
                        }
                    )
                )
                .into()
            ),
            1_000_000_000,
        ));
    });

    Kintsugi::execute_with(|| {
        let xcm_fee = 853_333;
        assert_eq!(
            Tokens::free_balance(Token(KINT), &AccountId::from(ALICE)),
            95_000_000_000_000 - xcm_fee
        );

        assert_eq!(
            Tokens::free_balance(Token(KINT), &KintsugiTreasuryAccount::get()),
            xcm_fee
        );

        assert_eq!(
            Tokens::free_balance(Token(KINT), &sibling_sovereign_account()),
            5_000_000_000_000
        );
    });
}
//
// #[test]
// fn transfer_from_relay_chain_deposit_to_treasury_if_below_ed() {
// 	KusamaNet::execute_with(|| {
// 		assert_ok!(kusama_runtime::XcmPallet::reserve_transfer_assets(
// 			kusama_runtime::Origin::signed(ALICE.into()),
// 			Box::new(Parachain(2092).into().into()),
// 			Box::new(
// 				Junction::AccountId32 {
// 					id: BOB,
// 					network: NetworkId::Any
// 				}
// 				.into()
// 				.into()
// 			),
// 			Box::new((Here, 128_000_111).into()),
// 			0
// 		));
// 	});
//
// 	Kintsugi::execute_with(|| {
// 		assert_eq!(Tokens::free_balance(KSM, &AccountId::from(BOB)), 0);
// 		assert_eq!(
// 			Tokens::free_balance(KSM, &kintsugi_runtime_parachain::KintsugiTreasuryAccount::get()),
// 			1_000_128_000_111
// 		);
// 	});
// }
//
#[test]
fn xcm_transfer_execution_barrier_trader_works() {
    let expect_weight_limit = 600_000_000;
    let weight_limit_too_low = 500_000_000;
    let unit_instruction_weight = 200_000_000;

    // relay-chain use normal account to send xcm, destination para-chain can't pass Barrier check
    let message = Xcm(vec![
        ReserveAssetDeposited((Parent, 100).into()),
        BuyExecution {
            fees: (Parent, 100).into(),
            weight_limit: Unlimited,
        },
        DepositAsset {
            assets: All.into(),
            max_assets: 1,
            beneficiary: Here.into(),
        },
    ]);
    KusamaNet::execute_with(|| {
        let r = pallet_xcm::Pallet::<kusama_runtime::Runtime>::send(
            kusama_runtime::Origin::signed(ALICE.into()),
            Box::new(Parachain(2092).into().into()),
            Box::new(xcm::VersionedXcm::from(message)),
        );
        assert_ok!(r);
    });
    Kintsugi::execute_with(|| {
        assert!(System::events().iter().any(|r| matches!(
            r.event,
            Event::DmpQueue(cumulus_pallet_dmp_queue::Event::ExecutedDownward(
                _,
                Outcome::Error(XcmError::Barrier)
            ))
        )));
    });

    // AllowTopLevelPaidExecutionFrom barrier test case:
    // para-chain use XcmExecutor `execute_xcm()` method to execute xcm.
    // if `weight_limit` in BuyExecution is less than `xcm_weight(max_weight)`, then Barrier can't pass.
    // other situation when `weight_limit` is `Unlimited` or large than `xcm_weight`, then it's ok.
    let message = Xcm::<kintsugi_runtime_parachain::Call>(vec![
        ReserveAssetDeposited((Parent, 100).into()),
        BuyExecution {
            fees: (Parent, 100).into(),
            weight_limit: Limited(weight_limit_too_low),
        },
        DepositAsset {
            assets: All.into(),
            max_assets: 1,
            beneficiary: Here.into(),
        },
    ]);
    Kintsugi::execute_with(|| {
        let r = XcmExecutor::<XcmConfig>::execute_xcm(Parent, message, expect_weight_limit);
        assert_eq!(r, Outcome::Error(XcmError::Barrier));
    });

    // trader inside BuyExecution have TooExpensive error if payment less than calculated weight amount.
    // the minimum of calculated weight amount(`FixedRateOfFungible<KsmPerSecond>`) is 96_000_000
    let message = Xcm::<kintsugi_runtime_parachain::Call>(vec![
        ReserveAssetDeposited((Parent, 95_999_999).into()),
        BuyExecution {
            fees: (Parent, 95_999_999).into(),
            weight_limit: Limited(expect_weight_limit),
        },
        DepositAsset {
            assets: All.into(),
            max_assets: 1,
            beneficiary: Here.into(),
        },
    ]);
    Kintsugi::execute_with(|| {
        let r = XcmExecutor::<XcmConfig>::execute_xcm(Parent, message, expect_weight_limit);
        assert_eq!(
            r,
            Outcome::Incomplete(expect_weight_limit - unit_instruction_weight, XcmError::TooExpensive)
        );
    });

    // all situation fulfilled, execute success
    let message = Xcm::<kintsugi_runtime_parachain::Call>(vec![
        ReserveAssetDeposited((Parent, 96_000_000).into()),
        BuyExecution {
            fees: (Parent, 96_000_000).into(),
            weight_limit: Limited(expect_weight_limit),
        },
        DepositAsset {
            assets: All.into(),
            max_assets: 1,
            beneficiary: Here.into(),
        },
    ]);
    Kintsugi::execute_with(|| {
        let r = XcmExecutor::<XcmConfig>::execute_xcm(Parent, message, expect_weight_limit);
        assert_eq!(r, Outcome::Complete(expect_weight_limit));
    });
}

#[test]
fn subscribe_version_notify_works() {
    // relay chain subscribe version notify of para chain
    KusamaNet::execute_with(|| {
        let r = pallet_xcm::Pallet::<kusama_runtime::Runtime>::force_subscribe_version_notify(
            kusama_runtime::Origin::root(),
            Box::new(Parachain(2092).into().into()),
        );
        assert_ok!(r);
    });
    KusamaNet::execute_with(|| {
        kusama_runtime::System::assert_has_event(kusama_runtime::Event::XcmPallet(
            pallet_xcm::Event::SupportedVersionChanged(
                MultiLocation {
                    parents: 0,
                    interior: X1(Parachain(2092)),
                },
                2,
            ),
        ));
    });

    // para chain subscribe version notify of relay chain
    Kintsugi::execute_with(|| {
        let r = pallet_xcm::Pallet::<kintsugi_runtime_parachain::Runtime>::force_subscribe_version_notify(
            Origin::root(),
            Box::new(Parent.into()),
        );
        assert_ok!(r);
    });
    Kintsugi::execute_with(|| {
        System::assert_has_event(kintsugi_runtime_parachain::Event::PolkadotXcm(
            pallet_xcm::Event::SupportedVersionChanged(
                MultiLocation {
                    parents: 1,
                    interior: Here,
                },
                2,
            ),
        ));
    });

    // para chain subscribe version notify of sibling chain
    Kintsugi::execute_with(|| {
        let r = pallet_xcm::Pallet::<kintsugi_runtime_parachain::Runtime>::force_subscribe_version_notify(
            Origin::root(),
            Box::new((Parent, Parachain(2001)).into()),
        );
        assert_ok!(r);
    });
    Kintsugi::execute_with(|| {
        assert!(kintsugi_runtime_parachain::System::events().iter().any(|r| matches!(
            r.event,
            kintsugi_runtime_parachain::Event::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent(Some(_)))
        )));
    });
    Sibling::execute_with(|| {
        assert!(System::events().iter().any(|r| matches!(
            r.event,
            kintsugi_runtime_parachain::Event::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent(Some(_)))
                | kintsugi_runtime_parachain::Event::XcmpQueue(cumulus_pallet_xcmp_queue::Event::Success(Some(_)))
        )));
    });
}

#[test]
fn trap_assets_larger_than_ed_works() {
    TestNet::reset();

    let mut kint_treasury_amount = 0;
    let (ksm_asset_amount, kar_asset_amount) = (KSM.one(), KINT.one());
    let trader_weight_to_treasury: u128 = 96_000_000;

    Kintsugi::execute_with(|| {
        assert_ok!(Tokens::deposit(Token(KSM), &AccountId::from(DEFAULT), 100 * KSM.one()));
        assert_ok!(Tokens::deposit(
            Token(KINT),
            &AccountId::from(DEFAULT),
            100 * KINT.one()
        ));

        kint_treasury_amount = Tokens::free_balance(Token(KINT), &KintsugiTreasuryAccount::get());
    });

    let assets: MultiAsset = (Parent, ksm_asset_amount).into();
    KusamaNet::execute_with(|| {
        let xcm = vec![
            WithdrawAsset(assets.clone().into()),
            BuyExecution {
                fees: assets,
                weight_limit: Limited(KSM.one() as u64),
            },
            WithdrawAsset(
                (
                    (Parent, X2(Parachain(2092), GeneralKey(Token(KINT).encode()))),
                    kar_asset_amount,
                )
                    .into(),
            ),
        ];
        assert_ok!(pallet_xcm::Pallet::<kusama_runtime::Runtime>::send_xcm(
            Here,
            Parachain(2092).into(),
            Xcm(xcm),
        ));
    });
    let mut ticket: Option<MultiAssets> = None;
    Kintsugi::execute_with(|| {
        assert!(System::events()
            .iter()
            .any(|r| matches!(r.event, Event::PolkadotXcm(pallet_xcm::Event::AssetsTrapped(_, _, _)))));

        let event = System::events()
            .iter()
            .find(|r| matches!(r.event, Event::PolkadotXcm(pallet_xcm::Event::AssetsTrapped(_, _, _))))
			.cloned()
            .unwrap();

        use std::convert::TryFrom;
        use xcm::VersionedMultiAssets;
        ticket = match event.event {
            Event::PolkadotXcm(pallet_xcm::Event::AssetsTrapped(_, _, ticket)) => {
                Some(TryFrom::<VersionedMultiAssets>::try_from(ticket).unwrap())
            }
            _ => panic!("event not found"),
        };

        assert_eq!(
            trader_weight_to_treasury + KSM.one(),
            Tokens::free_balance(Token(KSM), &KintsugiTreasuryAccount::get())
        );
        assert_eq!(
            kint_treasury_amount,
            Tokens::free_balance(Token(KINT), &KintsugiTreasuryAccount::get())
        );
    });

    KusamaNet::execute_with(|| {
        let xcm = vec![
            ClaimAsset {
                // assets: (
                //     (Parent, X2(Parachain(2092), GeneralKey(Token(KINT).encode()))),
                //     kar_asset_amount,
                // )
                //     .into(),
					assets: ticket.unwrap(),
                ticket: Here.into(),
            },
            BuyExecution {
                fees: (
                    (Parent, X2(Parachain(2092), GeneralKey(Token(KINT).encode()))),
                    kar_asset_amount / 2,
                )
                    .into(),
                weight_limit: Limited(KSM.one() as u64),
            },
            DepositAsset {
                assets: All.into(),
                max_assets: 1,
                beneficiary: Junction::AccountId32 {
                    id: BOB,
                    network: NetworkId::Any,
                }
                .into(),
            },
        ];
        assert_ok!(pallet_xcm::Pallet::<kusama_runtime::Runtime>::send_xcm(
            Here,
            Parachain(2092).into(),
            Xcm(xcm),
        ));
    });

    Kintsugi::execute_with(|| {
        assert_eq!(
            Tokens::free_balance(Token(KINT), &AccountId::from(BOB)),
            90_000_000_000_000
        );
    });
}

// #[test]
// fn trap_assets_lower_than_ed_works() {
// 	TestNet::reset();
//
// 	let mut kint_treasury_amount = 0;
// 	let (ksm_asset_amount, kar_asset_amount) = (100, 100);
//
// 	Kintsugi::execute_with(|| {
// 		assert_ok!(Tokens::deposit(Token(KSM), &AccountId::from(DEFAULT), KSM.one()));
// 		assert_ok!(Tokens::deposit(Token(KINT), &AccountId::from(DEFAULT), KINT.one()));
// 		kint_treasury_amount = Tokens::free_balance(Token(KINT), &KintsugiTreasuryAccount::get());
// 	});
//
// 	let assets: MultiAsset = (Parent, ksm_asset_amount).into();
// 	KusamaNet::execute_with(|| {
// 		let xcm = vec![
// 			WithdrawAsset(assets.clone().into()),
// 			BuyExecution {
// 				fees: assets,
// 				weight_limit: Limited(KSM.one() as u64),
// 			},
// 			WithdrawAsset(
// 				(
// 					(Parent, X2(Parachain(2092), GeneralKey(Token(KINT).encode()))),
// 					kar_asset_amount,
// 				)
// 					.into(),
// 			),
// 			// two asset left in holding register, they both lower than ED, so goes to treasury.
// 		];
// 		assert_ok!(pallet_xcm::Pallet::<kusama_runtime::Runtime>::send_xcm(
// 			Here,
// 			Parachain(2092).into(),
// 			Xcm(xcm),
// 		));
// 	});
//
// 	Kintsugi::execute_with(|| {
// 		assert_eq!(
// 			System::events()
// 				.iter()
// 				.find(|r| matches!(r.event, Event::PolkadotXcm(pallet_xcm::Event::AssetsTrapped(_, _, _)))),
// 			None
// 		);
//
// 		assert_eq!(
// 			ksm_asset_amount + KSM.one(),
// 			Tokens::free_balance(Token(KSM), &KintsugiTreasuryAccount::get())
// 		);
// 		assert_eq!(
// 			kar_asset_amount,
// 			Tokens::free_balance(Token(KINT), &KintsugiTreasuryAccount::get()) - kint_treasury_amount
// 		);
// 	});
// }
//
// #[test]
// fn sibling_trap_assets_works() {
// 	TestNet::reset();
//
// 	let mut kint_treasury_amount = 0;
// 	let (bnc_asset_amount, kar_asset_amount) = (cent(BNC) / 10, cent(KAR));
//
// 	fn sibling_account() -> AccountId {
// 		use sp_runtime::traits::AccountIdConversion;
// 		polkadot_parachain::primitives::Sibling::from(2001).into_account()
// 	}
//
// 	Kintsugi::execute_with(|| {
// 		assert_ok!(Tokens::deposit(BNC, &sibling_account(), BNC.one()));
// 		let _ = pallet_balances::Pallet::<Runtime>::deposit_creating(&sibling_account(), KINT.one());
// 		kint_treasury_amount = Tokens::free_balance(KAR, &KintsugiTreasuryAccount::get());
// 	});
//
// 	Sibling::execute_with(|| {
// 		let assets: MultiAsset = (
// 			(Parent, X2(Parachain(2092), GeneralKey(KAR.encode()))),
// 			kar_asset_amount,
// 		)
// 			.into();
// 		let xcm = vec![
// 			WithdrawAsset(assets.clone().into()),
// 			BuyExecution {
// 				fees: assets,
// 				weight_limit: Unlimited,
// 			},
// 			WithdrawAsset(
// 				(
// 					(
// 						Parent,
// 						X2(Parachain(2001), GeneralKey(parachains::bifrost::BNC_KEY.to_vec())),
// 					),
// 					bnc_asset_amount,
// 				)
// 					.into(),
// 			),
// 		];
// 		assert_ok!(pallet_xcm::Pallet::<Runtime>::send_xcm(
// 			Here,
// 			(Parent, Parachain(2092)),
// 			Xcm(xcm),
// 		));
// 	});
//
// 	Kintsugi::execute_with(|| {
// 		assert_eq!(
// 			System::events()
// 				.iter()
// 				.find(|r| matches!(r.event, Event::PolkadotXcm(pallet_xcm::Event::AssetsTrapped(_, _, _)))),
// 			None
// 		);
// 		assert_eq!(
// 			Tokens::free_balance(KAR, &KintsugiTreasuryAccount::get()) - kint_treasury_amount,
// 			kar_asset_amount
// 		);
// 		assert_eq!(
// 			Tokens::free_balance(BNC, &KintsugiTreasuryAccount::get()),
// 			bnc_asset_amount
// 		);
// 	});
// }
