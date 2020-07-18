#![cfg_attr(not(feature = "std"), no_std)]

extern crate frame_system as system;
extern crate pallet_collective as collective;
extern crate pallet_timestamp as timestamp;
extern crate pallet_treasury as treasury;

use self::treasury::AccountGetter;
use crate::constants::{congress::*, referendum::*};
use codec::{Decode, Encode};
use collective::Contain;
use frame_support::traits::{Currency, ReservableCurrency};
use frame_support::{
    debug, decl_error, decl_event, decl_module, decl_storage, dispatch::DispatchResult, ensure,
    storage::IterableStorageMap, StorageMap, StorageValue,
};
use sp_runtime::traits::SaturatedConversion;
use sp_std::convert::TryInto;
use sp_std::vec::Vec;
use system::{ensure_root, ensure_signed};

pub type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;
pub type ProposalId = u32;

pub const ZERO_GOALS: (u64, u64) = (0, 0);
pub const TOTAL_REWARDS: u64 = 100_000;

pub trait Trait: system::Trait + timestamp::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    type Currency: ReservableCurrency<Self::AccountId> + Currency<Self::AccountId>;
    type CouncilMembers: collective::Contain<Self::AccountId>;
    type Treasury: treasury::AccountGetter<Self::AccountId>;
}

#[derive(Encode, Decode, Clone, Default, Debug, PartialEq, Eq)]
pub struct TokenInfo<Balance> {
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
    pub rewards_remainder: Balance,
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
    ApprovedClosed,
    RejectedClosed,
}

impl Default for ProposalState {
    fn default() -> Self {
        ProposalState::Pending
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Ibo {
        pub Proposals get(fn proposal): map hasher(twox_64_concat) ProposalId => Option<Proposal<T::AccountId, BalanceOf<T>>>;

        pub Tokens get(fn token): map hasher(twox_64_concat) Vec<u8> => Option<TokenInfo<BalanceOf<T>>>;

        pub Reviewing get(fn reviewing): map hasher(twox_64_concat) ProposalId => Vec<T::AccountId>;

        pub Voting get(fn voting): map hasher(twox_64_concat) ProposalId => Vec<T::AccountId>;
        // (BalanceOf<T>, usize) = (staking, AGE_DAY index)
        pub Staking get(fn staking): map hasher(twox_64_concat) T::AccountId => Option<(BalanceOf<T>, u8)>;

        pub IdGenerator get(fn id_generator): ProposalId = 0;
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
                rewards_remainder: TOTAL_REWARDS.saturated_into::<BalanceOf<T>>(),
                timestamp: now,
            };
            Proposals::<T>::insert(id, new_proposal);
            Self::deposit_event(RawEvent::CreateProposal(id));
            Ok(())
        }

        #[weight = 100]
        fn update_list_proposal(
            origin,
            id: ProposalId,
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
                rewards_remainder: TOTAL_REWARDS.saturated_into::<BalanceOf<T>>(),
                timestamp: now,
            };
            Self::update_proposal(id, new_proposal)
        }

        #[weight = 100]
        fn delete_list_proposal(origin, id: ProposalId) -> DispatchResult {
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
        fn delete_delist_proposal(origin, id: ProposalId) -> DispatchResult {
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
        fn delete_rise_proposal(origin, id: ProposalId) -> DispatchResult {
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
        fn delete_fall_proposal(origin, id: ProposalId) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            Self::remove_proposal(id)
        }

        #[weight = 10]
        fn review_proposal(origin, id: ProposalId, stand: bool) -> DispatchResult {
            let member = ensure_signed(origin)?;
            ensure!(T::CouncilMembers::contains(&member), Error::<T>::NotInCollective);
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
            Proposals::<T>::mutate(id, |p| {
                if stand {
                    p.as_mut().unwrap().review_goals.0 += 1;
                } else {
                    p.as_mut().unwrap().review_goals.1 += 1;
                }
            });
            Ok(())
        }

        #[weight = 10]
        fn vote_proposal(origin, id: ProposalId, stand: bool) -> DispatchResult {
            let user = ensure_signed(origin)?;
            let proposal = Self::proposal(id).ok_or(Error::<T>::ProposalNotFound)?;
            ensure!(
                proposal.state == ProposalState::Voting,
                Error::<T>::ProposalCannotBeVoted
            );
            Voting::<T>::try_mutate(id, |voters| -> DispatchResult {
                ensure!(!(&*voters).contains(&user), Error::<T>::AlreadyVote);
                voters.push(user.clone());
                Ok(())
            })?;
            let stake = Self::staking(&user).ok_or(Error::<T>::NoneStaking)?;
            let goals = Self::get_goals_from_staking(&stake);
            Proposals::<T>::mutate(id, |p| {
                if stand {
                    p.as_mut().unwrap().vote_goals.0 += goals;
                } else {
                    p.as_mut().unwrap().vote_goals.1 += goals;
                }
            });
            Ok(())
        }

        #[weight = 10]
        fn receive_rewards(origin, id: ProposalId) -> DispatchResult {
            let user = ensure_signed(origin)?;
            ensure!(Self::voting(id).contains(&user), Error::<T>::NoVote);
            let proposal = Self::proposal(id).ok_or(Error::<T>::ProposalNotFound)?;
            let is_state_for_rewards =
                proposal.state == ProposalState::Approved || proposal.state == ProposalState::Rejected;
            ensure!(
                is_state_for_rewards,
                Error::<T>::StateNotForRewards
            );
            let stake = Self::staking(&user).ok_or(Error::<T>::NoneStaking)?;
            let goals = Self::get_goals_from_staking(&stake).saturated_into::<BalanceOf<T>>();
            let total_goals = (proposal.vote_goals.0 + proposal.vote_goals.1)
                .saturated_into::<BalanceOf<T>>();
            let reward = TOTAL_REWARDS.saturated_into::<BalanceOf<T>>() * goals / total_goals;
            Self::deposit_into_existing(&user, reward)?;
            Proposals::<T>::mutate(id, |p| p.as_mut().unwrap().rewards_remainder -= reward);
            Ok(())
        }

        #[weight = 10]
        fn stake(origin, amount: BalanceOf<T>, age_idx: u8) -> DispatchResult {
            let user = ensure_signed(origin)?;
            ensure!(!Staking::<T>::contains_key(&user), Error::<T>::AlreadyStaked);
            Staking::<T>::insert(user, (amount, age_idx));
            Ok(())
        }

        #[weight = 0]
        fn burn(origin, burn_amount: BalanceOf<T>) {
            ensure_root(origin)?;
            T::Currency::burn(burn_amount);
        }

        fn on_finalize() {
            let now = Self::get_now_ts();
            let mut iter = Proposals::<T>::iter();
            while let Some((id, proposal)) = iter.next() {
                Self::deal_proposal(id, proposal, now);
            }
        }

    }
}

impl<T: Trait> Module<T> {
    fn deposit_into_existing(account: &T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
        T::Currency::deposit_into_existing(account, amount)?;
        T::Currency::issue(amount);
        Ok(())
    }

    fn deal_proposal(id: ProposalId, proposal: Proposal<T::AccountId, BalanceOf<T>>, now: u64) {
        let duration = now - proposal.timestamp;
        match proposal.state {
            ProposalState::Pending => Self::check_proposal_pending(id, proposal, duration, now),
            ProposalState::Reviewing => Self::check_proposal_reviewed(id, proposal, duration, now),
            ProposalState::Voting => Self::check_proposal_voted(id, proposal, duration, now),
            ProposalState::Approved => Self::check_proposal_closed(id, proposal, duration, now),
            ProposalState::Rejected => Self::check_proposal_closed(id, proposal, duration, now),
            _ => {}
        }
    }

    fn check_proposal_pending(
        id: ProposalId,
        mut proposal: Proposal<T::AccountId, BalanceOf<T>>,
        duration: u64,
        now: u64,
    ) {
        if duration > ALLOW_MODIFY_DURATION {
            proposal.state = ProposalState::Reviewing;
            proposal.timestamp = now;
            Proposals::<T>::insert(id, proposal);
        }
    }

    fn check_proposal_reviewed(
        id: ProposalId,
        mut proposal: Proposal<T::AccountId, BalanceOf<T>>,
        duration: u64,
        now: u64,
    ) {
        if duration > REVIEW_DURATION {
            let supporters_goals = proposal.review_goals.0;
            let opponents_goals = proposal.review_goals.1;
            if proposal.proposal_type == ProposalType::Rise
                || proposal.proposal_type == ProposalType::Fall
            {
                proposal.state = if supporters_goals >= 2 * opponents_goals {
                    Tokens::<T>::insert(
                        &proposal.token_symbol,
                        Self::clone_from_proposal(proposal.clone()),
                    );
                    ProposalState::Approved
                } else {
                    ProposalState::Rejected
                };
            }

            if proposal.proposal_type == ProposalType::Delist {
                proposal.state = if supporters_goals >= opponents_goals {
                    ProposalState::Voting
                } else {
                    ProposalState::Rejected
                };
            }

            if proposal.proposal_type == ProposalType::List {
                proposal.state = if supporters_goals >= 2 * opponents_goals {
                    ProposalState::Voting
                } else {
                    ProposalState::Rejected
                };
            }
            proposal.timestamp = now;

            Proposals::<T>::insert(id, proposal);
        }
    }

    fn check_proposal_voted(
        id: ProposalId,
        mut proposal: Proposal<T::AccountId, BalanceOf<T>>,
        duration: u64,
        now: u64,
    ) {
        if duration > VOTE_DURATION {
            let supporters_goals = proposal.vote_goals.0;
            let opponents_goals = proposal.vote_goals.1;
            if proposal.proposal_type == ProposalType::List {
                proposal.state = if supporters_goals >= 2 * opponents_goals {
                    Tokens::<T>::insert(
                        &proposal.token_symbol,
                        Self::clone_from_proposal(proposal.clone()),
                    );
                    ProposalState::Approved
                } else {
                    ProposalState::Rejected
                };
            };

            if proposal.proposal_type == ProposalType::Delist {
                proposal.state = if supporters_goals >= opponents_goals {
                    Tokens::<T>::remove(&proposal.token_symbol);
                    ProposalState::Approved
                } else {
                    ProposalState::Rejected
                };
            }

            proposal.timestamp = now;
            Proposals::<T>::insert(id, proposal);
        }
    }

    fn check_proposal_closed(
        id: ProposalId,
        mut proposal: Proposal<T::AccountId, BalanceOf<T>>,
        duration: u64,
        now: u64,
    ) {
        if duration > RECEIVE_REWARDS_DURATION {
            if proposal.state == ProposalState::Approved {
                proposal.state = ProposalState::ApprovedClosed
            }
            if proposal.state == ProposalState::Rejected {
                proposal.state = ProposalState::RejectedClosed
            }
            proposal.timestamp = now;
            let treasury_account = T::Treasury::get_account_id();
            Self::deposit_into_existing(&treasury_account, proposal.rewards_remainder);
            Proposals::<T>::insert(id, proposal);
        }
    }

    fn get_goals_from_staking(stake: &(BalanceOf<T>, u8)) -> u64 {
        let balance = stake.0;
        let balance = balance.saturated_into::<u64>();
        let age_idx = stake.1;
        let vote_age = AGE_DAY.get(age_idx as usize).unwrap().0;
        balance * vote_age
    }

    fn get_now_ts() -> u64 {
        let now = <timestamp::Module<T>>::get();
        <T::Moment as TryInto<u64>>::try_into(now).ok().unwrap()
    }

    fn update_proposal(
        id: ProposalId,
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

    fn remove_proposal(id: ProposalId) -> DispatchResult {
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
        token_info: TokenInfo<BalanceOf<T>>,
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
            rewards_remainder: TOTAL_REWARDS.saturated_into::<BalanceOf<T>>(),
            timestamp,
        }
    }

    fn clone_from_proposal(
        proposal: Proposal<T::AccountId, BalanceOf<T>>,
    ) -> TokenInfo<BalanceOf<T>> {
        TokenInfo {
            official_website_url: proposal.official_website_url,
            token_icon_url: proposal.token_icon_url,
            token_symbol: proposal.token_symbol,
            total_issuance: proposal.total_issuance,
            total_circulation: proposal.total_circulation,
            current_board: proposal.target_board,
        }
    }

    fn generate_id() -> ProposalId {
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

            CreateProposal(ProposalId),
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
        /// You cannot receive rewards now.
        CannotReceiveRewards,
        /// None staking for voting.
        NoneStaking,
        /// Invalid vote age index.
        InvalidVoteAge,
        /// You have already staked.
        AlreadyStaked,
        /// You have not voted.
        NoVote,
        /// You cannot receive rewards now
        /// because proposal state is not allowed.
        StateNotForRewards,
    }
}
