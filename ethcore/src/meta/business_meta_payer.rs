use std::ops::Deref;
use transaction::{SignedTransaction};
use types::metalogs::MetaLogs;
use ethereum_types::{U256, Address};
use executive::{Executive, TransactOptions};

use meta::base_meta_payer::{BaseMetaPayer, MetaPay, PaymentOptions};
use meta::meta_util::{MetaUtilError};

pub struct BusinessMetaPayer {
    payer: BaseMetaPayer,

    transaction: &'a SignedTransaction,
    evm: &'a mut Executive,
}

impl BusinessMetaPayer {
    pub fn new(from: Address, meta_logs: MetaLogs, meta_limit: U256, transaction: &'a SignedTransaction, executive: &'a mut Executive) -> Self {
        BusinessMetaPayer {
            payer: BaseMetaPayer::new(from, meta_logs, meta_limit),
            transaction: transaction,
            evm: executive,
        }
    }
}

impl Deref for BusinessMetaPayer {
    type Target = BaseMetaPayer;

    fn deref(&self) -> &Self::Target {
        self.payer
    }
}

impl MetaPay for BusinessMetaPayer {
    fn pay(&self, gas: u64) -> Result<(U256, u64), ()> {
        if self.payer.meta_logs.logs().len() > 1 {
            return Err("Only one recipient is allowed for business call");
        }

        let sum = match self.payer.can_pay() {
            PaymentOptions::CanPay(amount) => amount,
            _ => return Err(MetaUtilError::InsufficientFunds),
        };

        let gas_left = try_pay(self.payer.from, self.payer.meta_logs, self.transaction, self.evm, gas)?;

        Ok((sum, gas_left))
    }
}

fn try_pay(from: Address, meta_logs: &MetaLogs, transaction: &'a SignedTransaction, evm: &'a mut Executive, gas: u64) -> Result<u64, ()> {
    let mut gas_left = gas;
    for log in meta_logs {
        let transact_options = TransactOptions::with_tracing_and_vm_tracing();
        let tx = SignedTransaction {
            from: from,
            to: log.recipient,
            value: log.amount,
            gas: gas_left,
            data: vec![],
            ..*transaction };
        let result = evm.transact_virtual(&tx, transact_options)?;
        gas_left = result.gas_left;
        info!("[iolite] TryPay gas={}; gas_left={}, gas_used={}", gas, gas_left, gas-gas_left);
    }

    Ok(gas_left)
}
