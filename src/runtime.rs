mod extension;
mod pallet_pink;

use crate::types::{AccountId, Balance, BlockNumber, Hash, Hashing, Nonce};
use drink::runtime::{AccountIdFor, Runtime, RuntimeMetadataPrefixed};
use frame_support::sp_runtime::{self, BuildStorage as _};
use frame_support::{
    parameter_types,
    traits::{ConstBool, ConstU32},
    weights::{constants::WEIGHT_REF_TIME_PER_SECOND, Weight},
};
use pallet_contracts::Determinism;
use pallet_contracts::{
    migration::{v11, v12, v13, v14, v15},
    weights::SubstrateWeight,
    Config, Frame, Schedule,
};
use sp_runtime::{
    traits::{Dispatchable, IdentityLookup},
    Perbill,
};

pub use pink_extension::{EcdhPublicKey, HookPoint, Message, OspMessage, PinkEvent};

type Block = sp_runtime::generic::Block<
    sp_runtime::generic::Header<BlockNumber, Hashing>,
    frame_system::mocking::MockUncheckedExtrinsic<PinkRuntime>,
>;

frame_support::construct_runtime! {
    pub struct PinkRuntime {
        System: frame_system,
        Timestamp: pallet_timestamp,
        Balances: pallet_balances,
        Contracts: pallet_contracts,
        Pink: pallet_pink,
    }
}

const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

parameter_types! {
    pub const BlockHashCount: u32 = 250;
    pub RuntimeBlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::with_sensible_defaults(
            Weight::from_parts(2u64 * WEIGHT_REF_TIME_PER_SECOND, u64::MAX),
            NORMAL_DISPATCH_RATIO,
        );
    pub const ExistentialDeposit: Balance = 1;
    pub const MaxLocks: u32 = 50;
    pub const MaxReserves: u32 = 50;
    pub const MaxHolds: u32 = 10;
}

impl pallet_pink::Config for PinkRuntime {
    type Currency = Balances;
}

impl pallet_balances::Config for PinkRuntime {
    type Balance = Balance;
    type DustRemoval = ();
    type RuntimeEvent = RuntimeEvent;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = frame_system::Pallet<PinkRuntime>;
    type WeightInfo = pallet_balances::weights::SubstrateWeight<PinkRuntime>;
    type MaxLocks = MaxLocks;
    type MaxReserves = MaxReserves;
    type ReserveIdentifier = [u8; 8];
    type FreezeIdentifier = ();
    type MaxHolds = MaxHolds;
    type MaxFreezes = ();
    type RuntimeHoldReason = RuntimeHoldReason;
}

impl frame_system::Config for PinkRuntime {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = RuntimeBlockWeights;
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type Nonce = Nonce;
    type Hash = Hash;
    type RuntimeCall = RuntimeCall;
    type Hashing = Hashing;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Block = Block;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
    pub const MinimumPeriod: u64 = 1;
}

impl pallet_timestamp::Config for PinkRuntime {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

const MAX_CODE_LEN: u32 = 2 * 1024 * 1024;

parameter_types! {
    pub DepositPerStorageByte: Balance = Pink::deposit_per_byte();
    pub DepositPerStorageItem: Balance = Pink::deposit_per_item();
    pub const DefaultDepositLimit: Balance = Balance::max_value();
    pub const MaxCodeLen: u32 = MAX_CODE_LEN;
    pub const MaxStorageKeyLen: u32 = 128;
    pub const MaxDebugBufferLen: u32 = 128 * 1024;
    pub DefaultSchedule: Schedule<PinkRuntime> = {
        let mut schedule = Schedule::<PinkRuntime>::default();
        const MB: u32 = 16;  // 64KiB * 16
        // Each concurrent query would create a VM instance to serve it. We couldn't
        // allocate too much here.
        schedule.limits.memory_pages = 4 * MB;
        schedule.instruction_weights.base = 8000;
        schedule.limits.runtime_memory = 2048 * 1024 * 1024; // For unittests
        schedule.limits.payload_len = 1024 * 1024; // Max size for storage value
        schedule
    };
    pub CodeHashLockupDepositPercent: Perbill = Perbill::from_percent(30);
}
use drink::runtime::minimal::SandboxRandomness;

impl Config for PinkRuntime {
    type Time = Timestamp;
    type Randomness = SandboxRandomness;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type CallFilter = frame_support::traits::Nothing;
    type CallStack = [Frame<Self>; 5];
    type WeightPrice = Pink;
    type WeightInfo = SubstrateWeight<Self>;
    type ChainExtension = extension::PinkExtension;
    type Schedule = DefaultSchedule;
    type DepositPerByte = DepositPerStorageByte;
    type DepositPerItem = DepositPerStorageItem;
    type DefaultDepositLimit = DefaultDepositLimit;
    type AddressGenerator = Pink;
    type MaxCodeLen = MaxCodeLen;
    type MaxStorageKeyLen = MaxStorageKeyLen;
    type UnsafeUnstableInterface = ConstBool<false>;
    type MaxDebugBufferLen = MaxDebugBufferLen;
    type Migrations = (
        v11::Migration<Self>,
        v12::Migration<Self, Balances>,
        v13::Migration<Self>,
        v14::Migration<Self, Balances>,
        v15::Migration<Self>,
    );
    type CodeHashLockupDepositPercent = CodeHashLockupDepositPercent;
    type MaxDelegateDependencies = ConstU32<32>;
    type RuntimeHoldReason = RuntimeHoldReason;
    type Debug = ();
    type Environment = ();
}

/// Default initial balance for the default account.
pub const INITIAL_BALANCE: u128 = 1_000_000_000_000_000;

impl Runtime for PinkRuntime {
    fn initialize_storage(storage: &mut sp_runtime::Storage) -> Result<(), String> {
        pallet_balances::GenesisConfig::<Self> {
            balances: vec![(Self::default_actor(), INITIAL_BALANCE)],
        }
        .assimilate_storage(storage)
    }

    fn default_actor() -> AccountIdFor<Self> {
        AccountId::new([1u8; 32])
    }

    fn get_metadata() -> RuntimeMetadataPrefixed {
        Self::metadata()
    }

    fn convert_account_to_origin(
        account: AccountIdFor<Self>,
    ) -> <<Self as Config>::RuntimeCall as Dispatchable>::RuntimeOrigin {
        Some(account).into()
    }
}

impl PinkRuntime {
    pub fn setup_cluster() -> Result<(), String> {
        type PalletPink = Pink;
        PalletPink::set_cluster_id(Hash::zero());
        PalletPink::set_gas_price(0);
        PalletPink::set_deposit_per_item(0);
        PalletPink::set_deposit_per_byte(0);
        PalletPink::set_treasury_account(&[0u8; 32].into());

        let system_code = include_bytes!("../artifacts/system.wasm").to_vec();

        let owner = PinkRuntime::default_actor();
        let code_hash = upload_code(owner.clone(), system_code, true)
            .map_err(|err| format!("FailedToUploadSystemCode: {err:?}"))?;

        let selector = vec![0xed, 0x4b, 0x9d, 0x1b]; // The default() constructor
        let result = Contracts::bare_instantiate(
            owner.clone(),
            0,
            Weight::MAX,
            None,
            pallet_contracts_primitives::Code::Existing(code_hash),
            selector,
            vec![],
            pallet_contracts::DebugInfo::UnsafeDebug,
            pallet_contracts::CollectEvents::Skip,
        );
        log::info!("Contract instantiation result: {:?}", &result.result);
        let address = result
            .result
            .expect("Failed to instantiate system contract")
            .account_id;
        PalletPink::set_system_contract(&address);
        Ok(())
    }

    pub fn execute_in_mode<T>(mode: crate::ExecMode, f: impl FnOnce() -> T) -> T {
        extension::exec_in_mode(mode, f)
    }
}

fn upload_code(account: AccountId, code: Vec<u8>, deterministic: bool) -> Result<Hash, String> {
    Contracts::bare_upload_code(
        account,
        code,
        None,
        if deterministic {
            Determinism::Enforced
        } else {
            Determinism::Relaxed
        },
    )
    .map(|v| v.code_hash)
    .map_err(|err| format!("{err:?}"))
}
