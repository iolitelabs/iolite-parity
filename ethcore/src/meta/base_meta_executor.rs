use vm::Error as VmError;


pub struct BaseMetaExecutor {
    metadata: Bytes,
}

pub trait MetaExecute {
    fn execute(&self) -> Result<MetaLogs, ()>;
}

impl BaseMetaExecutor {
    // Computes the 'intrinsic gas' for a message with a given metadata.
    //TODO: <IOLITE> need to rework algorithm
    pub fn intrinsic_gas(&self) -> Result<u64, ()> {
        if self.metadata.len() == 0 {
            return Err("[iolite] Error! Metadata is empty.");
        }

        // Set the starting gas for the raw tx
        let mut gas = 0u64;
        // Bump the required gas by the amount of the transactional data
        // Zero and non-zero bytes are priced differently
        let num_non_zero_bytes : u64 = self.metadata
            .iter()
            .filter(|byte| byte != 0u8)
            .fold(0, |sum, &val| sum + 1);

        // Make sure we don't exceed u64 for all data combinations
        //TODO: <IOLITE> don't use hardcoded values as
        // tx_data_non_zero_gas and tx_data_zero_gas is in ethcore/vm/src/schedule.rs
        let tx_data_zero_gas = 4u64;
        let tx_data_non_zero_gas = 68u64;
        // gas is always empty here?
        if (u64::MAX - gas) / tx_data_non_zero_gas < num_non_zero_bytes {
            return Err(VmError::OutOfGas);
        }
        gas += num_non_zero_bytes * tx_data_non_zero_gas;

        let num_zero_bytes = (self.metadata.len() as u64) - num_non_zero_bytes;
        if (u64::MAX - gas) / tx_data_zero_gas < num_zero_bytes {
            return Err(VmError::OutOfGas);
        }
        gas += num_zero_bytes * tx_data_zero_gas;

        return Ok(gas);
    }
}
