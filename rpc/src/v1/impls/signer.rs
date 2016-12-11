// Copyright 2015, 2016 Parity Technologies (UK) Ltd.
// This file is part of Parity.

// Parity is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity.  If not, see <http://www.gnu.org/licenses/>.

//! Transactions Confirmations rpc implementation

use std::sync::{Arc, Weak};

use rlp::{UntrustedRlp, View};
use ethcore::account_provider::AccountProvider;
use ethcore::client::MiningBlockChainClient;
use ethcore::transaction::SignedTransaction;
use ethcore::miner::MinerService;

use jsonrpc_core::Error;
use v1::traits::Signer;
use v1::types::{TransactionModification, ConfirmationRequest, ConfirmationResponse, U256, Bytes};
use v1::helpers::{errors, SignerService, SigningQueue, ConfirmationPayload};
use v1::helpers::dispatch::{self, dispatch_transaction};

/// Transactions confirmation (personal) rpc implementation.
pub struct SignerClient<C, M> where C: MiningBlockChainClient, M: MinerService {
	signer: Weak<SignerService>,
	accounts: Weak<AccountProvider>,
	client: Weak<C>,
	miner: Weak<M>,
}

impl<C: 'static, M: 'static> SignerClient<C, M> where C: MiningBlockChainClient, M: MinerService {

	/// Create new instance of signer client.
	pub fn new(
		store: &Arc<AccountProvider>,
		client: &Arc<C>,
		miner: &Arc<M>,
		signer: &Arc<SignerService>,
	) -> Self {
		SignerClient {
			signer: Arc::downgrade(signer),
			accounts: Arc::downgrade(store),
			client: Arc::downgrade(client),
			miner: Arc::downgrade(miner),
		}
	}

	fn active(&self) -> Result<(), Error> {
		// TODO: only call every 30s at most.
		take_weak!(self.client).keep_alive();
		Ok(())
	}
}

impl<C: 'static, M: 'static> Signer for SignerClient<C, M> where C: MiningBlockChainClient, M: MinerService {

	fn requests_to_confirm(&self) -> Result<Vec<ConfirmationRequest>, Error> {
		try!(self.active());
		let signer = take_weak!(self.signer);

		Ok(signer.requests()
			.into_iter()
			.map(Into::into)
			.collect()
		)
	}

	// TODO [ToDr] TransactionModification is redundant for some calls
	// might be better to replace it in future
	fn confirm_request(&self, id: U256, modification: TransactionModification, pass: String) -> Result<ConfirmationResponse, Error> {
		try!(self.active());

		let id = id.into();
		let accounts = take_weak!(self.accounts);
		let signer = take_weak!(self.signer);
		let client = take_weak!(self.client);
		let miner = take_weak!(self.miner);

		signer.peek(&id).map(|confirmation| {
			let mut payload = confirmation.payload.clone();
			// Modify payload
			if let ConfirmationPayload::SendTransaction(ref mut request) = payload {
				if let Some(gas_price) = modification.gas_price {
					request.gas_price = gas_price.into();
				}
				if let Some(gas) = modification.gas {
					request.gas = gas.into();
				}
			}
			// Execute
			let result = dispatch::execute(&*client, &*miner, &*accounts, payload, Some(pass));
			if let Ok(ref response) = result {
				signer.request_confirmed(id, Ok(response.clone()));
			}
			result
		}).unwrap_or_else(|| Err(errors::invalid_params("Unknown RequestID", id)))
	}

	fn confirm_request_raw(&self, id: U256, bytes: Bytes) -> Result<ConfirmationResponse, Error> {
		try!(self.active());

		let id = id.into();
		let signer = take_weak!(self.signer);
		let client = take_weak!(self.client);
		let miner = take_weak!(self.miner);

		signer.peek(&id).map(|confirmation| {
			let result = match confirmation.payload {
				ConfirmationPayload::SendTransaction(request) => {
					let signed_transaction: SignedTransaction = try!(
						UntrustedRlp::new(&bytes.0).as_val().map_err(errors::from_rlp_error)
					);
					let sender = try!(
						signed_transaction.sender().map_err(|e| errors::invalid_params("Invalid signature.", e))
					);

					// Verification
					let sender_matches = sender == request.from;
					let data_matches = signed_transaction.data == request.data;
					let value_matches = signed_transaction.value == request.value;
					let nonce_matches = match request.nonce {
						Some(nonce) => signed_transaction.nonce == nonce,
						None => true,
					};

					// Dispatch if everything is ok
					if sender_matches && data_matches && value_matches && nonce_matches {
						dispatch_transaction(&*client, &*miner, signed_transaction)
							.map(Into::into)
							.map(ConfirmationResponse::SendTransaction)
					} else {
						let mut error = Vec::new();
						if !sender_matches { error.push("from") }
						if !data_matches { error.push("data") }
						if !value_matches { error.push("value") }
						if !nonce_matches { error.push("nonce") }

						Err(errors::invalid_params("Sent transaction does not match the request.", error))
					}
				},
				// TODO [ToDr]:
				// 1. Sign - verify signature
				// 2. Decrypt - pass through?
				_ => Err(errors::unimplemented(Some("Non-transaction requests does not support RAW signing yet.".into()))),
			};
			if let Ok(ref response) = result {
				signer.request_confirmed(id, Ok(response.clone()));
			}
			result
		}).unwrap_or_else(|| Err(errors::invalid_params("Unknown RequestID", id)))
	}

	fn reject_request(&self, id: U256) -> Result<bool, Error> {
		try!(self.active());
		let signer = take_weak!(self.signer);

		let res = signer.request_rejected(id.into());
		Ok(res.is_some())
	}

	fn generate_token(&self) -> Result<String, Error> {
		try!(self.active());
		let signer = take_weak!(self.signer);

		signer.generate_token()
			.map_err(|e| errors::token(e))
	}
}

