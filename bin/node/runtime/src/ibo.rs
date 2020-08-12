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

pub const ZERO_GOALS_U64: (u64, u64) = (0, 0);
pub const ZERO_GOALS_U128: (u128, u128) = (0, 0);
pub const TOTAL_REWARDS: u64 = 100_000;
pub const MAX_SUPPLY: u64 = 1_000_000_000;

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
    pub token_name: Vec<u8>,
    pub token_symbol: Vec<u8>,
    pub max_supply: Balance,
    pub circulating_supply: Balance,
    pub current_market: MarketType,
}

#[derive(Encode, Decode, Clone, Default, Debug, PartialEq, Eq)]
pub struct Proposal<AccountId, Balance> {
    pub id: ProposalId,
    pub proposer: AccountId,
    pub proposal_type: ProposalType,
    pub official_website_url: Vec<u8>,
    pub token_icon_url: Vec<u8>,
    pub token_name: Vec<u8>,
    pub token_symbol: Vec<u8>,
    pub max_supply: Balance,
    pub circulating_supply: Balance,
    pub current_market: MarketType,
    pub target_market: MarketType,
    /// The state of proposal.
    pub state: ProposalState,
    /// The reviewing number of (supporters, opponents)
    /// Number = VoteAge * TokenAmount
    pub review_goals: (u64, u64),
    /// The voting number of (supporters, opponents)
    /// Number = VoteAge * TokenAmount
    pub vote_goals: (u128, u128),
    /// When the state of proposal changes, update this timestamp.
    pub rewards_remainder: Balance,
    pub timestamp: u64,
}

#[derive(Encode, Decode, Clone, Debug, PartialEq, Eq)]
pub enum MarketType {
    Main,
    Growth,
    Off,
}

impl Default for MarketType {
    fn default() -> Self {
        MarketType::Off
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

#[derive(Encode, Decode, Clone, Debug, PartialEq, Eq)]
pub struct StakingInfo<Balance> {
    pub proposal_id: ProposalId,
    pub staking_amount: Balance,
    pub age_idx: u8,
    pub wheather_received_reward: bool,
    pub timestamp: u64,
}

decl_storage! {
    trait Store for Module<T: Trait> as Ibo {
        pub Proposals get(fn proposal): map hasher(twox_64_concat) ProposalId => Option<Proposal<T::AccountId, BalanceOf<T>>>;

        pub VotingProposal get(fn voting_proposals): ProposalId;

        pub Tokens get(fn token): map hasher(twox_64_concat) Vec<u8> => Option<TokenInfo<BalanceOf<T>>>;

        pub Reviewers get(fn reviewers): map hasher(twox_64_concat) ProposalId => Vec<T::AccountId>;

        pub Voters get(fn voters): map hasher(twox_64_concat) ProposalId => Vec<T::AccountId>;

        pub Staking get(fn staking): map hasher(twox_64_concat) T::AccountId => Vec<StakingInfo<BalanceOf<T>>>;

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
            token_name: Vec<u8>,
            token_symbol: Vec<u8>,
            max_supply: BalanceOf<T>,
            circulating_supply: BalanceOf<T>,
            target_market: MarketType
        ) -> DispatchResult {
            let proposer = ensure_signed(origin)?;
            ensure!(
                MAX_SUPPLY - T::Currency::total_issuance().saturated_into::<u64>() >= TOTAL_REWARDS,
                Error::<T>::InsufficientIssuance
            );
            ensure!(!Tokens::<T>::contains_key(&token_name), Error::<T>::TokenExists);
            let now = Self::get_now_ts();
            let id = Self::generate_id();
            let new_proposal = Proposal {
                id,
                proposer,
                proposal_type: ProposalType::List,
                official_website_url,
                token_icon_url,
                token_name,
                token_symbol,
                max_supply,
                circulating_supply,
                current_market: MarketType::Off,
                target_market,
                state: ProposalState::Pending,
                review_goals: ZERO_GOALS_U64,
                vote_goals: ZERO_GOALS_U128,
                rewards_remainder: TOTAL_REWARDS.saturated_into::<BalanceOf<T>>(),
                timestamp: now,
            };
            Proposals::<T>::insert(id, new_proposal.clone());
            Self::deposit_event(RawEvent::ProposalChanged(CREATE, new_proposal));
            Ok(())
        }

        #[weight = 100]
        fn update_list_proposal(
            origin,
            id: ProposalId,
            official_website_url: Vec<u8>,
            token_icon_url: Vec<u8>,
            token_name: Vec<u8>,
            token_symbol: Vec<u8>,
            max_supply: BalanceOf<T>,
            circulating_supply: BalanceOf<T>,
            target_market: MarketType
        ) -> DispatchResult {
            let proposer = ensure_signed(origin)?;
            ensure!(!Tokens::<T>::contains_key(&token_name), Error::<T>::TokenExists);
            let now = Self::get_now_ts();
            let new_proposal = Proposal {
                id,
                proposer: proposer.clone(),
                proposal_type: ProposalType::List,
                official_website_url,
                token_icon_url,
                token_name,
                token_symbol,
                max_supply,
                circulating_supply,
                current_market: MarketType::Off,
                target_market,
                state: ProposalState::Pending,
                review_goals: ZERO_GOALS_U64,
                vote_goals: ZERO_GOALS_U128,
                rewards_remainder: TOTAL_REWARDS.saturated_into::<BalanceOf<T>>(),
                timestamp: now,
            };
            Self::update_proposal(id, proposer, new_proposal)
        }

        #[weight = 100]
        fn delete_list_proposal(origin, id: ProposalId) -> DispatchResult {
            let proposer = ensure_signed(origin)?;
            Self::remove_proposal(id, proposer)
        }

        #[weight = 200]
        fn create_delist_proposal(origin, token_name: Vec<u8>) -> DispatchResult {
            let proposer = ensure_signed(origin)?;
            ensure!(
                MAX_SUPPLY - T::Currency::total_issuance().saturated_into::<u64>() >= TOTAL_REWARDS,
                Error::<T>::InsufficientIssuance
            );
            let token_info = Self::token(&token_name).ok_or(Error::<T>::TokenNotFound)?;
            let now = Self::get_now_ts();
            let id = Self::generate_id();
            let new_proposal = Self::clone_from_token_info(
                id,
                proposer,
                ProposalType::Delist,
                MarketType::Off,
                TOTAL_REWARDS.saturated_into::<BalanceOf<T>>(),
                now,
                token_info
            );
            Proposals::<T>::insert(id, new_proposal.clone());
            Self::deposit_event(RawEvent::ProposalChanged(CREATE, new_proposal));
            Ok(())
        }

        #[weight = 100]
        fn delete_delist_proposal(origin, id: ProposalId) -> DispatchResult {
            let proposer = ensure_signed(origin)?;
            Self::remove_proposal(id, proposer)
        }

        #[weight = 100]
        fn create_rise_proposal(origin, token_name: Vec<u8>) -> DispatchResult {
            let proposer = ensure_signed(origin)?;
            let token_info = Self::token(&token_name).ok_or(Error::<T>::TokenNotFound)?;
            let now = Self::get_now_ts();
            let id = Self::generate_id();
            let new_proposal = Self::clone_from_token_info(
                id,
                proposer,
                ProposalType::Rise,
                MarketType::Main,
                0.saturated_into::<BalanceOf<T>>(),
                now,
                token_info
            );
            Proposals::<T>::insert(id, new_proposal.clone());
            Self::deposit_event(RawEvent::ProposalChanged(CREATE, new_proposal));
            Ok(())
        }

        #[weight = 50]
        fn delete_rise_proposal(origin, id: ProposalId) -> DispatchResult {
            let proposer = ensure_signed(origin)?;
            Self::remove_proposal(id, proposer)
        }

        #[weight = 100]
        fn create_fall_proposal(origin, token_name: Vec<u8>) -> DispatchResult {
            let proposer = ensure_signed(origin)?;
            let token_info = Self::token(&token_name).ok_or(Error::<T>::TokenNotFound)?;
            let now = Self::get_now_ts();
            let id = Self::generate_id();
            let new_proposal = Self::clone_from_token_info(
                id,
                proposer,
                ProposalType::Fall,
                MarketType::Growth,
                0.saturated_into::<BalanceOf<T>>(),
                now,
                token_info
            );
            Proposals::<T>::insert(id, new_proposal.clone());
            Self::deposit_event(RawEvent::ProposalChanged(CREATE, new_proposal));
            Ok(())
        }

        #[weight = 50]
        fn delete_fall_proposal(origin, id: ProposalId) -> DispatchResult {
            let proposer = ensure_signed(origin)?;
            Self::remove_proposal(id, proposer)
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
            Reviewers::<T>::try_mutate(id, |reviewers| -> DispatchResult {
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
            Self::deposit_event(RawEvent::ProposalChanged(UPDATE, Self::proposal(id).unwrap()));
            Ok(())
        }

        #[weight = 10]
        fn vote_proposal(origin, id: ProposalId, amount: BalanceOf<T>, age_idx: u8, stand: bool) -> DispatchResult {
            let user = ensure_signed(origin)?;
            let proposal = Self::proposal(id).ok_or(Error::<T>::ProposalNotFound)?;
            ensure!(
                proposal.state == ProposalState::Voting,
                Error::<T>::ProposalCannotBeVoted
            );

            Voters::<T>::try_mutate(id, |voters| -> DispatchResult {
                ensure!(!(&*voters).contains(&user), Error::<T>::AlreadyVote);
                T::Currency::reserve(&user, amount)?;
                voters.push(user.clone());
                Ok(())
            })?;

            let goals = Self::get_goals_from_staking(amount, age_idx);
            Proposals::<T>::mutate(id, |p| {
                if stand {
                    p.as_mut().unwrap().vote_goals.0 += goals;
                } else {
                    p.as_mut().unwrap().vote_goals.1 += goals;
                }
            });
            let now = Self::get_now_ts();
            Staking::<T>::mutate(&user, |infos| infos.push( StakingInfo {
                proposal_id: id,
                staking_amount: amount,
                age_idx,
                wheather_received_reward: false,
                timestamp: now,
            }));
            debug::info!("vote support goals = {}, vote opponents goals = {}",Self::proposal(id).unwrap().vote_goals.0, Self::proposal(id).unwrap().vote_goals.1);
            Self::deposit_event(RawEvent::ProposalChanged(UPDATE, Self::proposal(id).unwrap()));
            Ok(())
        }

        #[weight = 10]
        fn receive_rewards(origin, id: ProposalId) -> DispatchResult {
            let user = ensure_signed(origin)?;
            ensure!(Self::voters(id).contains(&user), Error::<T>::NoVote);
            let proposal = Self::proposal(id).ok_or(Error::<T>::ProposalNotFound)?;
            let is_state_for_rewards =
                proposal.state == ProposalState::Approved || proposal.state == ProposalState::Rejected;
            ensure!(
                is_state_for_rewards,
                Error::<T>::StateNotForRewards
            );
            let stake_info = Self::get_staking_info(&user, id).ok_or(Error::<T>::NoneStaking)?;
            let goals = Self::get_goals_from_staking(stake_info.staking_amount, stake_info.age_idx).saturated_into::<BalanceOf<T>>();
            let total_goals = (proposal.vote_goals.0 + proposal.vote_goals.1)
                .saturated_into::<BalanceOf<T>>();
            let reward = TOTAL_REWARDS.saturated_into::<BalanceOf<T>>() * goals / total_goals;
            Self::deposit_into_existing(&user, reward)?;
            Proposals::<T>::mutate(id, |p| p.as_mut().unwrap().rewards_remainder -= reward);
            Staking::<T>::mutate(&user, |infos| {
                let mut iter = infos.iter_mut();
                while let Some(info) = iter.next() {
                    if info.proposal_id == id {
                        info.wheather_received_reward = true;
                    }
                }
            });
            Self::deposit_event(RawEvent::ProposalChanged(UPDATE, Self::proposal(id).unwrap()));
            Ok(())
        }

        #[weight = 100]
        fn unstake(origin, id: ProposalId) -> DispatchResult {
            let user = ensure_signed(origin)?;
            let stake_info = Self::get_staking_info(&user, id).ok_or(Error::<T>::NoneStaking)?;
            let stake_days = AGE_DAY.get(stake_info.age_idx as usize).unwrap().1;
            let duration = Self::get_now_ts() - stake_info.timestamp;
            ensure!(duration >= stake_days, Error::<T>::StillInStaking);
            T::Currency::unreserve(&user, stake_info.staking_amount);
            Staking::<T>::mutate(user, |infos| infos.remove_item(&stake_info));
            Ok(())
        }

        #[weight = 10]
        fn burn(origin, burn_amount: BalanceOf<T>) {
            let user = ensure_signed(origin)?;
            T::Currency::slash(&user, burn_amount);
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
    fn get_staking_info(
        user: &T::AccountId,
        id: ProposalId,
    ) -> Option<StakingInfo<BalanceOf<T>>> {
        let stakes = Self::staking(user);
        let mut iter = stakes.iter();
        while let Some(info) = iter.next() {
            if info.proposal_id == id {
                return Some(info.clone());
            }
        }
        None
    }

    fn deposit_into_existing(account: &T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
        ensure!(
            MAX_SUPPLY.saturated_into::<BalanceOf<T>>() - T::Currency::total_issuance() >= amount,
            Error::<T>::InsufficientIssuance
        );
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
            Proposals::<T>::insert(id, proposal.clone());
            Self::deposit_event(RawEvent::ProposalChanged(UPDATE, proposal));
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
                proposal.state = if supporters_goals >= 2 * opponents_goals
                    && supporters_goals + opponents_goals > 0
                {
                    Tokens::<T>::insert(
                        &proposal.token_name,
                        Self::clone_from_proposal(proposal.clone()),
                    );
                    ProposalState::Approved
                } else {
                    ProposalState::RejectedClosed
                };
            } else {
                if VotingProposal::exists() {
                    return;
                }

                if proposal.proposal_type == ProposalType::Delist {
                    proposal.state = if supporters_goals > opponents_goals {
                        VotingProposal::put(id);
                        ProposalState::Voting
                    } else {
                        ProposalState::RejectedClosed
                    };
                }

                if proposal.proposal_type == ProposalType::List {
                    proposal.state = if supporters_goals >= 2 * opponents_goals
                        && supporters_goals + opponents_goals > 0
                    {
                        VotingProposal::put(id);
                        ProposalState::Voting
                    } else {
                        ProposalState::RejectedClosed
                    };
                }
            }

            proposal.timestamp = now;
            Proposals::<T>::insert(id, proposal.clone());
            Self::deposit_event(RawEvent::ProposalChanged(UPDATE, proposal));
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
                proposal.state = if supporters_goals >= 2 * opponents_goals
                    && supporters_goals + opponents_goals > 0
                {
                    Tokens::<T>::insert(
                        &proposal.token_name,
                        Self::clone_from_proposal(proposal.clone()),
                    );
                    ProposalState::Approved
                } else {
                    ProposalState::Rejected
                };
            };

            if proposal.proposal_type == ProposalType::Delist {
                proposal.state = if supporters_goals > opponents_goals {
                    Tokens::<T>::remove(&proposal.token_name);
                    ProposalState::Approved
                } else {
                    ProposalState::Rejected
                };
            }

            proposal.timestamp = now;

            VotingProposal::kill();
            Proposals::<T>::insert(id, proposal.clone());
            Self::deposit_event(RawEvent::ProposalChanged(UPDATE, proposal));
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
            Proposals::<T>::insert(id, proposal.clone());
            Self::deposit_event(RawEvent::ProposalChanged(UPDATE, proposal));
        }
    }

    fn get_goals_from_staking(stake: BalanceOf<T>, age_idx: u8) -> u128 {
        let stake = stake.saturated_into::<u128>();
        debug::info!("***************************stake: {}", stake);
        let vote_age = AGE_DAY.get(age_idx as usize).unwrap().0 as u128;
        debug::info!("***************************vote_age: {}", vote_age);
        stake * vote_age
    }

    fn get_now_ts() -> u64 {
        let now = <timestamp::Module<T>>::get();
        <T::Moment as TryInto<u64>>::try_into(now).ok().unwrap()
    }

    fn update_proposal(
        id: ProposalId,
        proposer: T::AccountId,
        new_proposal: Proposal<T::AccountId, BalanceOf<T>>,
    ) -> DispatchResult {
        let proposal: Proposal<T::AccountId, BalanceOf<T>> =
            Self::proposal(id).ok_or(Error::<T>::ProposalNotFound)?;
        ensure!(proposal.proposer == proposer, Error::<T>::NotYourProposal);
        ensure!(
            proposal.state == ProposalState::Pending,
            Error::<T>::ProposalCannotBeModified
        );
        Proposals::<T>::insert(id, new_proposal.clone());
        Self::deposit_event(RawEvent::ProposalChanged(UPDATE, new_proposal));
        Ok(())
    }

    fn remove_proposal(id: ProposalId, proposer: T::AccountId) -> DispatchResult {
        let proposal: Proposal<T::AccountId, BalanceOf<T>> =
            Self::proposal(id).ok_or(Error::<T>::ProposalNotFound)?;
        ensure!(proposal.proposer == proposer, Error::<T>::NotYourProposal);
        ensure!(
            proposal.state == ProposalState::Pending,
            Error::<T>::ProposalCannotBeModified
        );
        Proposals::<T>::remove(id);
        Self::deposit_event(RawEvent::ProposalChanged(DELETE, proposal));
        Ok(())
    }

    fn clone_from_token_info(
        id: ProposalId,
        proposer: T::AccountId,
        proposal_type: ProposalType,
        target_market: MarketType,
        rewards_remainder: BalanceOf<T>,
        timestamp: u64,
        token_info: TokenInfo<BalanceOf<T>>,
    ) -> Proposal<T::AccountId, BalanceOf<T>> {
        Proposal {
            id,
            proposer,
            proposal_type,
            official_website_url: token_info.official_website_url,
            token_icon_url: token_info.token_icon_url,
            token_name: token_info.token_name,
            token_symbol: token_info.token_symbol,
            max_supply: token_info.max_supply,
            circulating_supply: token_info.circulating_supply,
            current_market: token_info.current_market,
            target_market,
            state: ProposalState::Pending,
            review_goals: ZERO_GOALS_U64,
            vote_goals: ZERO_GOALS_U128,
            rewards_remainder,
            timestamp,
        }
    }

    fn clone_from_proposal(
        proposal: Proposal<T::AccountId, BalanceOf<T>>,
    ) -> TokenInfo<BalanceOf<T>> {
        TokenInfo {
            official_website_url: proposal.official_website_url,
            token_icon_url: proposal.token_icon_url,
            token_name: proposal.token_name,
            token_symbol: proposal.token_symbol,
            max_supply: proposal.max_supply,
            circulating_supply: proposal.circulating_supply,
            current_market: proposal.target_market,
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

pub type ProposalChangedType = u8;
pub const CREATE: ProposalChangedType = 1;
pub const UPDATE: ProposalChangedType = 2;
pub const DELETE: ProposalChangedType = 3;

decl_event! {
    pub enum Event<T>
        where
        AccountId = <T as system::Trait>::AccountId,
        Balance = BalanceOf<T>
        {
            ProposalChanged(ProposalChangedType, Proposal<AccountId, Balance>),
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
        /// Not your proposal, you cannot update or delete it.
        NotYourProposal,
        /// The proposal now cannot be reviewed, must in reviewing time can review.
        ProposalCannotBeReviewed,
        /// The proposal now cannot be voted, must in voting time can vote.
        ProposalCannotBeVoted,
        /// The proposal type is Rise or Fall, not for people to vote.
        ProposalNotForVoting,
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
        /// Only can stake in voting-state.
        IllegalStakeTime,
        /// You have not voted.
        NoVote,
        /// You cannot receive rewards now
        /// because proposal state is not allowed.
        StateNotForRewards,
        /// The stake does not match the proposal.
        StakeNotMatch,
        /// Your balance is still in staking time.
        StillInStaking,
        /// total issuance insufficient
        InsufficientIssuance,
    }
}
