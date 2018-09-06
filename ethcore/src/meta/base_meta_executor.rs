//TODO: <IOLITE> copyright
use std::u64;

use vm::Error as VmError;
use types::metalogs::MetaLogs;

pub type Bytes = Vec<u8>;


pub struct BaseMetaExecutor {
    pub metadata: Bytes,
}

pub trait MetaExecute<'a> {
    fn execute(&'a mut self) -> Result<MetaLogs, String>;
}

impl BaseMetaExecutor {
    // Computes the 'intrinsic gas' for a message with a given metadata.
    //TODO: <IOLITE> need to rework algorithm
    pub fn intrinsic_gas(&self) -> Result<u64, String> {
        if self.metadata.len() == 0 {
            return Err("[iolite] Error! Metadata is empty.".to_string());
        }

        //TODO: <IOLITE> don't use hardcoded values as
        // tx_data_non_zero_gas and tx_data_zero_gas is in ethcore/vm/src/schedule.rs
        let tx_data_zero_gas = 4u64;
        let tx_data_non_zero_gas = 68u64;

        // Set the starting gas for the raw tx
        let mut gas = 0u64;
        // Bump the required gas by the amount of the transactional data
        // Zero and non-zero bytes are priced differently
        let num_non_zero_bytes : u64 = self.metadata
            .iter()
            .filter(|&&byte| byte != 0u8)
            .fold(0, |sum, &val| sum + 1);


        gas += num_non_zero_bytes * tx_data_non_zero_gas;
        // Make sure we don't exceed u64 for all data combinations
        if (u64::MAX - gas) / tx_data_non_zero_gas < num_non_zero_bytes {
            return Err(VmError::OutOfGas.to_string());
        }

        let num_zero_bytes = (self.metadata.len() as u64) - num_non_zero_bytes;
        gas += num_zero_bytes * tx_data_zero_gas;
        if (u64::MAX - gas) / tx_data_zero_gas < num_zero_bytes {
            return Err(VmError::OutOfGas.to_string());
        }

        return Ok(gas);
    }
}
