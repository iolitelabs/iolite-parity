//TODO: <IOLITE> copyright
use types::metalogs::MetaLogs;
use ethereum_types::{U256, Address};
use vm::Error as VmError;


pub struct BaseMetaPayer {
    from: Address,
    meta_logs: MetaLogs,
    meta_limit: U256,
}

pub trait MetaPayable {
    fn can_pay(&self) -> PaymentOptions;
}

pub trait MetaPay {
    fn pay(&self, gas: u64) -> Result<U256, ()>;
}

pub enum PaymentOptions {
    CanPay(U256),
    CantPay,
}

impl BaseMetaPayer {
    pub fn new(from: Address, meta_logs: MetaLogs, meta_limit: U256) -> Self {
        BaseMetaPayer {
            from: from,
            meta_logs: meta_logs,
            meta_limit: meta_limit,
        }
    }

    pub fn intrinsic_gas(&self) -> Result<u64, ()> {
        let num_logs = self.meta_logs.logs().len() as u64;
        if num_logs == 0 {
            return Err("Metalogs are empty.");
        }

        //TODO: <IOLITE> don't use hardcoded values as
        // tx_gas is in ethcore/vm/src/schedule.rs
        let tx_gas = 21000u64;
        let gas = num_logs * tx_gas;
        // Check overflow
        if gas / num_logs != tx_gas {
            return Err(VmError::OutOfGas);
        }

        Ok(gas)
    }
}

impl MetaPayable for BaseMetaPayer {
    fn can_pay(&self) -> PaymentOptions {
        let mut sum = U256::zero();

        if self.meta_logs.logs().len() == 0 {
            return PaymentOptions::CanPay(sum);
        }

        for log in self.meta_logs.logs() {
            sum.add(log.amount);
        }

        info!("[iolite] CanPay sum={}, metaLimit={}", sum, self.meta_limit);
        if sum.cmp(self.meta_limit) > 0 {
            return PaymentOptions::CantPay;
        }

        PaymentOptions::CanPay(sum)
    }
}

