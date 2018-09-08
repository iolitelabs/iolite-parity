use std::ops::Deref;
use transaction::{SignedTransaction, Action};
use types::metalogs::{MetaLogs, MetaLog};
use ethereum_types::{U256, Address};
use executive::{Executive, TransactOptions};
use state::{Backend as StateBackend};

use meta::base_meta_payer::{BaseMetaPayer, MetaPay, MetaPayable, PaymentOptions};
use meta::meta_util::{MetaUtilError};

pub struct BusinessMetaPayer<'a, T: 'a + StateBackend> {
    payer: BaseMetaPayer,

    transaction: &'a SignedTransaction,
    evm: &'a mut Executive<'a, T>,
}

impl<'a, T: 'a + StateBackend> BusinessMetaPayer<'a, T> {
    pub fn new(from: Address, meta_logs: MetaLogs, meta_limit: U256, transaction: &'a SignedTransaction, executive: &'a mut Executive<'a, T>) -> Self {
        BusinessMetaPayer {
            payer: BaseMetaPayer::new(from, meta_logs, meta_limit),
            transaction: transaction,
            evm: executive,
        }
    }
}

impl<'a, T: 'a + StateBackend> Deref for BusinessMetaPayer<'a, T> {
    type Target = BaseMetaPayer;

    fn deref(&self) -> &Self::Target {
        &self.payer
    }
}

impl<'a, T: 'a + StateBackend> MetaPay<'a> for BusinessMetaPayer<'a, T> {
    fn pay(&'a mut self, gas: u64) -> Result<(U256, u64), String> {
        if self.payer.meta_logs.logs().len() != 1 {
            return Err("Only one recipient is allowed for business call".to_string());
        }

        let sum = match self.payer.can_pay() {
            PaymentOptions::CanPay(amount) => amount,
            _ => return Err(MetaUtilError::InsufficientFunds.to_string()),
        };

        let gas_left = try_pay(self.payer.from, &self.payer.meta_logs.logs()[0], self.transaction, self.evm, gas)?;

        Ok((sum, gas_left))
    }
}

fn try_pay<'a, T: 'a + StateBackend>(from: Address, log: &MetaLog, transaction: &'a SignedTransaction, evm: &'a mut Executive<'a, T>, gas: u64) -> Result<u64, String> {
    let mut gas_left = gas;
    let transact_options = TransactOptions::with_tracing_and_vm_tracing();
    let mut tx = transaction.clone();
    tx._set_sender(from);
    tx._as_mut_unverified_tx()._as_mut_unsigned().value = log.amount;
    tx._as_mut_unverified_tx()._as_mut_unsigned().gas = U256::from(gas_left);
    tx._as_mut_unverified_tx()._as_mut_unsigned().data = vec![];
    tx._as_mut_unverified_tx()._as_mut_unsigned().action = Action::Call(log.recipient);
    let result = match evm.transact_virtual(&tx, transact_options) {
        Ok(executed_result) => executed_result,
        Err(e) => return Err(e.to_string()),
    };
    //TODO: <IOLITE> is gas_left == refunded ?
    gas_left = result.refunded.as_u64(); // Will panic if number is larger then 2^64
    info!("[iolite] TryPay gas={}; gas_left={}, gas_used={}", gas, gas_left, gas-gas_left);

    Ok(gas_left)
}
