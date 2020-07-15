#![cfg_attr(not(feature = "std"), no_std)]

extern crate frame_system as system;
extern crate pallet_collective as collective;
extern crate pallet_timestamp as timestamp;

use crate::constants::{congress::*, referendum::*};
use codec::{Decode, Encode};
use frame_support::traits::{BalanceStatus, Currency, ReservableCurrency};
use frame_support::{
    debug, decl_error, decl_event, decl_module, decl_storage, dispatch::DispatchResult, ensure,
    StorageMap, StorageValue,
};
use sp_runtime::traits::SaturatedConversion;
use sp_std::convert::TryInto;
use sp_std::vec::Vec;
use system::ensure_signed;

pub type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;

pub const ZERO_GOALS: (u64, u64) = (0, 0);

pub trait Trait: system::Trait + timestamp::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    type Currency: ReservableCurrency<Self::AccountId>;
    // type Congress: collective::Trait<collective::Instance1>;
}

#[derive(Encode, Decode, Clone, Default, Debug, PartialEq, Eq)]
pub struct TokenInfo<AccountId, Balance> {
    pub proposer: AccountId,
    pub official_website_url: Vec<u8>,
    pub token_icon_url: Vec<u8>,
    pub token_symbol: Vec<u8>,
    pub total_issuance: Balance,
    pub total_circulation: Balance,
    pub current_board: BoardType,
}

#[derive(Encode, Decode, Clone, Default, Debug, PartialEq, Eq)]
pub struct Proposal<AccountId, Balance> {
    pub proposer: AccountId,
    pub proposal_type: ProposalType,
    pub official_website_url: Vec<u8>,
    pub token_icon_url: Vec<u8>,
    pub token_symbol: Vec<u8>,
    pub total_issuance: Balance,
    pub total_circulation: Balance,
    pub current_board: BoardType,
    pub target_board: BoardType,
    /// The state of proposal.
    pub state: ProposalState,
    /// The reviewing number of (supporters, opponents)
    /// Number = VoteAge * TokenAmount
    pub review_goals: (u64, u64),
    /// The voting number of (supporters, opponents)
    /// Number = VoteAge * TokenAmount
    pub vote_goals: (u64, u64),
    /// When the state of proposal changes, update this timestamp.
    pub timestamp: u64,
}

#[derive(Encode, Decode, Clone, Debug, PartialEq, Eq)]
pub enum BoardType {
    Main,
    Growth,
    Off,
}

impl Default for BoardType {
    fn default() -> Self {
        BoardType::Off
    }
}

#[derive(Encode, Decode, Clone, Debug, PartialEq, Eq)]
pub enum ProposalType {
    List,
    Delist,
    Rise,
    Fall,
}

impl Default for ProposalType {
    fn default() -> Self {
        ProposalType::List
    }
}

#[derive(Encode, Decode, Clone, Debug, PartialEq, Eq)]
pub enum ProposalState {
    Pending,
    Reviewing,
    Voting,
    Approved,
    Rejected,
}

impl Default for ProposalState {
    fn default() -> Self {
        ProposalState::Pending
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Ibo {
        pub Proposals get(fn proposal): map hasher(twox_64_concat) u64 => Option<Proposal<T::AccountId, BalanceOf<T>>>;

        pub Tokens get(fn token): map hasher(twox_64_concat) Vec<u8> => Option<TokenInfo<T::AccountId, BalanceOf<T>>>;

        pub Reviewing get(fn reviewing): map hasher(twox_64_concat) u64 => Vec<T::AccountId>;

        pub Voting get(fn voting): map hasher(twox_64_concat) u64 => Vec<T::AccountId>;

        pub IdGenerator get(fn id_generator): u64 = 0;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        #[weight = 200]
        fn create_list_proposal(
            origin,
            official_website_url: Vec<u8>,
            token_icon_url: Vec<u8>,
            token_symbol: Vec<u8>,
            total_issuance: BalanceOf<T>,
            total_circulation: BalanceOf<T>,
            target_board: BoardType
        ) -> DispatchResult {
            let proposer = ensure_signed(origin)?;
            ensure!(!Tokens::<T>::contains_key(&token_symbol), Error::<T>::TokenExists);
            let now = Self::get_now_ts();
            let id = Self::generate_id();
            let new_proposal = Proposal {
                proposer,
                proposal_type: ProposalType::List,
                official_website_url,
                token_icon_url,
                token_symbol,
                total_issuance,
                total_circulation,
                current_board: BoardType::Off,
                target_board,
                state: ProposalState::Pending,
                review_goals: ZERO_GOALS,
                vote_goals: ZERO_GOALS,
                timestamp: now,
            };
            Proposals::<T>::insert(id, new_proposal);
            Self::deposit_event(RawEvent::CreateProposal(id));
            Ok(())
        }

        #[weight = 100]
        fn update_list_proposal(
            origin,
            id: u64,
            official_website_url: Vec<u8>,
            token_icon_url: Vec<u8>,
            token_symbol: Vec<u8>,
            total_issuance: BalanceOf<T>,
            total_circulation: BalanceOf<T>,
            target_board: BoardType
        ) -> DispatchResult {
            let proposer = ensure_signed(origin)?;
            let now = Self::get_now_ts();
            let new_proposal = Proposal {
                proposer,
                proposal_type: ProposalType::List,
                official_website_url,
                token_icon_url,
                token_symbol,
                total_issuance,
                total_circulation,
                current_board: BoardType::Off,
                target_board,
                state: ProposalState::Pending,
                review_goals: ZERO_GOALS,
                vote_goals: ZERO_GOALS,
                timestamp: now,
            };
            Self::update_proposal(id, new_proposal)
        }

        #[weight = 100]
        fn delete_list_proposal(origin, id: u64) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            Self::remove_proposal(id)
        }

        #[weight = 200]
        fn create_delist_proposal(origin, token_symbol: Vec<u8>) -> DispatchResult {
            let proposer = ensure_signed(origin)?;
            let token_info = Self::token(&token_symbol).ok_or(Error::<T>::TokenNotFound)?;
            let now = Self::get_now_ts();
            let id = Self::generate_id();
            let new_proposal = Self::clone_from_token_info(proposer, ProposalType::Delist, BoardType::Off, now, token_info);
            Proposals::<T>::insert(id, new_proposal);
            Self::deposit_event(RawEvent::CreateProposal(id));
            Ok(())
        }

        #[weight = 100]
        fn delete_delist_proposal(origin, id: u64) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            Self::remove_proposal(id)
        }

        #[weight = 100]
        fn create_rise_proposal(origin, token_symbol: Vec<u8>) -> DispatchResult {
            let proposer = ensure_signed(origin)?;
            let token_info = Self::token(&token_symbol).ok_or(Error::<T>::TokenNotFound)?;
            let now = Self::get_now_ts();
            let new_proposal = Self::clone_from_token_info(proposer, ProposalType::Rise, BoardType::Main, now, token_info);
            let id = Self::generate_id();
            Proposals::<T>::insert(id, new_proposal);
            Self::deposit_event(RawEvent::CreateProposal(id));
            Ok(())
        }

        #[weight = 50]
        fn delete_rise_proposal(origin, id: u64) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            Self::remove_proposal(id)
        }

        #[weight = 100]
        fn create_fall_proposal(origin, token_symbol: Vec<u8>) -> DispatchResult {
            let proposer = ensure_signed(origin)?;
            let token_info = Self::token(&token_symbol).ok_or(Error::<T>::TokenNotFound)?;
            let now = Self::get_now_ts();
            let id = Self::generate_id();
            let new_proposal = Self::clone_from_token_info(proposer, ProposalType::Fall, BoardType::Growth, now, token_info);
            Proposals::<T>::insert(id, new_proposal);
            Self::deposit_event(RawEvent::CreateProposal(id));
            Ok(())
        }

        #[weight = 50]
        fn delete_fall_proposal(origin, id: u64) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            Self::remove_proposal(id)
        }

        #[weight = 10]
        fn review_proposal(origin, id: u64, stand: bool) -> DispatchResult {
            let member = ensure_signed(origin)?;
            // ensure!(<collective::Module<T>>::is_member(&member), Error::<T>::NotInCollective);
            let proposal = Self::proposal(id).ok_or(Error::<T>::ProposalNotFound)?;
            ensure!(
                proposal.state == ProposalState::Reviewing,
                Error::<T>::ProposalCannotBeReviewed
            );
            Reviewing::<T>::try_mutate(id, |reviewers| -> DispatchResult {
                ensure!(!(&*reviewers).contains(&member), Error::<T>::AlreadyReview);
                reviewers.push(member);
                Ok(())
            })?;


            Ok(())
        }

        #[weight = 10]
        fn vote_proposal(origin, id: u64, stand: bool) -> DispatchResult {
            let user = ensure_signed(origin)?;
            let proposal = Self::proposal(id).ok_or(Error::<T>::ProposalNotFound)?;
            ensure!(
                proposal.state == ProposalState::Voting,
                Error::<T>::ProposalCannotBeVoted
            );
            Voting::<T>::try_mutate(id, |voters| -> DispatchResult {
                ensure!(!(&*voters).contains(&user), Error::<T>::AlreadyVote);
                voters.push(user);
                // todo: add goals
                Ok(())
            })?;

            Ok(())
        }

        fn on_finalize() {
            let now = Self::get_now_ts();
        }

    }
}

impl<T: Trait> Module<T> {
    fn get_now_ts() -> u64 {
        let now = <timestamp::Module<T>>::get();
        <T::Moment as TryInto<u64>>::try_into(now).ok().unwrap()
    }

    fn update_proposal(
        id: u64,
        new_proposal: Proposal<T::AccountId, BalanceOf<T>>,
    ) -> DispatchResult {
        let proposal: Proposal<T::AccountId, BalanceOf<T>> =
            Self::proposal(id).ok_or(Error::<T>::ProposalNotFound)?;
        ensure!(
            proposal.state == ProposalState::Pending,
            Error::<T>::ProposalCannotBeModified
        );
        Proposals::<T>::insert(id, new_proposal);
        Ok(())
    }

    fn remove_proposal(id: u64) -> DispatchResult {
        let proposal: Proposal<T::AccountId, BalanceOf<T>> =
            Self::proposal(id).ok_or(Error::<T>::ProposalNotFound)?;
        ensure!(
            proposal.state == ProposalState::Pending,
            Error::<T>::ProposalCannotBeModified
        );
        Proposals::<T>::remove(id);
        Ok(())
    }

    fn clone_from_token_info(
        proposer: T::AccountId,
        proposal_type: ProposalType,
        target_board: BoardType,
        timestamp: u64,
        token_info: TokenInfo<T::AccountId, BalanceOf<T>>,
    ) -> Proposal<T::AccountId, BalanceOf<T>> {
        Proposal {
            proposer,
            proposal_type,
            official_website_url: token_info.official_website_url,
            token_icon_url: token_info.token_icon_url,
            token_symbol: token_info.token_symbol,
            total_issuance: token_info.total_issuance,
            total_circulation: token_info.total_circulation,
            current_board: token_info.current_board,
            target_board,
            state: ProposalState::Pending,
            review_goals: ZERO_GOALS,
            vote_goals: ZERO_GOALS,
            timestamp,
        }
    }

    fn generate_id() -> u64 {
        let mut id = 0;
        IdGenerator::mutate(|i| {
            id = *i;
            *i = *i + 1;
        });
        id
    }
}

decl_event! {
    pub enum Event<T>
        where
        AccountId = <T as system::Trait>::AccountId
        {
            Vote(AccountId),

            CreateProposal(u64),
        }
}

decl_error! {
    /// Error for the ipse module.
    pub enum Error for Module<T: Trait> {
        /// There is the same name token.
        TokenExists,
        /// There is no token named it.
        TokenNotFound,
        /// Proposal not found.
        ProposalNotFound,
        /// The proposal now cannot be reviewed.
        ProposalCannotBeReviewed,
        /// The proposal now cannot be voted.
        ProposalCannotBeVoted,
        /// There is no proposal can be modified.
        ProposalCannotBeModified,
        /// You already review and Cannot review again.
        AlreadyReview,
        /// You already vote and Cannot vote again.
        AlreadyVote,
        /// You are not a member of collective.
        NotInCollective,
    }
}
