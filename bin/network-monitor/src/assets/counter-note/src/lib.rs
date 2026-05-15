#![no_std]
#![feature(alloc_error_handler)]

use miden::{Word, assert_eq, felt, note, note_script};

use crate::bindings::miden::monitor_counter_contract::counter_contract;

#[note]
struct CounterNote;

#[note]
impl CounterNote {
    #[note_script]
    pub fn run(self, _arg: Word) {
        let initial_value = counter_contract::get_count();
        counter_contract::increment();
        let expected_value = initial_value + felt!(1);
        let final_value = counter_contract::get_count();
        assert_eq(final_value, expected_value);
    }
}
