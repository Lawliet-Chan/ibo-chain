#![cfg_attr(not(feature = "std"), no_std)]

extern crate frame_system as system;
extern crate pallet_timestamp as timestamp;

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

pub trait Trait: system::Trait + timestamp::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    type Currency: ReservableCurrency<Self::AccountId>;
}

#[derive(Encode, Decode, Clone, Debug, Default, PartialEq, Eq)]
pub struct TokenInfo<AccountId, Balance> {
    pub proposer: AccountId,
    pub official_website_url: Vec<u8>,
    pub token_icon_url: Vec<u8>,
    pub token_symbol: Vec<u8>,
    pub total_issuance: Balance,
    pub total_circulation: Balance,
    pub current_board: BoardType,
}

#[derive(Encode, Decode, Clone, Debug, Default, PartialEq, Eq)]
pub struct Proposal<AccountId, Balance> {
    pub proposer: AccountId,
    pub official_website_url: Vec<u8>,
    pub token_icon_url: Vec<u8>,
    pub token_symbol: Vec<u8>,
    pub total_issuance: Balance,
    pub total_circulation: Balance,
    pub current_board: BoardType,
    pub target_board: BoardType,
    pub state: ProposalState,
    pub timestamp: u64,
}

#[derive(Encode, Decode, Clone, Debug, Default, PartialEq, Eq)]
pub enum BoardType {
    Main,
    Growth,
    Off,
}

#[derive(Encode, Decode, Clone, Debug, Default, PartialEq, Eq)]
pub enum ProposalState {
    Pending,
    Approved,
    Rejected,
}

decl_storage! {
    trait Store for Module<T: Trait> as Ibo {
        pub Proposals get(fn proposals): Vec<Proposal<T::AccountId, BalanceOf<T>>>;

        pub Tokens get(fn token): map hasher(twox_64_concat) Vec<u8> => TokenInfo<T::AccountId, BalanceOf<T>>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        #[weight = 200]
        fn list_proposal(
            origin,
            official_website_url: Vec<u8>,
            token_icon_url: Vec<u8>,
            token_symbol: Vec<u8>,
            total_issuance: BalanceOf<T>,
            total_circulation: BalanceOf<T>,
            target_board: BoardType
        ) {
            let proposer = ensure_signed(origin)?;
            ensure!(Tokens::<T>::contains_key(&token_symbol), Error::<T>::TokenExists);
            let now = Self::get_now_ts();
            let new_proposal = Proposal {
                proposer,
                official_website_url,
                token_icon_url,
                token_symbol,
                total_issuance,
                total_circulation,
                current_board: BoardType::Off,
                target_board,
                state: ProposalState::Pending,
                timestamp: now,
            }
            Proposals::<T>::mutate(|p| p.push(new_proposal));
        }

        #[weight = 200]
        fn delist_proposal(origin, token_symbol: Vec<u8>) {
            let proposer = ensure_signed(origin)?;
            let token_info = Self::token(&token_symbol).ok_or(Error::<T>::TokenNotFound)?;
        }

        #[weight = 100]
        fn rise_proposal(origin, token_symbol: Vec<u8>) {
            let proposer = ensure_signed(origin)?;
            let token_info = Self::token(&token_symbol).ok_or(Error::<T>::TokenNotFound)?;
        }

        #[weight = 100]
        fn fall_proposal(origin, token_symbol: Vec<u8>) {
            let proposer = ensure_signed(origin)?;
            let token_info = Self::token(&token_symbol).ok_or(Error::<T>::TokenNotFound)?;
        }

    }
}

impl<T: Trait> Module<T> {
    fn get_now_ts() -> u64 {
        let now = <timestamp::Module<T>>::get();
        <T::Moment as TryInto<u64>>::try_into(now).ok().unwrap()
    }
}

decl_error! {
    /// Error for the ipse module.
    pub enum Error for Module<T: Trait> {
        /// There is the same name token.
        TokenExists,
        /// There is no token named it.
        TokenNotFound,
    }
}
