use crate as refund;
use crate::Config;
use frame_support::{assert_ok, parameter_types, traits::StorageMapShim};
use mocktopus::mocking::clear_mocks;
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    FixedI128, FixedPointNumber, FixedU128,
};

pub const VAULT: AccountId = 1;
pub const USER: AccountId = 2;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Module, Call, Storage, Config, Event<T>},
        Timestamp: pallet_timestamp::{Module, Call, Storage, Inherent},

        // Tokens & Balances
        DOT: pallet_balances::<Instance1>::{Module, Call, Storage, Config<T>, Event<T>},
        PolkaBTC: pallet_balances::<Instance2>::{Module, Call, Storage, Config<T>, Event<T>},

        Collateral: collateral::{Module, Call, Storage, Event<T>},
        Treasury: treasury::{Module, Call, Storage, Event<T>},

        // Operational
        BTCRelay: btc_relay::{Module, Call, Config<T>, Storage, Event},
        Security: security::{Module, Call, Storage, Event},
        VaultRegistry: vault_registry::{Module, Call, Config<T>, Storage, Event<T>},
        ExchangeRateOracle: exchange_rate_oracle::{Module, Call, Config<T>, Storage, Event<T>},
        Fee: fee::{Module, Call, Config<T>, Storage, Event<T>},
        Sla: sla::{Module, Call, Config<T>, Storage, Event<T>},
        Refund: refund::{Module, Call, Config<T>, Storage, Event<T>},
    }
);

pub type AccountId = u64;
pub type Balance = u64;
pub type BlockNumber = u64;

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

impl frame_system::Config for Test {
    type BaseCallFilter = ();
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = BlockNumber;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = TestEvent;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
    pub const MaxLocks: u32 = 50;
}

/// DOT
impl pallet_balances::Config<pallet_balances::Instance1> for Test {
    type MaxLocks = MaxLocks;
    type Balance = Balance;
    type Event = TestEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = StorageMapShim<
        pallet_balances::Account<Test, pallet_balances::Instance1>,
        frame_system::Provider<Test>,
        AccountId,
        pallet_balances::AccountData<Balance>,
    >;
    type WeightInfo = ();
}

/// PolkaBTC
impl pallet_balances::Config<pallet_balances::Instance2> for Test {
    type MaxLocks = MaxLocks;
    type Balance = Balance;
    type Event = TestEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = StorageMapShim<
        pallet_balances::Account<Test, pallet_balances::Instance2>,
        frame_system::Provider<Test>,
        AccountId,
        pallet_balances::AccountData<Balance>,
    >;
    type WeightInfo = ();
}

impl Config for Test {
    type Event = TestEvent;
    type WeightInfo = ();
}

impl treasury::Config for Test {
    type Event = TestEvent;
    type PolkaBTC = pallet_balances::Module<Test, pallet_balances::Instance2>;
}

impl fee::Config for Test {
    type Event = TestEvent;
    type UnsignedFixedPoint = FixedU128;
    type WeightInfo = ();
}

impl sla::Config for Test {
    type Event = TestEvent;
    type SignedFixedPoint = FixedI128;
}

impl collateral::Config for Test {
    type Event = TestEvent;
    type DOT = pallet_balances::Module<Test, pallet_balances::Instance1>;
}

impl btc_relay::Config for Test {
    type Event = TestEvent;
    type WeightInfo = ();
}

impl security::Config for Test {
    type Event = TestEvent;
}

impl vault_registry::Config for Test {
    type Event = TestEvent;
    type RandomnessSource = pallet_randomness_collective_flip::Module<Test>;
    type UnsignedFixedPoint = FixedU128;
    type WeightInfo = ();
}

impl exchange_rate_oracle::Config for Test {
    type Event = TestEvent;
    type UnsignedFixedPoint = FixedU128;
    type WeightInfo = ();
}

parameter_types! {
    pub const MinimumPeriod: u64 = 5;
}

impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

pub type TestEvent = Event;

pub struct ExtBuilder;

impl ExtBuilder {
    pub fn build() -> sp_io::TestExternalities {
        let storage = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        sp_io::TestExternalities::from(storage)
    }
}

pub fn run_test<T>(test: T) -> ()
where
    T: FnOnce() -> (),
{
    clear_mocks();
    ExtBuilder::build().execute_with(|| {
        assert_ok!(<exchange_rate_oracle::Module<Test>>::_set_exchange_rate(
            FixedU128::one()
        ));
        System::set_block_number(1);
        test();
    });
}
