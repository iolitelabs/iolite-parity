use std::ops::Deref;
use types::metalogs::MetaLogs;
use ethereum_types::{U256, Address};
use executive::{Executive};
use state::{Backend as StateBackend, CleanupMode};

use meta::base_meta_payer::{BaseMetaPayer, MetaPay, MetaPayable, PaymentOptions};
use meta::meta_util::{MetaUtilError};

pub struct SimpleMetaPayer<'a, T: 'a + StateBackend> {
    payer: BaseMetaPayer,

    evm: &'a mut Executive<'a, T>,
}

impl<'a, T: 'a + StateBackend> SimpleMetaPayer<'a, T> {
    pub fn new(from: Address, meta_logs: MetaLogs, meta_limit: U256, evm: &'a mut Executive<'a, T>) -> Self {
        SimpleMetaPayer {
            payer: BaseMetaPayer::new(from, meta_logs, meta_limit),
            evm: evm,
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
    fn pay(&mut self, _gas: u64) -> Result<(U256, u64), String> {
        let sum = match self.can_pay() {
            PaymentOptions::CanPay(amount) => amount,
            _ => return Err(MetaUtilError::InsufficientFunds.to_string()),
        };

        let meta_logs = self.meta_logs.clone();
        let vm_state = self.evm._get_mut_state();
        for log in meta_logs.logs() {
            vm_state.add_balance(&log.recipient, &log.amount, CleanupMode::NoEmpty).map_err(|err| err.to_string())?;
            vm_state.sub_balance(&self.payer.from, &log.amount, &mut CleanupMode::NoEmpty).map_err(|err| err.to_string())?;
        }

        Ok((sum, 0u64))
    }
}
