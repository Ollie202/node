#![no_std]
#![feature(alloc_error_handler)]

use miden::{AccountId, Felt, StorageValue, active_note, assert_eq, component, felt};

#[component]
struct CounterContract {
    #[storage(description = "authorized note sender")]
    owner: StorageValue<AccountId>,
    #[storage(description = "network monitor counter value")]
    counter: StorageValue<Felt>,
}

#[component]
impl CounterContract {
    pub fn increment(&mut self) {
        let sender = active_note::get_sender();
        let owner = self.owner.get();
        assert_eq(sender.prefix, owner.prefix);
        assert_eq(sender.suffix, owner.suffix);

        let next_value = self.counter.get() + felt!(1);
        self.counter.set(next_value);
    }

    pub fn get_count(&self) -> Felt {
        self.counter.get()
    }
}
