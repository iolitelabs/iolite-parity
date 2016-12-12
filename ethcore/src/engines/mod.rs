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

//! Consensus engine specification and basic implementations.

mod null_engine;
mod instant_seal;
mod basic_authority;
mod authority_round;

pub use self::null_engine::NullEngine;
pub use self::instant_seal::InstantSeal;
pub use self::basic_authority::BasicAuthority;
pub use self::authority_round::AuthorityRound;

use util::*;
use account_provider::AccountProvider;
use block::ExecutedBlock;
use builtin::Builtin;
use env_info::EnvInfo;
use error::Error;
use spec::CommonParams;
use evm::Schedule;
use io::IoChannel;
use service::ClientIoMessage;
use header::Header;
use transaction::SignedTransaction;
use ethereum::ethash;
use blockchain::extras::BlockDetails;
use views::HeaderView;

/// A consensus mechanism for the chain. Generally either proof-of-work or proof-of-stake-based.
/// Provides hooks into each of the major parts of block import.
pub trait Engine : Sync + Send {
	/// The name of this engine.
	fn name(&self) -> &str;
	/// The version of this engine. Should be of the form
	fn version(&self) -> SemanticVersion { SemanticVersion::new(0, 0, 0) }

	/// The number of additional header fields required for this engine.
	fn seal_fields(&self) -> usize { 0 }

	/// Additional engine-specific information for the user/developer concerning `header`.
	fn extra_info(&self, _header: &Header) -> BTreeMap<String, String> { BTreeMap::new() }

	/// Additional information.
	fn additional_params(&self) -> HashMap<String, String> { HashMap::new() }

	/// Get the general parameters of the chain.
	fn params(&self) -> &CommonParams;

	/// Get the EVM schedule for the given `env_info`.
	fn schedule(&self, env_info: &EnvInfo) -> Schedule;

	/// Builtin-contracts we would like to see in the chain.
	/// (In principle these are just hints for the engine since that has the last word on them.)
	fn builtins(&self) -> &BTreeMap<Address, Builtin>;

	/// Some intrinsic operation parameters; by default they take their value from the `spec()`'s `engine_params`.
	fn maximum_extra_data_size(&self) -> usize { self.params().maximum_extra_data_size }
	/// Maximum number of uncles a block is allowed to declare.
	fn maximum_uncle_count(&self) -> usize { 2 }
	/// The number of generations back that uncles can be.
	fn maximum_uncle_age(&self) -> usize { 6 }
	/// The nonce with which accounts begin.
	fn account_start_nonce(&self) -> U256 { self.params().account_start_nonce }

	/// Block transformation functions, before the transactions.
	fn on_new_block(&self, _block: &mut ExecutedBlock) {}
	/// Block transformation functions, after the transactions.
	fn on_close_block(&self, _block: &mut ExecutedBlock) {}

	/// If Some(true) this author is able to generate seals, generate_seal has to be implemented.
	/// None indicates that this Engine never seals internally regardless of author (e.g. PoW).
	fn is_sealer(&self, _author: &Address) -> Option<bool> { None }
	/// Checks if default address is able to seal.
	fn is_default_sealer(&self) -> Option<bool> { self.is_sealer(&Default::default()) }
	/// Attempt to seal the block internally.
	///
	/// If `Some` is returned, then you get a valid seal.
	///
	/// This operation is synchronous and may (quite reasonably) not be available, in which None will
	/// be returned.
	fn generate_seal(&self, _block: &ExecutedBlock) -> Option<Vec<Bytes>> { None }

	/// Phase 1 quick block verification. Only does checks that are cheap. `block` (the header's full block)
	/// may be provided for additional checks. Returns either a null `Ok` or a general error detailing the problem with import.
	fn verify_block_basic(&self, _header: &Header,  _block: Option<&[u8]>) -> Result<(), Error> { Ok(()) }

	/// Phase 2 verification. Perform costly checks such as transaction signatures. `block` (the header's full block)
	/// may be provided for additional checks. Returns either a null `Ok` or a general error detailing the problem with import.
	fn verify_block_unordered(&self, _header: &Header, _block: Option<&[u8]>) -> Result<(), Error> { Ok(()) }

	/// Phase 3 verification. Check block information against parent and uncles. `block` (the header's full block)
	/// may be provided for additional checks. Returns either a null `Ok` or a general error detailing the problem with import.
	fn verify_block_family(&self, _header: &Header, _parent: &Header, _block: Option<&[u8]>) -> Result<(), Error> { Ok(()) }

	/// Additional verification for transactions in blocks.
	// TODO: Add flags for which bits of the transaction to check.
	// TODO: consider including State in the params.
	fn verify_transaction_basic(&self, _t: &SignedTransaction, _header: &Header) -> Result<(), Error> { Ok(()) }
	/// Verify a particular transaction is valid.
	fn verify_transaction(&self, _t: &SignedTransaction, _header: &Header) -> Result<(), Error> { Ok(()) }

	/// The network ID that transactions should be signed with.
	fn signing_network_id(&self, _env_info: &EnvInfo) -> Option<u64> { None }

	/// Verify the seal of a block. This is an auxilliary method that actually just calls other `verify_` methods
	/// to get the job done. By default it must pass `verify_basic` and `verify_block_unordered`. If more or fewer
	/// methods are needed for an Engine, this may be overridden.
	fn verify_block_seal(&self, header: &Header) -> Result<(), Error> {
		self.verify_block_basic(header, None).and_then(|_| self.verify_block_unordered(header, None))
	}

	/// Populate a header's fields based on its parent's header.
	/// Takes gas floor and ceiling targets.
	/// The gas floor target must not be lower than the engine's minimum gas limit.
	fn populate_from_parent(&self, header: &mut Header, parent: &Header, _gas_floor_target: U256, _gas_ceil_target: U256) {
		header.set_difficulty(parent.difficulty().clone());
		header.set_gas_limit(parent.gas_limit().clone());
	}

	// TODO: builtin contract routing - to do this properly, it will require removing the built-in configuration-reading logic
	// from Spec into here and removing the Spec::builtins field.
	/// Determine whether a particular address is a builtin contract.
	fn is_builtin(&self, a: &Address) -> bool { self.builtins().contains_key(a) }
	/// Determine the code execution cost of the builtin contract with address `a`.
	/// Panics if `is_builtin(a)` is not true.
	fn cost_of_builtin(&self, a: &Address, input: &[u8]) -> U256 {
		self.builtins().get(a).expect("queried cost of nonexistent builtin").cost(input.len())
	}
	/// Execution the builtin contract `a` on `input` and return `output`.
	/// Panics if `is_builtin(a)` is not true.
	fn execute_builtin(&self, a: &Address, input: &[u8], output: &mut BytesRef) {
		self.builtins().get(a).expect("attempted to execute nonexistent builtin").execute(input, output);
	}

	/// Check if new block should be chosen as the one  in chain.
	fn is_new_best_block(&self, best_total_difficulty: U256, _best_header: HeaderView, parent_details: &BlockDetails, new_header: &HeaderView) -> bool {
		ethash::is_new_best_block(best_total_difficulty, parent_details, new_header)
	}

	/// Register an account which signs consensus messages.
	fn set_signer(&self, _address: Address, _password: String) {}

	/// Add a channel for communication with Client which can be used for sealing.
	fn register_message_channel(&self, _message_channel: IoChannel<ClientIoMessage>) {}

	/// Add an account provider useful for Engines that sign stuff.
	fn register_account_provider(&self, _account_provider: Arc<AccountProvider>) {}
	/// Trigger next step of the consensus engine.
	fn step(&self) {}
}
