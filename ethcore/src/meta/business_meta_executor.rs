//TODO: <IOLITE> copyright
use std::ops::Deref;
use rlp::{self};
use executive::{Executive, TransactOptions};
use transaction::{SignedTransaction, Transaction, UnverifiedTransaction};
use ethereum_types::{U256, Address};
use state::{Backend as StateBackend};

use types::metalogs::MetaLogs;
use meta::base_meta_executor::{BaseMetaExecutor, MetaExecute, Bytes};

pub struct BusinessMetaExecutor<'a, T: 'a + StateBackend> {
    executor: BaseMetaExecutor,

    transaction: &'a SignedTransaction,
    from: Address,
    read_evm: &'a mut Executive<'a, T>,
}

impl<'a, T: 'a + StateBackend> Deref for BusinessMetaExecutor<'a, T> {
    type Target = BaseMetaExecutor;

    fn deref(&self) -> &Self::Target {
        &self.executor
    }
}

impl<'a, T: 'a + StateBackend> BusinessMetaExecutor<'a, T> {
    pub fn new(metadata: Bytes, transaction: &'a SignedTransaction, from: Address, read_evm: &'a mut Executive<'a, T>)
            -> Self {
        BusinessMetaExecutor {
            executor: BaseMetaExecutor { metadata: metadata },
            transaction: transaction,
            from: from,
            read_evm: read_evm,
        }
    }
}

impl<'a, T: 'a + StateBackend> MetaExecute for BusinessMetaExecutor<'a, T> {
    fn execute(&self) -> Result<MetaLogs, Err> {
        if self.metadata.len() == 0 {
            return Err("Error! Metadata is empty.");
        }

        //TODO: <IOLITE> implement BusinessMetadata
        //let business_metadata: BusinessMetadata = rlp::decode(&self.metadata)?;
        //info!("[iolite] Business metadata: {:#?}", business_metadata);

        let tx = SignedTransaction {
            transaction: Transaction {
                data: self.transaction.metadata.cloned(),
                ..self.transaction
            },
            ..self.transaction
        };
        let transact_options = TransactOptions::with_tracing_and_vm_tracing();
        let result = self.read_evm.transact_virtual(&tx, transact_options)?;

        info!("[iolite] Executed metadata: {:#?}", result.output);
        if result.output.len() != 64 {
            return Err("The business call result does not match the format (address, uint256)");
        }

        let metalogs = MetaLogs::new();
        //TODO: <IOLITE> should we convert address simillar to geth? `common.BytesToAddress(&result.output[:32])`
        metalogs.push(Address::from(&result.output[..32]), U256::from(&result.output[32..]));

        for data in metalogs.logs() {
            info!("[iolite] Decoded Metalogs. To: {}, Value: {}", data.recipient, data.amount);
        }

        Ok(metalogs)
    }
}
