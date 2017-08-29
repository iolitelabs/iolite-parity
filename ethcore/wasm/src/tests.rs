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

use std::sync::Arc;
use byteorder::{LittleEndian, ByteOrder};
use util::{U256, H256, Address};

use super::WasmInterpreter;
use vm::{self, Vm, GasLeft, ActionParams, ActionValue};
use vm::tests::{FakeCall, FakeExt, FakeCallType};

macro_rules! load_sample {
	($name: expr) => {
		include_bytes!(concat!("../../res/wasm-tests/compiled/", $name)).to_vec()
	}
}

fn test_finalize(res: Result<GasLeft, vm::Error>) -> Result<U256, vm::Error> {
	match res {
		Ok(GasLeft::Known(gas)) => Ok(gas),
		Ok(GasLeft::NeedsReturn{..}) => unimplemented!(), // since ret is unimplemented.
		Err(e) => Err(e),
	}
}

fn wasm_interpreter() -> WasmInterpreter {
	WasmInterpreter::new().expect("wasm interpreter to create without errors")
}

/// Empty contract does almost nothing except producing 1 (one) local node debug log message
#[test]
fn empty() {
	let code = load_sample!("empty.wasm");
	let address: Address = "0f572e5295c57f15886f9b263e2f6d2d6c7b5ec6".parse().unwrap();

	let mut params = ActionParams::default();
	params.address = address.clone();
	params.gas = U256::from(100_000);
	params.code = Some(Arc::new(code));
	let mut ext = FakeExt::new();

	let gas_left = {
		let mut interpreter = wasm_interpreter();
		test_finalize(interpreter.exec(params, &mut ext)).unwrap()
	};

	assert_eq!(gas_left, U256::from(99_992));
}

// This test checks if the contract deserializes payload header properly.
//   Contract is provided with receiver(address), sender, origin and transaction value
//   logger.wasm writes all these provided fixed header fields to some arbitrary storage keys.
#[test]
fn logger() {
	let code = load_sample!("logger.wasm");
	let address: Address = "0f572e5295c57f15886f9b263e2f6d2d6c7b5ec6".parse().unwrap();
	let sender: Address = "0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d".parse().unwrap();
	let origin: Address = "0102030405060708090a0b0c0d0e0f1011121314".parse().unwrap();

	let mut params = ActionParams::default();
	params.address = address.clone();
	params.sender = sender.clone();
	params.origin = origin.clone();
	params.gas = U256::from(100_000);
	params.value = ActionValue::transfer(1_000_000_000);
	params.code = Some(Arc::new(code));
	let mut ext = FakeExt::new();

	let gas_left = {
		let mut interpreter = wasm_interpreter();
		test_finalize(interpreter.exec(params, &mut ext)).unwrap()
	};

	println!("ext.store: {:?}", ext.store);
	assert_eq!(gas_left, U256::from(99327));
	let address_val: H256 = address.into();
	assert_eq!(
		ext.store.get(&"0100000000000000000000000000000000000000000000000000000000000000".parse().unwrap()).expect("storage key to exist"),
		&address_val,
		"Logger sets 0x01 key to the provided address"
	);
	let sender_val: H256 = sender.into();
	assert_eq!(
		ext.store.get(&"0200000000000000000000000000000000000000000000000000000000000000".parse().unwrap()).expect("storage key to exist"),
		&sender_val,
		"Logger sets 0x02 key to the provided sender"
	);
	let origin_val: H256 = origin.into();
	assert_eq!(
		ext.store.get(&"0300000000000000000000000000000000000000000000000000000000000000".parse().unwrap()).expect("storage key to exist"),
		&origin_val,
		"Logger sets 0x03 key to the provided origin"
	);
	assert_eq!(
		U256::from(ext.store.get(&"0400000000000000000000000000000000000000000000000000000000000000".parse().unwrap()).expect("storage key to exist")),
		U256::from(1_000_000_000),
		"Logger sets 0x04 key to the trasferred value"
	);
}

// This test checks if the contract can allocate memory and pass pointer to the result stream properly.
//   1. Contract is being provided with the call descriptor ptr
//   2. Descriptor ptr is 16 byte length
//   3. The last 8 bytes of call descriptor is the space for the contract to fill [result_ptr[4], result_len[4]]
//      if it has any result.
#[test]
fn identity() {
	let code = load_sample!("identity.wasm");
	let sender: Address = "01030507090b0d0f11131517191b1d1f21232527".parse().unwrap();

	let mut params = ActionParams::default();
	params.sender = sender.clone();
	params.gas = U256::from(100_000);
	params.code = Some(Arc::new(code));
	let mut ext = FakeExt::new();

	let (gas_left, result) = {
		let mut interpreter = wasm_interpreter();
		let result = interpreter.exec(params, &mut ext).expect("Interpreter to execute without any errors");
		match result {
			GasLeft::Known(_) => { panic!("Identity contract should return payload"); },
			GasLeft::NeedsReturn { gas_left: gas, data: result, apply_state: _apply } => (gas, result.to_vec()),
		}
	};

	assert_eq!(gas_left, U256::from(99_672));

	assert_eq!(
		Address::from_slice(&result),
		sender,
		"Idenity test contract does not return the sender passed"
	);
}

// Dispersion test sends byte array and expect the contract to 'disperse' the original elements with
// their modulo 19 dopant.
// The result is always twice as long as the input.
// This also tests byte-perfect memory allocation and in/out ptr lifecycle.
#[test]
fn dispersion() {
	let code = load_sample!("dispersion.wasm");

	let mut params = ActionParams::default();
	params.gas = U256::from(100_000);
	params.code = Some(Arc::new(code));
	params.data = Some(vec![
		0u8, 125, 197, 255, 19
	]);
	let mut ext = FakeExt::new();

	let (gas_left, result) = {
		let mut interpreter = wasm_interpreter();
		let result = interpreter.exec(params, &mut ext).expect("Interpreter to execute without any errors");
		match result {
			GasLeft::Known(_) => { panic!("Dispersion routine should return payload"); },
			GasLeft::NeedsReturn { gas_left: gas, data: result, apply_state: _apply } => (gas, result.to_vec()),
		}
	};

	assert_eq!(gas_left, U256::from(99_270));

	assert_eq!(
		result,
		vec![0u8, 0, 125, 11, 197, 7, 255, 8, 19, 0]
	);
}

#[test]
fn suicide_not() {
	let code = load_sample!("suicidal.wasm");

	let mut params = ActionParams::default();
	params.gas = U256::from(100_000);
	params.code = Some(Arc::new(code));
	params.data = Some(vec![
		0u8
	]);
	let mut ext = FakeExt::new();

	let (gas_left, result) = {
		let mut interpreter = wasm_interpreter();
		let result = interpreter.exec(params, &mut ext).expect("Interpreter to execute without any errors");
		match result {
			GasLeft::Known(_) => { panic!("Suicidal contract should return payload when had not actualy killed himself"); },
			GasLeft::NeedsReturn { gas_left: gas, data: result, apply_state: _apply } => (gas, result.to_vec()),
		}
	};

	assert_eq!(gas_left, U256::from(99_578));

	assert_eq!(
		result,
		vec![0u8]
	);
}

#[test]
fn suicide() {
	let code = load_sample!("suicidal.wasm");

	let refund: Address = "01030507090b0d0f11131517191b1d1f21232527".parse().unwrap();
	let mut params = ActionParams::default();
	params.gas = U256::from(100_000);
	params.code = Some(Arc::new(code));

	let mut args = vec![127u8];
	args.extend(refund.to_vec());
	params.data = Some(args);

	let mut ext = FakeExt::new();

	let gas_left = {
		let mut interpreter = wasm_interpreter();
		let result = interpreter.exec(params, &mut ext).expect("Interpreter to execute without any errors");
		match result {
			GasLeft::Known(gas) => gas,
			GasLeft::NeedsReturn { .. } => {
				panic!("Suicidal contract should not return anything when had killed itself");
			},
		}
	};

	assert_eq!(gas_left, U256::from(99_621));
	assert!(ext.suicides.contains(&refund));
}

#[test]
fn create() {
	::ethcore_logger::init_log();

	let mut params = ActionParams::default();
	params.gas = U256::from(100_000);
	params.code = Some(Arc::new(load_sample!("creator.wasm")));
	params.data = Some(vec![0u8, 2, 4, 8, 16, 32, 64, 128]);
	params.value = ActionValue::transfer(1_000_000_000);

	let mut ext = FakeExt::new();

	let gas_left = {
		let mut interpreter = wasm_interpreter();
		let result = interpreter.exec(params, &mut ext).expect("Interpreter to execute without any errors");
		match result {
			GasLeft::Known(gas) => gas,
			GasLeft::NeedsReturn { .. } => {
				panic!("Create contract should not return anthing because ext always fails on creation");
			},
		}
	};

	trace!(target: "wasm", "fake_calls: {:?}", &ext.calls);
	assert!(ext.calls.contains(
		&FakeCall {
			call_type: FakeCallType::Create,
			gas: U256::from(99_674),
			sender_address: None,
			receive_address: None,
			value: Some(1_000_000_000.into()),
			data: vec![0u8, 2, 4, 8, 16, 32, 64, 128],
			code_address: None,
		}
	));
	assert_eq!(gas_left, U256::from(99_596));
}


#[test]
fn call_code() {
	::ethcore_logger::init_log();

	let sender: Address = "01030507090b0d0f11131517191b1d1f21232527".parse().unwrap();
	let receiver: Address = "0f572e5295c57f15886f9b263e2f6d2d6c7b5ec6".parse().unwrap();

	let mut params = ActionParams::default();
	params.sender = sender.clone();
	params.address = receiver.clone();
	params.gas = U256::from(100_000);
	params.code = Some(Arc::new(load_sample!("call_code.wasm")));
	params.data = Some(Vec::new());
	params.value = ActionValue::transfer(1_000_000_000);

	let mut ext = FakeExt::new();

	let (gas_left, result) = {
		let mut interpreter = wasm_interpreter();
		let result = interpreter.exec(params, &mut ext).expect("Interpreter to execute without any errors");
		match result {
			GasLeft::Known(_) => { panic!("Call test should return payload"); },
			GasLeft::NeedsReturn { gas_left: gas, data: result, apply_state: _apply } => (gas, result.to_vec()),
		}
	};

	trace!(target: "wasm", "fake_calls: {:?}", &ext.calls);
	assert!(ext.calls.contains(
		&FakeCall {
			call_type: FakeCallType::Call,
			gas: U256::from(99_069),
			sender_address: Some(sender),
			receive_address: Some(receiver),
			value: None,
			data: vec![1u8, 2, 3, 5, 7, 11],
			code_address: Some("0d13710000000000000000000000000000000000".parse().unwrap()),
		}
	));
	assert_eq!(gas_left, U256::from(94144));

	// siphash result
	let res = LittleEndian::read_u32(&result[..]);
	assert_eq!(res, 4198595614);
}

#[test]
fn call_static() {
	::ethcore_logger::init_log();

	let sender: Address = "0f572e5295c57f15886f9b263e2f6d2d6c7b5ec6".parse().unwrap();
	let receiver: Address = "01030507090b0d0f11131517191b1d1f21232527".parse().unwrap();

	let mut params = ActionParams::default();
	params.sender = sender.clone();
	params.address = receiver.clone();
	params.gas = U256::from(100_000);
	params.code = Some(Arc::new(load_sample!("call_static.wasm")));
	params.data = Some(Vec::new());
	params.value = ActionValue::transfer(1_000_000_000);

	let mut ext = FakeExt::new();

	let (gas_left, result) = {
		let mut interpreter = wasm_interpreter();
		let result = interpreter.exec(params, &mut ext).expect("Interpreter to execute without any errors");
		match result {
			GasLeft::Known(_) => { panic!("Static call test should return payload"); },
			GasLeft::NeedsReturn { gas_left: gas, data: result, apply_state: _apply } => (gas, result.to_vec()),
		}
	};

	trace!(target: "wasm", "fake_calls: {:?}", &ext.calls);
	assert!(ext.calls.contains(
		&FakeCall {
			call_type: FakeCallType::Call,
			gas: U256::from(99_069),
			sender_address: Some(sender),
			receive_address: Some(receiver),
			value: None,
			data: vec![1u8, 2, 3, 5, 7, 11],
			code_address: Some("13077bfb00000000000000000000000000000000".parse().unwrap()),
		}
	));
	assert_eq!(gas_left, U256::from(94144));

	// siphash result
	let res = LittleEndian::read_u32(&result[..]);
	assert_eq!(res, 317632590);
}

// Realloc test
#[test]
fn realloc() {
	let code = load_sample!("realloc.wasm");

	let mut params = ActionParams::default();
	params.gas = U256::from(100_000);
	params.code = Some(Arc::new(code));
	params.data = Some(vec![0u8]);
	let mut ext = FakeExt::new();

	let (gas_left, result) = {
		let mut interpreter = wasm_interpreter();
		let result = interpreter.exec(params, &mut ext).expect("Interpreter to execute without any errors");
		match result {
				GasLeft::Known(_) => { panic!("Realloc should return payload"); },
				GasLeft::NeedsReturn { gas_left: gas, data: result, apply_state: _apply } => (gas, result.to_vec()),
		}
	};
	assert_eq!(gas_left, U256::from(99432));
	assert_eq!(result, vec![0u8; 2]);
}

// Tests that contract's ability to read from a storage
// Test prepopulates address into storage, than executes a contract which read that address from storage and write this address into result
#[test]
fn storage_read() {
	let code = load_sample!("storage_read.wasm");
	let address: Address = "0f572e5295c57f15886f9b263e2f6d2d6c7b5ec6".parse().unwrap();

	let mut params = ActionParams::default();
	params.gas = U256::from(100_000);
	params.code = Some(Arc::new(code));
	let mut ext = FakeExt::new();
	ext.store.insert("0100000000000000000000000000000000000000000000000000000000000000".into(), address.into());

	let (gas_left, result) = {
		let mut interpreter = wasm_interpreter();
		let result = interpreter.exec(params, &mut ext).expect("Interpreter to execute without any errors");
		match result {
				GasLeft::Known(_) => { panic!("storage_read should return payload"); },
				GasLeft::NeedsReturn { gas_left: gas, data: result, apply_state: _apply } => (gas, result.to_vec()),
		}
	};

	assert_eq!(gas_left, U256::from(99682));
	assert_eq!(Address::from(&result[12..32]), address);
}

macro_rules! reqrep_test {
	($name: expr, $input: expr) => {
		{
			::ethcore_logger::init_log();
			let code = load_sample!($name);

			let mut params = ActionParams::default();
			params.gas = U256::from(100_000);
			params.code = Some(Arc::new(code));
			params.data = Some($input);

			let (gas_left, result) = {
				let mut interpreter = wasm_interpreter();
				let result = interpreter.exec(params, &mut FakeExt::new()).expect("Interpreter to execute without any errors");
				match result {
						GasLeft::Known(_) => { panic!("Test is expected to return payload to check"); },
						GasLeft::NeedsReturn { gas_left: gas, data: result, apply_state: _apply } => (gas, result.to_vec()),
				}
			};

			(gas_left, result)
		}
	}
}

// math_* tests check the ability of wasm contract to perform big integer operations
// - addition
// - multiplication
// - substraction
// - division

// addition
#[test]
fn math_add() {

	let (gas_left, result) = reqrep_test!(
		"math.wasm",
		{
			let mut args = [0u8; 65];
			let arg_a = U256::from_dec_str("999999999999999999999999999999").unwrap();
			let arg_b = U256::from_dec_str("888888888888888888888888888888").unwrap();
			arg_a.to_big_endian(&mut args[1..33]);
			arg_b.to_big_endian(&mut args[33..65]);
			args.to_vec()
		}
	);

	assert_eq!(gas_left, U256::from(98087));
	assert_eq!(
		U256::from_dec_str("1888888888888888888888888888887").unwrap(),
		(&result[..]).into()
	);
}

// multiplication
#[test]
fn math_mul() {
	let (gas_left, result) = reqrep_test!(
		"math.wasm",
		{
			let mut args = [1u8; 65];
			let arg_a = U256::from_dec_str("888888888888888888888888888888").unwrap();
			let arg_b = U256::from_dec_str("999999999999999999999999999999").unwrap();
			arg_a.to_big_endian(&mut args[1..33]);
			arg_b.to_big_endian(&mut args[33..65]);
			args.to_vec()
		}
	);

	assert_eq!(gas_left, U256::from(97236));
	assert_eq!(
		U256::from_dec_str("888888888888888888888888888887111111111111111111111111111112").unwrap(),
		(&result[..]).into()
	);
}

// substraction
#[test]
fn math_sub() {
	let (gas_left, result) = reqrep_test!(
		"math.wasm",
		{
			let mut args = [2u8; 65];
			let arg_a = U256::from_dec_str("999999999999999999999999999999").unwrap();
			let arg_b = U256::from_dec_str("888888888888888888888888888888").unwrap();
			arg_a.to_big_endian(&mut args[1..33]);
			arg_b.to_big_endian(&mut args[33..65]);
			args.to_vec()
		}
	);

	assert_eq!(gas_left, U256::from(98131));
	assert_eq!(
		U256::from_dec_str("111111111111111111111111111111").unwrap(),
		(&result[..]).into()
	);
}

#[test]
fn math_div() {
	let (gas_left, result) = reqrep_test!(
		"math.wasm",
		{
			let mut args = [3u8; 65];
			let arg_a = U256::from_dec_str("999999999999999999999999999999").unwrap();
			let arg_b = U256::from_dec_str("888888888888888888888888").unwrap();
			arg_a.to_big_endian(&mut args[1..33]);
			arg_b.to_big_endian(&mut args[33..65]);
			args.to_vec()
		}
	);

	assert_eq!(gas_left, U256::from(91420));
	assert_eq!(
		U256::from_dec_str("1125000").unwrap(),
		(&result[..]).into()
	);
}
