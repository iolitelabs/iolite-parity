// Copyright 2015-2017 Parity Technologies (UK) Ltd.
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

/// Validator set maintained in a contract, updated using `getValidators` method.

use std::sync::Weak;
use ethabi;
use util::*;
use util::cache::MemoryLruCache;
use types::ids::BlockId;
use client::{Client, BlockChainClient};
use super::ValidatorSet;
use super::simple_list::SimpleList;

const MEMOIZE_CAPACITY: usize = 500;
const CONTRACT_INTERFACE: &'static [u8] = b"[{\"constant\":true,\"inputs\":[],\"name\":\"getValidators\",\"outputs\":[{\"name\":\"\",\"type\":\"address[]\"}],\"payable\":false,\"type\":\"function\"}]";
const GET_VALIDATORS: &'static str = "getValidators";

/// The validator contract should have the following interface:
/// [{"constant":true,"inputs":[],"name":"getValidators","outputs":[{"name":"","type":"address[]"}],"payable":false,"type":"function"}]
pub struct ValidatorSafeContract {
	pub address: Address,
	validators: RwLock<MemoryLruCache<H256, SimpleList>>,
	provider: RwLock<Option<provider::Contract>>,
}

impl ValidatorSafeContract {
	pub fn new(contract_address: Address) -> Self {
		ValidatorSafeContract {
			address: contract_address,
			validators: RwLock::new(MemoryLruCache::new(MEMOIZE_CAPACITY)),
			provider: RwLock::new(None),
		}
	}

	/// Queries the state and gets the set of validators.
	fn get_list(&self, block_hash: H256) -> Option<SimpleList> {
		if let Some(ref provider) = *self.provider.read() {
			match provider.get_validators(BlockId::Hash(block_hash)) {
				Ok(new) => {
					debug!(target: "engine", "Set of validators obtained: {:?}", new);
					Some(SimpleList::new(new))
				},
				Err(s) => {
					debug!(target: "engine", "Set of validators could not be updated: {}", s);
					None
				},
			}
		} else {
			warn!(target: "engine", "Set of validators could not be updated: no provider contract.");
			None
		}
	}
}

impl ValidatorSet for ValidatorSafeContract {
	fn contains(&self, block_hash: &H256, address: &Address) -> bool {
		let mut guard = self.validators.write();
		let maybe_existing = guard
			.get_mut(block_hash)
			.map(|list| list.contains(block_hash, address));
		maybe_existing
			.unwrap_or_else(|| self
				.get_list(block_hash.clone())
				.map_or(false, |list| {
					let contains = list.contains(block_hash, address);
					guard.insert(block_hash.clone(), list);
					contains
				 }))
	}

	fn get(&self, block_hash: &H256, nonce: usize) -> Address {
		let mut guard = self.validators.write();
		let maybe_existing = guard
			.get_mut(block_hash)
			.map(|list| list.get(block_hash, nonce));
		maybe_existing
			.unwrap_or_else(|| self
				.get_list(block_hash.clone())
				.map_or_else(Default::default, |list| {
					let address = list.get(block_hash, nonce);
					guard.insert(block_hash.clone(), list);
					address
				 }))
	}

	fn count(&self, block_hash: &H256) -> usize {
		let mut guard = self.validators.write();
		let maybe_existing = guard
			.get_mut(block_hash)
			.map(|list| list.count(block_hash));
		maybe_existing
			.unwrap_or_else(|| self
				.get_list(block_hash.clone())
				.map_or_else(usize::max_value, |list| {
					let address = list.count(block_hash);
					guard.insert(block_hash.clone(), list);
					address
				 }))
	}

	fn register_contract(&self, client: Weak<Client>) {
		trace!(target: "engine", "Setting up contract caller.");
		let contract = ethabi::Contract::new(ethabi::Interface::load(CONTRACT_INTERFACE).expect("JSON interface is valid; qed"));
		let call = contract.function(GET_VALIDATORS.into()).expect("Method name is valid; qed");
		let data = call.encode_call(vec![]).expect("get_validators does not take any arguments; qed");
		let contract_address = self.address.clone();
		let do_call = move |id| client
			.upgrade()
			.ok_or("No client!".into())
			.and_then(|c| c.call_contract(id, contract_address.clone(), data.clone()))
			.map(|raw_output| call.decode_output(raw_output).expect("ethabi is correct; qed"));
		*self.provider.write() = Some(provider::Contract::new(do_call));
	}
}

mod provider {
	use std::string::String;
	use std::result::Result;
	use {util, ethabi};
	use types::ids::BlockId;

	pub struct Contract {
		do_call: Box<Fn(BlockId) -> Result<Vec<ethabi::Token>, String> + Send + Sync + 'static>,
	}

	impl Contract {
		pub fn new<F>(do_call: F) -> Self where F: Fn(BlockId) -> Result<Vec<ethabi::Token>, String> + Send + Sync + 'static {
			Contract {
				do_call: Box::new(do_call),
			}
		}

		/// Gets validators from contract with interface: `{"constant":true,"inputs":[],"name":"getValidators","outputs":[{"name":"","type":"address[]"}],"payable":false,"type":"function"}`
		pub fn get_validators(&self, id: BlockId) -> Result<Vec<util::Address>, String> {
			Ok((self.do_call)(id)?
				 .into_iter()
				 .rev()
				 .collect::<Vec<_>>()
				 .pop()
				 .expect("get_validators returns one argument; qed")
				 .to_array()
				 .and_then(|v| v
					 .into_iter()
					 .map(|a| a.to_address())
					 .collect::<Option<Vec<[u8; 20]>>>())
				 .expect("get_validators returns a list of addresses; qed")
				 .into_iter()
				 .map(util::Address::from)
				 .collect::<Vec<_>>()
			)
		}
	}
}

#[cfg(test)]
mod tests {
	use util::*;
	use types::ids::BlockId;
	use spec::Spec;
	use account_provider::AccountProvider;
	use transaction::{Transaction, Action};
	use client::{BlockChainClient, EngineClient};
	use ethkey::Secret;
	use miner::MinerService;
	use tests::helpers::{generate_dummy_client_with_spec_and_accounts, generate_dummy_client_with_spec_and_data};
	use super::super::ValidatorSet;
	use super::ValidatorSafeContract;

	#[test]
	fn fetches_validators() {
		let client = generate_dummy_client_with_spec_and_accounts(Spec::new_validator_safe_contract, None);
		let vc = Arc::new(ValidatorSafeContract::new(Address::from_str("0000000000000000000000000000000000000005").unwrap()));
		vc.register_contract(Arc::downgrade(&client));
		let last_hash = client.best_block_header().hash();
		assert!(vc.contains(&last_hash, &Address::from_str("7d577a597b2742b498cb5cf0c26cdcd726d39e6e").unwrap()));
		assert!(vc.contains(&last_hash, &Address::from_str("82a978b3f5962a5b0957d9ee9eef472ee55b42f1").unwrap()));
	}

	#[test]
	fn knows_validators() {
		let tap = Arc::new(AccountProvider::transient_provider());
		let s0 = Secret::from_slice(&"1".sha3()).unwrap();
		let v0 = tap.insert_account(s0.clone(), "").unwrap();
		let v1 = tap.insert_account(Secret::from_slice(&"0".sha3()).unwrap(), "").unwrap();
		let network_id = Spec::new_validator_safe_contract().network_id();
		let client = generate_dummy_client_with_spec_and_accounts(Spec::new_validator_safe_contract, Some(tap));
		client.engine().register_client(Arc::downgrade(&client));
		let validator_contract = Address::from_str("0000000000000000000000000000000000000005").unwrap();

		client.miner().set_engine_signer(v1, "".into()).unwrap();
		// Remove "1" validator.
		let tx = Transaction {
			nonce: 0.into(),
			gas_price: 0.into(),
			gas: 500_000.into(),
			action: Action::Call(validator_contract),
			value: 0.into(),
			data: "bfc708a000000000000000000000000082a978b3f5962a5b0957d9ee9eef472ee55b42f1".from_hex().unwrap(),
		}.sign(&s0, Some(network_id));
		client.miner().import_own_transaction(client.as_ref(), tx.into()).unwrap();
		client.update_sealing();
		assert_eq!(client.chain_info().best_block_number, 1);
		// Add "1" validator back in.
		let tx = Transaction {
			nonce: 1.into(),
			gas_price: 0.into(),
			gas: 500_000.into(),
			action: Action::Call(validator_contract),
			value: 0.into(),
			data: "4d238c8e00000000000000000000000082a978b3f5962a5b0957d9ee9eef472ee55b42f1".from_hex().unwrap(),
		}.sign(&s0, Some(network_id));
		client.miner().import_own_transaction(client.as_ref(), tx.into()).unwrap();
		client.update_sealing();
		// The transaction is not yet included so still unable to seal.
		assert_eq!(client.chain_info().best_block_number, 1);

		// Switch to the validator that is still there.
		client.miner().set_engine_signer(v0, "".into()).unwrap();
		client.update_sealing();
		assert_eq!(client.chain_info().best_block_number, 2);
		// Switch back to the added validator, since the state is updated.
		client.miner().set_engine_signer(v1, "".into()).unwrap();
		let tx = Transaction {
			nonce: 2.into(),
			gas_price: 0.into(),
			gas: 21000.into(),
			action: Action::Call(Address::default()),
			value: 0.into(),
			data: Vec::new(),
		}.sign(&s0, Some(network_id));
		client.miner().import_own_transaction(client.as_ref(), tx.into()).unwrap();
		client.update_sealing();
		// Able to seal again.
		assert_eq!(client.chain_info().best_block_number, 3);

		// Check syncing.
		let sync_client = generate_dummy_client_with_spec_and_data(Spec::new_validator_safe_contract, 0, 0, &[]);
		sync_client.engine().register_client(Arc::downgrade(&sync_client));
		for i in 1..4 {
			sync_client.import_block(client.block(BlockId::Number(i)).unwrap().into_inner()).unwrap();
		}
		sync_client.flush_queue();
		assert_eq!(sync_client.chain_info().best_block_number, 3);
	}
}
