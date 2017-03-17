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

use std::sync::Weak;
use ethcore::client::MiningBlockChainClient;
use ethcore::miner::MinerService;
use ethcore::transaction::{Transaction, SignedTransaction, Action};

use jsonrpc_core::Error;
use v1::helpers::CallRequest;
use v1::helpers::dispatch::default_gas_price;

pub fn sign_call<B: MiningBlockChainClient, M: MinerService>(
	client: &Weak<B>,
	miner: &Weak<M>,
	request: CallRequest,
) -> Result<SignedTransaction, Error> {
	let client = take_weak!(client);
	let miner = take_weak!(miner);
	let from = request.from.unwrap_or(0.into());

	Ok(Transaction {
		nonce: request.nonce.unwrap_or_else(|| client.latest_nonce(&from)),
		action: request.to.map_or(Action::Create, Action::Call),
		gas: request.gas.unwrap_or(50_000_000.into()),
		gas_price: request.gas_price.unwrap_or_else(|| default_gas_price(&*client, &*miner)),
		value: request.value.unwrap_or(0.into()),
		data: request.data.map_or_else(Vec::new, |d| d.to_vec())
	}.fake_sign(from))
}
