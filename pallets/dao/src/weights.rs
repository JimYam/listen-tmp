// This file is part of Substrate.

// Copyright (C) 2020-2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Weights for pallet_collective
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0
//! DATE: 2020-10-27, STEPS: `[50, ]`, REPEAT: 20, LOW RANGE: [], HIGH RANGE: []
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 128

// Executed Command:
// target/release/substrate
// benchmark
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet_collective
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./frame/collective/src/weights.rs
// --template=./.maintain/frame-weight-template.hbs

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{
	traits::Get,
	weights::{constants::RocksDbWeight, Weight},
};
use sp_std::marker::PhantomData;

const default_weight: u64 = 20_0000_0000;

/// Weight functions needed for pallet_collective.
pub trait WeightInfo {
	fn set_members(_m: u32, _n: u32, _p: u32) -> Weight;
	fn execute(_b: u32, _m: u32) -> Weight;
	fn propose_execute(_b: u32, _m: u32) -> Weight;
	fn propose_proposed(_b: u32, _m: u32, _p: u32) -> Weight;
	fn vote(_m: u32) -> Weight;
	fn close_early_disapproved(_m: u32, _p: u32) -> Weight;
	fn close_early_approved(_b: u32, _m: u32, _p: u32) -> Weight;
	fn close_disapproved(_m: u32, _p: u32) -> Weight;
	fn close_approved(_b: u32, _m: u32, _p: u32) -> Weight;
	fn disapprove_proposal(_p: u32) -> Weight;
}

/// Weights for pallet_collective using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	fn set_members(m: u32, n: u32, p: u32) -> Weight {
		Weight::from_ref_time(default_weight)
	}
	fn execute(b: u32, m: u32) -> Weight {
		Weight::from_ref_time(default_weight)
	}
	fn propose_execute(b: u32, m: u32) -> Weight {
		Weight::from_ref_time(default_weight)
	}
	fn propose_proposed(b: u32, m: u32, p: u32) -> Weight {
		Weight::from_ref_time(default_weight)
	}
	fn vote(m: u32) -> Weight {
		Weight::from_ref_time(default_weight)
	}
	fn close_early_disapproved(m: u32, p: u32) -> Weight {
		Weight::from_ref_time(default_weight)
	}
	fn close_early_approved(b: u32, m: u32, p: u32) -> Weight {
		Weight::from_ref_time(default_weight)
	}
	fn close_disapproved(m: u32, p: u32) -> Weight {
		Weight::from_ref_time(default_weight)
	}
	fn close_approved(b: u32, m: u32, p: u32) -> Weight {
		Weight::from_ref_time(default_weight)
	}
	fn disapprove_proposal(p: u32) -> Weight {
		Weight::from_ref_time(default_weight)
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	fn set_members(m: u32, n: u32, p: u32) -> Weight {
		Weight::from_ref_time(default_weight)
	}
	fn execute(b: u32, m: u32) -> Weight {
		Weight::from_ref_time(default_weight)
	}
	fn propose_execute(b: u32, m: u32) -> Weight {
		Weight::from_ref_time(default_weight)
	}
	fn propose_proposed(b: u32, m: u32, p: u32) -> Weight {
		Weight::from_ref_time(default_weight)
	}
	fn vote(m: u32) -> Weight {
		Weight::from_ref_time(default_weight)
	}
	fn close_early_disapproved(m: u32, p: u32) -> Weight {
		Weight::from_ref_time(default_weight)
	}
	fn close_early_approved(b: u32, m: u32, p: u32) -> Weight {
		Weight::from_ref_time(default_weight)
	}
	fn close_disapproved(m: u32, p: u32) -> Weight {
		Weight::from_ref_time(default_weight)
	}
	fn close_approved(b: u32, m: u32, p: u32) -> Weight {
		Weight::from_ref_time(default_weight)
	}
	fn disapprove_proposal(p: u32) -> Weight {
		Weight::from_ref_time(default_weight)
	}
}
