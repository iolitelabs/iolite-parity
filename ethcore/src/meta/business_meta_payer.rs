use std::ops::Deref;
use transaction::{Transaction, SignedTransaction, UnverifiedTransaction};
use types::metalogs::MetaLogs;
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
    pub fn new(from: Address, meta_logs: MetaLogs, meta_limit: U256, transaction: &'a SignedTransaction, executive: &mut Executive<'a, T>) -> Self {
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

impl<'a, T: 'a + StateBackend> MetaPay for BusinessMetaPayer<'a, T> {
    fn pay(&self, gas: u64) -> Result<(U256, u64), Err> {
        if self.payer.meta_logs.logs().len() > 1 {
            return Err("Only one recipient is allowed for business call");
        }

        let sum = match self.payer.can_pay() {
            PaymentOptions::CanPay(amount) => amount,
            _ => return Err(MetaUtilError::InsufficientFunds),
        };

        let gas_left = try_pay(self.payer.from, &self.payer.meta_logs, self.transaction, self.evm, gas)?;

        Ok((sum, gas_left))
    }
}

fn try_pay<'a, T: 'a + StateBackend>(from: Address, meta_logs: &MetaLogs, transaction: &'a SignedTransaction, evm: &'a mut Executive<T>, gas: u64) -> Result<u64, ()> {
    let mut gas_left = gas;
    for log in meta_logs {
        let transact_options = TransactOptions::with_tracing_and_vm_tracing();
        let tx = SignedTransaction {
            transaction: UnverifiedTransaction {
                unsinged: Transaction {
                    from: from,
                    value: log.amount,
                    gas: U256::from(gas_left),
                    data: vec![],
                    ..Default::default()
                }
                to: log.recipient,
            },
            ..*transaction };
        let result = evm.transact_virtual(&tx, transact_options)?;
        gas_left = result.gas_left;
        info!("[iolite] TryPay gas={}; gas_left={}, gas_used={}", gas, gas_left, gas-gas_left);
    }

    Ok(gas_left)
}
