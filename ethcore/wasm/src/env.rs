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

//! Wasm env module bindings

use parity_wasm::elements::ValueType::*;
use parity_wasm::interpreter::UserFunctionDescriptor;
use parity_wasm::interpreter::UserFunctionDescriptor::*;

pub const SIGNATURES: &'static [UserFunctionDescriptor] = &[
	Static(
		"_storage_read",
		&[I32; 2],
		Some(I32),
	),
	Static(
		"_storage_write",
		&[I32; 2],
		Some(I32),
	),
	Static(
		"_malloc",
		&[I32],
		Some(I32),
	),
	Static(
		"_free",
		&[I32],
		None,
	),
	Static(
		"gas",
		&[I32],
		None,
	),
	Static(
		"_debug",
		&[I32; 2],
		None,
	),
	Static(
		"_suicide",
		&[I32],
		None,
	),
	Static(
		"_create",
		&[I32; 4],
		Some(I32),
	),
	Static(
		"_ccall",
		&[I32; 6],
		Some(I32),
	),
	Static(
		"_dcall",
		&[I32; 5],
		Some(I32),
	),
	Static(
		"_scall",
		&[I32; 5],
		Some(I32),
	),
	Static(
		"abort",
		&[I32],
		None,
	),
	Static(
		"_abort",
		&[],
		None,
	),
	Static(
		"abortOnCannotGrowMemory",
		&[I32; 0],
		Some(I32)
	),

	/*
		THIS IS EXPERIMENTAL RUST-ONLY RUNTIME EXTERNS, THEY ARE SUBJECT TO CHANGE

		AVOID YOUR WASM CONTAINS ANY OF THESE OTHERWISE
			EITHER FACE THE NEED OF HARDFORK
			OR YOU CAN STUCK ON SPECIFIC RUST VERSION FOR WASM COMPILATION
	*/

	Static(
		"_rust_begin_unwind",
		&[I32; 4],
		None,
	),
	Static(
		"_emscripten_memcpy_big",
		&[I32; 3],
		Some(I32),
	),
	Static(
		"___syscall6",
		&[I32; 2],
		Some(I32),
	),
	Static(
		"___syscall140",
		&[I32; 2],
		Some(I32)
	),
	Static(
		"___syscall146",
		&[I32; 2],
		Some(I32)
	),
	Static(
		"___syscall54",
		&[I32; 2],
		Some(I32)
	),
	Static(
		"_llvm_trap",
		&[I32; 0],
		None
	),
	Static(
		"___setErrNo",
		&[I32; 1],
		None
	),
];
