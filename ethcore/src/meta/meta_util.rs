use types::MetaLogs;
use meta_payer::MetaPayer;
use simple_meta_executor::SimpleMetaExecutor;
use business_meta_executor::BusinessMetaExecutor;


#[derive(Debug, PartialEq, Clone, Copy, Eq)]
pub enum MetaUtilError {
    // Insufficient funds: (provided: u64, expected: u64)
    InsufficientFunds(u64, u64),
    //TODO: <IOLITE> since we don't have `IntrinsicGas()` method in parity
    // this error could be not relevant or redundant
    // Intrinsic gas (provided: u64, expected: u64)
    IntrinsicGasFailed(u64, u64),
};

impl fmt::Display for MetaUtilError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match *self {
            MetaUtilError::InsufficientFunds(ref provided, ref expected) => {
                format!("insufficient funds for metadata payment or payment are not allowed. Provided: {}; Expected: {}",
                        provided, expected)
            },
            MetaUtilError::IntrinsicGasFailed(ref provided, ref expected) => {
                format!("Metadata intrinsic gas error. Provided: {}; Expected: {}", provided, expected)
            },
        }

        f.write_fmt(format_args!(MetaUtilError({}), msg))
    }
}

fn unpack_simple_metadata(from: U256, metadata: Bytes, meta_limit: U256, read_evm: &'a mut Executive)//state: State)
        // return (payer, meta_logs, payment)
        -> Result<(MetaPayer, MetaLogs, U256), MetaUtilError> {
    println!("[iolite] UnpackSimpleMetadata. Metalimit={}", meta_limit);
    let executor = SimpleMetaExecutor::new(metadata);

    //TODO: <IOLITE> do we really need this?
    let executor_gas = executor.intrinsic_gas()?;

    let meta_logs = executor.execute()?;

    let payer = SimpleMetaPayer::new(from, meta_logs, meta_limit, state);
    //TODO: <IOLITE> do we really need this?
    let payer_gas = payer.intrinsic_gas()?;

    let payment = match payer.can_pay() {
        //TODO: implement enum for payer: e.g. `enum Payer::PaymentOptions { Payment(u64), CantPay, }`
        Payer::Payment(payment) => payment,
        Payer::CantPay => return Err(InsufficientFunds(0u64, 0u64)),
    };

    //TODO: <IOLITE> do we really need this?
    let intrinsic_gas = executor_gas + payer_gas;
    if intrinsic_gas < executor_gas {
        return Err(IntrinsicGasFailed(0u64, 0u64));
    }

    Ok(payer, meta_logs, payment, intrinsic_gas)
}


fn unpack_business_metadata(from: U256, metadata: Bytes, meta_limit: U256,
                            transaction: &SignedTransaction,
                            read_evm: &mut Executive, write_evm: &mut Executive)
                            //read_state: State, write_state: State)
        // return (payer, meta_logs, payment, intrinsic_gas)
        -> Result<(MetaPayer, MetaLogs, U256, U256), MetaUtilError> {
    println!("[iolite] UnpackBusinessMetadata. Metalimit={}", meta_limit);

    let executor = BusinessMetaExecutor::new(metadata, transaction, from, read_evm);

    //TODO: <IOLITE> do we really need this?
    let executor_gas = executor.intrinsic_gas()?;

    let meta_logs = executor.execute()?;

    let payer = BusinessMetaPayer::new(from, meta_logs, meta_limit, write_state);
    //TODO: <IOLITE> do we really need this?
    let payer_gas = payer.intrinsic_gas()?;

    let payment = match payer.can_pay() {
        //TODO: implement enum for payer: e.g. `enum Payer::PaymentOptions { Payment(u64), CantPay, }`
        Payer::Payment(payment) => payment,
        Payer::CantPay => return Err(InsufficientFunds(0u64, 0u64)),
    };

    //TODO: <IOLITE> do we really need this?
    let intrinsic_gas = executor_gas + payer_gas;
    if intrinsic_gas < executor_gas {
        return Err(IntrinsicGasFailed(0u64, 0u64));
    }

    Ok(payer, meta_logs, payment, intrinsic_gas)
}
