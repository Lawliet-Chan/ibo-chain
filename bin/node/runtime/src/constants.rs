// This file is part of Substrate.

// Copyright (C) 2019-2020 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

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

//! A set of constant values used in substrate runtime.

/// Money matters.
pub mod currency {
    use node_primitives::Balance;

    pub const MILLICENTS: Balance = 1_000_000_000;
    pub const CENTS: Balance = 1_000 * MILLICENTS; // assume this is worth about a cent.
    pub const DOLLARS: Balance = 100 * CENTS;

    pub const fn deposit(items: u32, bytes: u32) -> Balance {
        items as Balance * 15 * CENTS + (bytes as Balance) * 6 * CENTS
    }
}

/// Time.
pub mod time {
    use node_primitives::{BlockNumber, Moment};

    /// Since BABE is probabilistic this is the average expected block time that
    /// we are targetting. Blocks will be produced at a minimum duration defined
    /// by `SLOT_DURATION`, but some slots will not be allocated to any
    /// authority and hence no block will be produced. We expect to have this
    /// block time on average following the defined slot duration and the value
    /// of `c` configured for BABE (where `1 - c` represents the probability of
    /// a slot being empty).
    /// This value is only used indirectly to define the unit constants below
    /// that are expressed in blocks. The rest of the code should use
    /// `SLOT_DURATION` instead (like the Timestamp pallet for calculating the
    /// minimum period).
    ///
    /// If using BABE with secondary slots (default) then all of the slots will
    /// always be assigned, in which case `MILLISECS_PER_BLOCK` and
    /// `SLOT_DURATION` should have the same value.
    ///
    /// <https://research.web3.foundation/en/latest/polkadot/BABE/Babe/#6-practical-results>
    pub const MILLISECS_PER_BLOCK: Moment = 3000;
    pub const SECS_PER_BLOCK: Moment = MILLISECS_PER_BLOCK / 1000;

    pub const SLOT_DURATION: Moment = MILLISECS_PER_BLOCK;

    // 1 in 4 blocks (on average, not counting collisions) will be primary BABE blocks.
    pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);

    pub const EPOCH_DURATION_IN_BLOCKS: BlockNumber = 10 * MINUTES;
    pub const EPOCH_DURATION_IN_SLOTS: u64 = {
        const SLOT_FILL_RATE: f64 = MILLISECS_PER_BLOCK as f64 / SLOT_DURATION as f64;

        (EPOCH_DURATION_IN_BLOCKS as f64 * SLOT_FILL_RATE) as u64
    };

    // These time units are defined in number of blocks.
    pub const MINUTES: BlockNumber = 60 / (SECS_PER_BLOCK as BlockNumber);
    pub const HOURS: BlockNumber = MINUTES * 60;
    pub const DAYS: BlockNumber = HOURS * 24;

    pub const MINUTE: u64 = 1000 * 60;
    pub const DAY: u64 = 1000 * 60 * 60 * 24;
}

pub mod congress {
    use crate::constants::time::DAY;
    use crate::constants::time::MINUTE;

    pub const MAX_MEMBERS: u8 = 23;

    pub const PASS_RATE: f32 = 0.66;

    pub const ALLOW_MODIFY_DURATION: u64 = 1000 * 60 * 10; // 1 * DAY;

    pub const REVIEW_DURATION: u64 = 1000 * 60 * 10; // 7 * DAY;
}

pub mod referendum {
    use crate::constants::time::DAY;
    use crate::constants::time::MINUTE;

    pub type VoteAge = u64;

    pub const VOTE_DURATION: u64 = 1000 * 60 * 10; // 7 * DAY;

    pub const RECEIVE_REWARDS_DURATION: u64 = 1000 * 60 * 10; // 30 * DAY;

    pub const AGE_DAY: [(VoteAge, LockPeriod); 6] = [
        (A_AGE, A_DAY),
        (B_AGE, B_DAY),
        (C_AGE, C_DAY),
        (D_AGE, D_DAY),
        (E_AGE, E_DAY),
        (F_AGE, F_DAY),
    ];

    /// lock period 8 days
    pub const A_AGE: VoteAge = 1000;
    /// lock period 16 days
    pub const B_AGE: VoteAge = 1500;
    /// lock period 32 days
    pub const C_AGE: VoteAge = 2250;
    /// lock period 64 days
    pub const D_AGE: VoteAge = 3375;
    /// lock period 128 days
    pub const E_AGE: VoteAge = 5000;
    /// lock period 256 days
    pub const F_AGE: VoteAge = 7600;

    pub type LockPeriod = u64;

    pub const A_DAY: LockPeriod = MINUTE; // 8 * DAY;
    pub const B_DAY: LockPeriod = 2 * MINUTE; // 16 * DAY;
    pub const C_DAY: LockPeriod = 3 * MINUTE; // 32 * DAY;
    pub const D_DAY: LockPeriod = 4 * MINUTE; // 64 * DAY;
    pub const E_DAY: LockPeriod = 5 * MINUTE; // 128 * DAY;
    pub const F_DAY: LockPeriod = 6 * MINUTE; // 256 * DAY;

    /// If total (LockPeriod * VoteAge) >= LIST_PASS_RATE,
    /// it will list the token.
    pub const LIST_PASS_RATE: f32 = 0.66;
    /// If total (LockPeriod * VoteAge) >= DELIST_PASS_RATE,
    /// it will delist the token.
    pub const DELIST_PASS_RATE: f32 = 0.5;
}
