//! Counter program account creation functionality.

use std::path::Path;

use anyhow::Result;
#[cfg(not(compiled_miden_rust_assets))]
use miden_protocol::account::StorageSlot;
#[cfg(not(compiled_miden_rust_assets))]
use miden_protocol::account::component::AccountComponentMetadata;
#[cfg(compiled_miden_rust_assets)]
use miden_protocol::account::component::{InitStorageData, StorageValueName};
use miden_protocol::account::{
    Account,
    AccountBuilder,
    AccountComponent,
    AccountFile,
    AccountId,
    AccountStorageMode,
    AccountType,
    StorageSlotName,
};
#[cfg(not(compiled_miden_rust_assets))]
use miden_protocol::assembly::Library;
use miden_protocol::utils::serde::Deserializable;
use miden_protocol::utils::sync::LazyLock;
#[cfg(compiled_miden_rust_assets)]
use miden_protocol::vm::Package;
use miden_protocol::{Felt, Word};
use miden_standards::testing::account_component::IncrNonceAuthComponent;
use tracing::instrument;

use crate::COMPONENT;

pub static OWNER_SLOT_NAME: LazyLock<StorageSlotName> = LazyLock::new(|| {
    #[cfg(compiled_miden_rust_assets)]
    {
        StorageSlotName::new("miden_monitor_counter_contract::counter_contract::owner")
            .expect("storage slot name should be valid")
    }

    #[cfg(not(compiled_miden_rust_assets))]
    {
        StorageSlotName::new("miden::monitor::counter_contract::owner")
            .expect("storage slot name should be valid")
    }
});

pub static COUNTER_SLOT_NAME: LazyLock<StorageSlotName> = LazyLock::new(|| {
    #[cfg(compiled_miden_rust_assets)]
    {
        StorageSlotName::new("miden_monitor_counter_contract::counter_contract::counter")
            .expect("storage slot name should be valid")
    }

    #[cfg(not(compiled_miden_rust_assets))]
    {
        StorageSlotName::new("miden::monitor::counter_contract::counter")
            .expect("storage slot name should be valid")
    }
});

#[cfg(compiled_miden_rust_assets)]
static COUNTER_CONTRACT_PACKAGE: LazyLock<Package> = LazyLock::new(|| {
    let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/counter_contract.masp"));
    Package::read_from_bytes(bytes).expect("counter contract package should be valid")
});

#[cfg(not(compiled_miden_rust_assets))]
static COUNTER_PROGRAM_LIBRARY: LazyLock<Library> = LazyLock::new(|| {
    let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/counter_program.masl"));
    Library::read_from_bytes(bytes).expect("counter program library should be valid")
});

/// An [`AccountComponent`] implementing the counter contract used by the network monitor.
pub struct CounterComponent {
    pub owner_account_id: AccountId,
}

impl TryFrom<CounterComponent> for AccountComponent {
    type Error = anyhow::Error;

    fn try_from(component: CounterComponent) -> Result<Self> {
        #[cfg(compiled_miden_rust_assets)]
        {
            let owner_account_id_prefix = component.owner_account_id.prefix().as_felt();
            let owner_account_id_suffix = component.owner_account_id.suffix();

            let mut init_storage_data = InitStorageData::default();
            init_storage_data.insert_value(
                StorageValueName::from_slot_name(&*OWNER_SLOT_NAME),
                Word::from([
                    Felt::ZERO,
                    Felt::ZERO,
                    owner_account_id_suffix,
                    owner_account_id_prefix,
                ]),
            )?;
            init_storage_data
                .insert_value(StorageValueName::from_slot_name(&*COUNTER_SLOT_NAME), Felt::ZERO)?;

            AccountComponent::from_package(&COUNTER_CONTRACT_PACKAGE, &init_storage_data)
                .map_err(Into::into)
        }

        #[cfg(not(compiled_miden_rust_assets))]
        {
            let owner_account_id_prefix = component.owner_account_id.prefix().as_felt();
            let owner_account_id_suffix = component.owner_account_id.suffix();

            let owner_id_slot = StorageSlot::with_value(
                OWNER_SLOT_NAME.clone(),
                Word::from([
                    owner_account_id_suffix,
                    owner_account_id_prefix,
                    Felt::ZERO,
                    Felt::ZERO,
                ]),
            );

            let counter_slot = StorageSlot::with_value(COUNTER_SLOT_NAME.clone(), Word::empty());
            let metadata = AccountComponentMetadata::new("counter::program", AccountType::all());

            AccountComponent::new(
                COUNTER_PROGRAM_LIBRARY.clone(),
                vec![counter_slot, owner_id_slot],
                metadata,
            )
            .map_err(Into::into)
        }
    }
}

/// Create a counter program account.
#[instrument(target = COMPONENT, name = "create-counter-account", skip_all, ret(level = "debug"))]
pub fn create_counter_account(owner_account_id: AccountId) -> Result<Account> {
    let counter_component: AccountComponent = CounterComponent { owner_account_id }.try_into()?;
    let incr_nonce_auth: AccountComponent = IncrNonceAuthComponent.into();

    let init_seed: [u8; 32] = rand::random();
    let counter_account = AccountBuilder::new(init_seed)
        .account_type(AccountType::RegularAccountUpdatableCode)
        .storage_mode(AccountStorageMode::Network)
        .with_component(counter_component)
        .with_auth_component(incr_nonce_auth)
        .build()?;

    Ok(counter_account)
}

/// Save counter program account to disk without extra auth material.
pub fn save_counter_account(account: &Account, file_path: &Path) -> Result<()> {
    let account_file = AccountFile::new(account.clone(), vec![]);
    account_file.write(file_path)?;
    Ok(())
}
