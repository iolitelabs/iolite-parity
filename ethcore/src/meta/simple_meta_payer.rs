use std::ops::Deref;
use types::metalogs::MetaLogs;
use ethereum_types::{U256, Address};
use state::{State, Backend as StateBackend};

use meta::base_meta_payer::{BaseMetaPayer, MetaPay, MetaPayable, PaymentOptions};
use meta::meta_util::{MetaUtilError};

pub struct SimpleMetaPayer<'a, T: 'a + StateBackend> {
    payer: BaseMetaPayer,

    vm_state: &'a mut State<T>,
}

impl<'a, T: 'a + StateBackend> SimpleMetaPayer<'a, T> {
    pub fn new(from: Address, meta_logs: MetaLogs, meta_limit: U256, vm_state: &'a mut State<T>) -> Self {
        SimpleMetaPayer {
            payer: BaseMetaPayer::new(from, meta_logs, meta_limit),
            vm_state: vm_state,
        }
    }
}

impl<'a, T: 'a + StateBackend> Deref for SimpleMetaPayer<'a, T> {
    type Target = BaseMetaPayer;

    fn deref(&self) -> &Self::Target {
        &self.payer
    }
}

impl<'a, T: 'a + StateBackend> MetaPay for SimpleMetaPayer<'a, T> {
    // return (sum, gas_left)
    fn pay(&self, gas: u64) -> Result<(U256, u64), Err> {
        let sum = match self.can_pay() {
            PaymentOptions::CanPay(amount) => amount,
            _ => return MetaUtilError::InsufficientFunds,
        };

        for log in self.meta_logs.logs() {
            self.vm_state.add_balance(log.recipient, log.amount);
            self.vm_state.sub_balance(self.payer.from, log.amount);
        }

        Ok((sum, 0u64))
    }
}
