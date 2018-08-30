use std::ops::Deref;
use types::metalogs::MetaLogs;
use ethereum_types::{U256, Address};
use state::State;

use meta::base_meta_payer::{BaseMetaPayer, MetaPay, PaymentOptions};
use meta::meta_util::{MetaUtilError};

pub struct SimpleMetaPayer {
    payer: BaseMetaPayer,

    vm_state: &'a mut State,
}

impl SimpleMetaPayer {
    pub fn new(from: Address, meta_logs: MetaLogs, meta_limit: U256, vm_state: &'a mut State) -> Self {
        SimpleMetaPayer {
            payer: BaseMetaPayer::new(from, meta_logs, meta_limit),
            vm_state: vm_state,
        }
    }
}

impl Deref for SimpleMetaPayer {
    type Target = BaseMetaPayer;

    fn deref(&self) -> &Self::Target {
        self.payer
    }
}

impl MetaPay for SimpleMetaPayer {
    // return (sum, gas_left)
    fn pay(&self, gas: u64) -> Result<(U256, u64), ()> {
        let sum = match self.can_pay() {
            PaymentOptions::CanPay(amount) => amount,
            _ => return MetaUtilError::InsufficientFunds(),
        };

        for log in self.meta_logs.logs() {
            self.state.add_balance(log.recipient, log.amount);
            self.state.sub_balance(self.payer.from, log.amount);
        }

        Ok(sum, 0u64)
    }
}
