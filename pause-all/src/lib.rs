#![no_std]

use elrond_wasm_modules::ongoing_operation::{CONTINUE_OP, STOP_OP};
use ongoing_pause_operation::{OngoingOperation, MIN_GAS_TO_SAVE_PROGRESS};

elrond_wasm::imports!();

mod pause_proxy {
    elrond_wasm::imports!();

    #[elrond_wasm::proxy]
    pub trait Pausable {
        #[endpoint]
        fn pause(&self);

        #[endpoint]
        fn resume(&self);
    }
}

pub mod ongoing_pause_operation;
/// https://github.com/multiversx/mx-sdk-rs/blob/master/contracts/modules/src/ongoing_operation.rs 
#[elrond_wasm::contract]
pub trait PauseAll:
    ongoing_pause_operation::OngoingPauseOperationModule
    + elrond_wasm_modules::ongoing_operation::OngoingOperationModule
{
    #[init]
    fn init(&self) {}

    /// Добавить адреса контрактов в сторэдж мэппер pausable_contracts
    #[only_owner]
    #[endpoint(addPausableContracts)]
    fn add_pausable_contracts(&self, pausable_sc_addr: MultiValueEncoded<ManagedAddress>) {
        let mut whitelist = self.pausable_contracts();
        for addr in pausable_sc_addr {
            let _ = whitelist.insert(addr);
        }
    }

    /// Удалить адреса из сторэджа
    #[only_owner]
    #[endpoint(removePausableContracts)]
    fn remove_pausable_contracts(&self, pausable_sc_addr: MultiValueEncoded<ManagedAddress>) {
        let mut whitelist = self.pausable_contracts();
        for addr in pausable_sc_addr {
            let _ = whitelist.swap_remove(&addr);
        }
    }

    /// Поставить на паузу контракты из аргумента функции, если они уже есть сторэдже. 
    /// Если некоторых адресов нет в сторэдже - эти контракты будут проигнорированы 
    #[only_owner]
    #[endpoint(pauseSelected)]
    fn pause_selected(&self, pausable_sc_addr: MultiValueEncoded<ManagedAddress>) {
        let whitelist = self.pausable_contracts();
        for addr in pausable_sc_addr {
            if whitelist.contains(&addr) {
                self.call_pause(addr); /// ставит на паузу через прокси
            }
        }
    }

    /// Паузим контракты из сторэджа. Если абсолютно все запаузили, возвращает "завершено"
    /// Если нет - возвращает "прерванно", и для завершения потребуется больше вызовов
    #[only_owner]
    #[endpoint(pauseAll)]
    fn pause_all(&self) -> OperationCompletionStatus {
        let mut current_index = self.load_pause_all_operation();
        let whitelist = self.pausable_contracts();
        let whitelist_len = whitelist.len();

        /// проверка, есть ли газ в количестве MIN_GAS_TO_SAVE_PROGRESS
        let run_result = self.run_while_it_has_gas(MIN_GAS_TO_SAVE_PROGRESS, || {
            if current_index > whitelist_len {
                return STOP_OP;
            }
            /// идем по индексам и ставим на паузу контракты из сторэджа
            let sc_addr = whitelist.get_by_index(current_index);
            self.call_pause(sc_addr);
            current_index += 1;

            CONTINUE_OP
        });
        /// выводим количество контрактов поставленных на паузу, если абсолютно все контракты запаузили
        if run_result == OperationCompletionStatus::InterruptedBeforeOutOfGas {
            self.save_progress(&OngoingOperation::PauseAll {
                addr_index: current_index,
            });
        }

        run_result
    }

    fn call_pause(&self, sc_addr: ManagedAddress) {
        let _: IgnoreValue = self.pause_proxy(sc_addr).pause().execute_on_dest_context();
    }

    /// 3 функции ниже абсолютно аналогичны верхним трем функциям, только pause заменили на resume
    #[only_owner]
    #[endpoint(resumeSelected)]
    fn resume_selected(&self, pausable_sc_addr: MultiValueEncoded<ManagedAddress>) {
        let whitelist = self.pausable_contracts();
        for addr in pausable_sc_addr {
            if whitelist.contains(&addr) {
                self.call_resume(addr);
            }
        }
    }

    /// Will attempt to unpause all contracts from the whitelist.
    /// Returns "completed" if all were unpaused.
    /// Otherwise, it will save progress and return "interrupted",
    /// and will require more calls to complete
    #[only_owner]
    #[endpoint(resumeAll)]
    fn resume_all(&self) -> OperationCompletionStatus {
        let mut current_index = self.load_resume_all_operation();
        let whitelist = self.pausable_contracts();
        let whitelist_len = whitelist.len();

        let run_result = self.run_while_it_has_gas(MIN_GAS_TO_SAVE_PROGRESS, || {
            if current_index > whitelist_len {
                return STOP_OP;
            }

            let sc_addr = whitelist.get_by_index(current_index);
            self.call_resume(sc_addr);
            current_index += 1;

            CONTINUE_OP
        });
        if run_result == OperationCompletionStatus::InterruptedBeforeOutOfGas {
            self.save_progress(&OngoingOperation::ResumeAll {
                addr_index: current_index,
            });
        }

        run_result
    }

    fn call_resume(&self, sc_addr: ManagedAddress) {
        let _: IgnoreValue = self.pause_proxy(sc_addr).resume().execute_on_dest_context();
    }

    #[proxy]
    fn pause_proxy(&self, addr: ManagedAddress) -> pause_proxy::Proxy<Self::Api>;

    #[view(getPausableContracts)]
    #[storage_mapper("pausableContracts")]
    fn pausable_contracts(&self) -> UnorderedSetMapper<ManagedAddress>;
}
