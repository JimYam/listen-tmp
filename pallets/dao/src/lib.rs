// Forked from https://github.com/paritytech/substrate/tree/master/frame/collective.

// Copyright 2021 LISTEN Developer.
// This file is part of LISTEN.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "128"]

pub use crate::pallet::*;
use frame_support::{
	codec::{Decode, Encode},
	dispatch::{
		DispatchError, DispatchResult, DispatchResultWithPostInfo, Dispatchable, Parameter,
		PostDispatchInfo,
	},
	ensure,
	traits::{Contains, EnsureOrigin, Get},
	weights::GetDispatchInfo,
};
use frame_system::{self as system, ensure_root, ensure_signed};
use listen_primitives::traits::{CollectiveHandler, ListenHandler};
use scale_info::TypeInfo;
use sp_runtime::{traits::Hash, RuntimeDebug};
use sp_std::{convert::From, prelude::*, result};
pub use weights::WeightInfo;

pub mod weights;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

/// Simple index type for proposal counting.
pub type ProposalIndex = u32;
pub type RoomIndex = u64;
/// A number of members.
///
/// This also serves as a number of voting members, and since for motions, each member may
/// vote exactly once, therefore also the number of votes for any given motion.
pub type MemberCount = u32;
pub type IsEnd = bool;
pub type IsPass = bool;

/// Default voting strategy when a member is inactive.
pub trait DefaultVote {
	/// Get the default voting strategy, given:
	///
	/// - Whether the prime member voted Aye.
	/// - Raw number of yes votes.
	/// - Raw number of no votes.
	/// - Total number of member count.
	/// - Total number of member count.
	fn default_vote(
		prime_vote: Option<bool>,
		yes_votes: MemberCount,
		no_votes: MemberCount,
		len: MemberCount,
	) -> bool;
}

/// Set the prime member's vote as the default vote.
pub struct PrimeDefaultVote;

impl DefaultVote for PrimeDefaultVote {
	fn default_vote(
		prime_vote: Option<bool>,
		_yes_votes: MemberCount,
		_no_votes: MemberCount,
		_len: MemberCount,
	) -> bool {
		prime_vote.unwrap_or(false)
	}
}

/// First see if yes vote are over majority of the whole collective. If so, set the default vote
/// as yes. Otherwise, use the prime meber's vote as the default vote.
pub struct MoreThanMajorityThenPrimeDefaultVote;

impl DefaultVote for MoreThanMajorityThenPrimeDefaultVote {
	fn default_vote(
		prime_vote: Option<bool>,
		yes_votes: MemberCount,
		_no_votes: MemberCount,
		len: MemberCount,
	) -> bool {
		let more_than_majority = yes_votes * 2 > len;
		more_than_majority || prime_vote.unwrap_or(false)
	}
}

/// Origin for the collective module.
#[derive(PartialEq, Eq, Clone, RuntimeDebug, Encode, Decode, TypeInfo)]
#[scale_info(skip_type_params(I))]
pub enum RoomRawOrigin<AccountId, I> {
	/// It has been condoned by a given number of members of the collective from a given total.
	Members(MemberCount, MemberCount),
	/// It has been condoned by a single member of the collective.
	Member(RoomIndex, AccountId),
	/// Dummy to manage the fact we have instancing.
	_Phantom(sp_std::marker::PhantomData<I>),
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
/// Info for keeping track of a motion being voted on.
pub struct ListenDaoVotes<AccountId, BlockNumber> {
	/// The proposal's unique index.
	index: ProposalIndex,
	/// The proposal's reason,
	reason: Option<Vec<u8>>,
	/// The number of approval ListenDaoVotes that are needed to pass the motion.
	threshold: MemberCount,
	/// The current set of voters that approved it.
	ayes: Vec<AccountId>,
	/// The current set of voters that rejected it.
	nays: Vec<AccountId>,
	/// The hard end time of this vote.
	end: BlockNumber,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::{
		Blake2_128Concat, IsType, OptionQuery, PhantomData, StorageDoubleMap, StorageMap,
		ValueQuery,
	};
	use frame_system::pallet_prelude::*;

	#[pallet::config]
	#[pallet::disable_frame_system_supertrait_check]
	pub trait Config<I: 'static = ()>: frame_system::Config {
		/// The outer origin type.
		type Origin: From<RoomRawOrigin<Self::AccountId, I>>;
		/// The outer call dispatch type.
		type Proposal: Parameter
			+ Dispatchable<Origin = <Self as Config<I>>::Origin, PostInfo = PostDispatchInfo>
			+ From<frame_system::Call<Self>>
			+ GetDispatchInfo;
		/// The outer event type.
		type Event: From<Event<Self, I>>
			+ Into<<Self as frame_system::Config>::Event>
			+ IsType<<Self as frame_system::Config>::Event>;
		/// Default vote strategy of this collective.
		type DefaultVote: DefaultVote;
		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;
		type ListenHandler: ListenHandler<u64, Self::AccountId, DispatchError, u128>;
		type BaseCallFilter: Contains<Self::Proposal>;
		#[pallet::constant]
		type MotionDuration: Get<Self::BlockNumber>;
		/// Maximum number of proposals allowed to be active in parallel.
		#[pallet::constant]
		type MaxProposals: Get<ProposalIndex>;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(pub (super) trait Store)]
	pub struct Pallet<T, I = ()>(PhantomData<(T, I)>);

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		/// A motion (given hash) has been proposed (by given account) with a threshold (given
		/// `MemberCount`).
		/// \[account, proposal_index, proposal_hash, threshold\]
		Proposed(T::AccountId, ProposalIndex, T::Hash, MemberCount),
		/// A motion (given hash) has been voted on by given account, leaving
		/// a tally (yes votes and no votes given respectively as `MemberCount`).
		/// \[account, proposal_hash, voted, yes, no\]
		Voted(T::AccountId, T::Hash, bool, MemberCount, MemberCount, MemberCount),
		/// A motion was approved by the required threshold.
		/// \[proposal_hash\]
		Approved(T::Hash),
		/// A motion was not approved by the required threshold.
		/// \[proposal_hash\]
		Disapproved(T::Hash),
		/// A motion was executed; result will be `Ok` if it returned without error.
		/// \[proposal_hash, result\]
		Executed(T::Hash, DispatchResult),
		/// A single member did some action; result will be `Ok` if it returned without error.
		/// \[proposal_hash, result\]
		MemberExecuted(T::Hash, DispatchResult),
		/// A proposal was closed because its threshold was reached or after its duration was up.
		/// \[proposal_hash, yes, no\]
		Closed(T::Hash, MemberCount, MemberCount),
	}

	/// Origin for the collective pallet.
	#[pallet::origin]
	pub type Origin<T, I = ()> = RoomRawOrigin<<T as frame_system::Config>::AccountId, I>;

	#[pallet::storage]
	#[pallet::getter(fn proposals)]
	pub type Proposals<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, RoomIndex, Vec<T::Hash>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn proposal_of)]
	pub type ProposalOf<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		RoomIndex,
		Blake2_128Concat,
		T::Hash,
		T::Proposal,
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn voting)]
	pub type Voting<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		RoomIndex,
		Blake2_128Concat,
		T::Hash,
		ListenDaoVotes<T::AccountId, T::BlockNumber>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn proposal_count)]
	pub type ProposalCount<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, RoomIndex, u32, ValueQuery>;

	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// Account is not a member
		NotMember,
		NotRoomOwner,
		/// Duplicate proposals not allowed
		DuplicateProposal,
		/// Proposal must exist
		ProposalMissing,
		/// Mismatched index
		WrongIndex,
		/// Duplicate vote ignored
		DuplicateVote,
		/// There can only be a maximum of `MaxProposals` active proposals.
		TooManyProposals,
		/// The given length bound for the proposal was too low.
		WrongProposalLength,
		VoteExpire,
		DisallowFunc,
	}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		/// Method of direct execution by the group master.
		#[pallet::weight(1500_000_000)]
		pub fn execute(
			origin: OriginFor<T>,
			room_id: RoomIndex,
			proposal: Box<<T as Config<I>>::Proposal>,
			#[pallet::compact] length_bound: u32,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(
				<T as pallet::Config<I>>::BaseCallFilter::contains(&proposal),
				Error::<T, I>::DisallowFunc
			);
			let members = T::ListenHandler::get_room_council(room_id.into())?;
			let room_owner = T::ListenHandler::get_root(room_id.into())?;

			ensure!(members.contains(&who), Error::<T, I>::NotMember);
			ensure!(room_owner == who, Error::<T, I>::NotRoomOwner);

			let proposal_len = proposal.using_encoded(|x| x.len());
			ensure!(proposal_len <= length_bound as usize, Error::<T, I>::WrongProposalLength);

			let proposal_hash = T::Hashing::hash_of(&proposal);
			let result = proposal.dispatch(RoomRawOrigin::Member(room_id, who).into());

			Self::deposit_event(Event::MemberExecuted(
				proposal_hash,
				result.map(|_| ()).map_err(|e| e.error),
			));
			Ok(())
		}

		/// A group of members of parliament introduced a motion.
		#[pallet::weight(1500_000_000)]
		pub fn propose(
			origin: OriginFor<T>,
			room_id: RoomIndex,
			#[pallet::compact] threshold: MemberCount,
			proposal: Box<<T as Config<I>>::Proposal>,
			reason: Option<Vec<u8>>,
			#[pallet::compact] length_bound: u32,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(
				<T as pallet::Config<I>>::BaseCallFilter::contains(&proposal),
				Error::<T, I>::DisallowFunc
			);
			let members = T::ListenHandler::get_room_council(room_id.into())?;
			ensure!(members.contains(&who), Error::<T, I>::NotMember);

			let proposal_len = proposal.using_encoded(|x| x.len());
			ensure!(proposal_len <= length_bound as usize, Error::<T, I>::WrongProposalLength);

			let proposal_hash = T::Hashing::hash_of(&proposal);
			ensure!(
				!<ProposalOf<T, I>>::contains_key(room_id, proposal_hash),
				Error::<T, I>::DuplicateProposal
			);

			if threshold < 2 {
				let seats = members.len() as MemberCount;
				let result = proposal.dispatch(RoomRawOrigin::Members(1, seats).into());
				Self::deposit_event(Event::Executed(
					proposal_hash,
					result.map(|_| ()).map_err(|e| e.error),
				));
				Ok(())
			} else {
				let _active_proposals = <Proposals<T, I>>::try_mutate(
					room_id,
					|proposals| -> Result<usize, DispatchError> {
						proposals.push(proposal_hash);
						ensure!(
							proposals.len() <= T::MaxProposals::get() as usize,
							Error::<T, I>::TooManyProposals
						);
						Ok(proposals.len())
					},
				)?;
				let index = Self::proposal_count(room_id);
				ProposalCount::<T, I>::mutate(room_id, |i| *i += 1);
				<ProposalOf<T, I>>::insert(room_id, proposal_hash, *proposal);
				let end = system::Pallet::<T>::block_number() + T::MotionDuration::get();
				let votes = ListenDaoVotes {
					index,
					reason,
					threshold,
					ayes: vec![who.clone()],
					nays: vec![],
					end,
				};
				<Voting<T, I>>::insert(room_id, proposal_hash, votes);

				Self::deposit_event(Event::Proposed(who, index, proposal_hash, threshold));
				Ok(())
			}
		}

		#[pallet::weight(1500_000_000)]
		pub fn vote(
			origin: OriginFor<T>,
			room_id: RoomIndex,
			proposal: T::Hash,
			#[pallet::compact] index: ProposalIndex,
			approve: bool,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let members = T::ListenHandler::get_room_council(room_id.into())?;
			let seats = members.len() as MemberCount;
			ensure!(members.contains(&who), Error::<T, I>::NotMember);

			let mut voting =
				Self::voting(room_id, &proposal).ok_or(Error::<T, I>::ProposalMissing)?;
			ensure!(voting.index == index, Error::<T, I>::WrongIndex);

			let position_yes = voting.ayes.iter().position(|a| a == &who);
			let position_no = voting.nays.iter().position(|a| a == &who);

			if approve {
				if position_yes.is_none() {
					voting.ayes.push(who.clone());
				} else {
					Err(Error::<T, I>::DuplicateVote)?
				}
				if let Some(pos) = position_no {
					voting.nays.swap_remove(pos);
				}
			} else {
				if position_no.is_none() {
					voting.nays.push(who.clone());
				} else {
					Err(Error::<T, I>::DuplicateVote)?
				}
				if let Some(pos) = position_yes {
					voting.ayes.swap_remove(pos);
				}
			}

			let yes_votes = voting.ayes.len() as MemberCount;
			let no_votes = voting.nays.len() as MemberCount;
			Self::deposit_event(Event::Voted(who, proposal, approve, seats, yes_votes, no_votes));
			Voting::<T, I>::insert(room_id, &proposal, voting.clone());

			Self::normal_close(voting.clone(), room_id, proposal)?;
			Ok(())
		}

		#[pallet::weight(1500_000_000)]
		pub fn disapprove_proposal(
			origin: OriginFor<T>,
			room_id: RoomIndex,
			proposal_hash: T::Hash,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			let proposal_count = Self::do_disapprove_proposal(room_id, proposal_hash);
			Ok(Some(T::WeightInfo::disapprove_proposal(proposal_count)).into())
		}
	}

	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		fn normal_close(
			voting: ListenDaoVotes<T::AccountId, T::BlockNumber>,
			room_id: RoomIndex,
			proposal_hash: T::Hash,
		) -> DispatchResult {
			let no_votes = voting.nays.len() as MemberCount;
			let yes_votes = voting.ayes.len() as MemberCount;
			let seats = T::ListenHandler::get_room_council(room_id.into())?.len() as MemberCount;

			let result: (IsEnd, IsPass) = Self::vote_result(&voting, room_id)?;

			if result.0 {
				if result.1 {
					let proposal = ProposalOf::<T, I>::get(room_id, proposal_hash)
						.ok_or(Error::<T, I>::ProposalMissing)?;
					Self::do_approve_proposal(room_id, seats, voting, proposal_hash, proposal);
					Self::deposit_event(Event::Closed(proposal_hash, yes_votes, no_votes));
				} else {
					Self::do_disapprove_proposal(room_id, proposal_hash);
					if Self::is_expire(&voting) {
						return Err(Error::<T, I>::VoteExpire)?
					}
					Self::deposit_event(Event::Closed(proposal_hash, yes_votes, no_votes));
				}
			}

			Ok(())
		}

		fn is_expire(voting: &ListenDaoVotes<T::AccountId, T::BlockNumber>) -> bool {
			if voting.end <= system::Pallet::<T>::block_number() {
				return true
			}
			false
		}

		fn vote_result(
			voting: &ListenDaoVotes<T::AccountId, T::BlockNumber>,
			room_id: RoomIndex,
		) -> result::Result<(IsEnd, IsPass), DispatchError> {
			let no_votes = voting.nays.len() as MemberCount;
			let yes_votes = voting.ayes.len() as MemberCount;
			let seats = T::ListenHandler::get_room_council(room_id.into())?.len() as MemberCount;

			let approved = yes_votes >= voting.threshold;
			let disapproved =
				seats.saturating_sub(no_votes) < voting.threshold || Self::is_expire(&voting);
			if approved || disapproved {
				if approved {
					Ok((true, true))
				} else {
					Ok((true, false))
				}
			} else {
				Ok((false, false))
			}
		}

		fn do_approve_proposal(
			room_id: RoomIndex,
			seats: MemberCount,
			voting: ListenDaoVotes<T::AccountId, T::BlockNumber>,
			proposal_hash: T::Hash,
			proposal: <T as Config<I>>::Proposal,
		) -> u32 {
			Self::deposit_event(Event::Approved(proposal_hash));

			let _dispatch_weight = proposal.get_dispatch_info().weight;

			// let origin = RoomRawOrigin::Members(voting.threshold, seats).into();
			let origin = RoomRawOrigin::Members(voting.ayes.len() as MemberCount, seats).into();

			let result = proposal.dispatch(origin);
			Self::deposit_event(Event::Executed(
				proposal_hash,
				result.map(|_| ()).map_err(|e| e.error),
			));

			let proposal_count = Self::remove_proposal(room_id, proposal_hash);
			proposal_count
		}

		fn do_disapprove_proposal(room_id: RoomIndex, proposal_hash: T::Hash) -> u32 {
			// disapproved
			Self::deposit_event(Event::Disapproved(proposal_hash));
			Self::remove_proposal(room_id, proposal_hash)
		}

		// Removes a proposal from the pallet, cleaning up votes and the vector of proposals.
		fn remove_proposal(room_id: RoomIndex, proposal_hash: T::Hash) -> u32 {
			// remove proposal and vote
			ProposalOf::<T, I>::remove(room_id, &proposal_hash);
			Voting::<T, I>::remove(room_id, &proposal_hash);
			let num_proposals = Proposals::<T, I>::mutate(room_id, |proposals| {
				proposals.retain(|h| h != &proposal_hash);
				proposals.len() + 1 // calculate weight based on original length
			});
			num_proposals as u32
		}
	}
}

pub struct EnsureMember<AccountId, I: 'static>(sp_std::marker::PhantomData<(AccountId, I)>);
impl<
		O: Into<Result<RoomRawOrigin<AccountId, I>, O>> + From<RoomRawOrigin<AccountId, I>>,
		AccountId: Default,
		I,
	> EnsureOrigin<O> for EnsureMember<AccountId, I>
{
	type Success = AccountId;
	fn try_origin(o: O) -> Result<Self::Success, O> {
		o.into().and_then(|o| match o {
			RoomRawOrigin::Member(_id, who) => Ok(who),
			r => Err(O::from(r)),
		})
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn successful_origin() -> O {
		O::from(RoomRawOrigin::Member(Default::default(), Default::default()))
	}
}

pub struct EnsureMembers<AccountId, I: 'static, const N: u32>(
	sp_std::marker::PhantomData<(AccountId, I)>,
);
impl<
		O: Into<Result<RoomRawOrigin<AccountId, I>, O>> + From<RoomRawOrigin<AccountId, I>>,
		AccountId,
		I,
		const N: u32,
	> EnsureOrigin<O> for EnsureMembers<AccountId, I, N>
{
	type Success = (MemberCount, MemberCount);
	fn try_origin(o: O) -> Result<Self::Success, O> {
		o.into().and_then(|o| match o {
			RoomRawOrigin::Members(n, m) if n >= N => Ok((n, m)),
			r => Err(O::from(r)),
		})
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn successful_origin() -> O {
		O::from(RoomRawOrigin::Members(N, N))
	}
}

pub struct EnsureRoomRoot<T, AccountId, I: 'static>(sp_std::marker::PhantomData<(T, AccountId, I)>);

impl<
		O: Into<Result<RoomRawOrigin<<T as frame_system::Config>::AccountId, I>, O>>
			+ From<RoomRawOrigin<<T as frame_system::Config>::AccountId, I>>,
		AccountId,
		T: Config<I>,
		I: 'static,
	> EnsureOrigin<O> for EnsureRoomRoot<T, AccountId, I>
{
	type Success = ();
	fn try_origin(o: O) -> Result<Self::Success, O> {
		o.into().and_then(|o| match o {
			RoomRawOrigin::Member(room_id, who)
				if T::ListenHandler::get_root(room_id.into()).is_ok() &&
					T::ListenHandler::get_root(room_id.into()).unwrap() == who =>
				Ok(()),
			r => Err(O::from(r)),
		})
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn successful_origin() -> O {
		unimplemented!()
	}
}

pub struct EnsureProportionMoreThan<AccountId, I: 'static, const N: u32, const D: u32>(
	sp_std::marker::PhantomData<(AccountId, I)>,
);
impl<
		O: Into<Result<RoomRawOrigin<AccountId, I>, O>> + From<RoomRawOrigin<AccountId, I>>,
		AccountId,
		I,
		const N: u32,
		const D: u32,
	> EnsureOrigin<O> for EnsureProportionMoreThan<AccountId, I, N, D>
{
	type Success = ();
	fn try_origin(o: O) -> Result<Self::Success, O> {
		o.into().and_then(|o| match o {
			RoomRawOrigin::Members(n, m) if n * D > N * m => Ok(()),
			r => Err(O::from(r)),
		})
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn successful_origin() -> O {
		O::from(RoomRawOrigin::Members(1u32, 0u32))
	}
}

pub struct EnsureProportionAtLeast<AccountId, I: 'static, const N: u32, const D: u32>(
	sp_std::marker::PhantomData<(AccountId, I)>,
);
impl<
		O: Into<Result<RoomRawOrigin<AccountId, I>, O>> + From<RoomRawOrigin<AccountId, I>>,
		AccountId,
		I,
		const N: u32,
		const D: u32,
	> EnsureOrigin<O> for EnsureProportionAtLeast<AccountId, I, N, D>
{
	type Success = ();
	fn try_origin(o: O) -> Result<Self::Success, O> {
		o.into().and_then(|o| match o {
			RoomRawOrigin::Members(n, m) if n * D >= N * m => Ok(()),
			r => Err(O::from(r)),
		})
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn successful_origin() -> O {
		O::from(RoomRawOrigin::Members(0u32, 0u32))
	}
}

impl<T: Config<I>, I: 'static> CollectiveHandler<u64, T::BlockNumber, DispatchError>
	for Pallet<T, I>
{
	fn remove_room_collective_info(room_id: u64) -> result::Result<(), DispatchError> {
		<ProposalCount<T, I>>::remove(room_id);
		<Voting<T, I>>::remove_prefix(room_id, None);
		<ProposalOf<T, I>>::remove_prefix(room_id, None);
		<Proposals<T, I>>::remove(room_id);
		Ok(())
	}

	fn get_motion_duration(_room_id: u64) -> T::BlockNumber {
		T::MotionDuration::get()
	}
}
