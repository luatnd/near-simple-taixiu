use std::collections::HashMap;
use std::hash::Hash;
use std::vec::Vec;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Serialize, Deserialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::collections::Vector;
use near_sdk::json_types::{U128};
use near_sdk::{
    env,
    near_bindgen,
    // ext_contract,
    log,
    AccountId,
    Balance,
    Promise,
    // PromiseResult,
    // PublicKey,
    // PanicOnDefault,
};


const PAYOUT_PERCENT: u8 = 95; // pay out some % of deposit if user win
const MIN_BET_AMOUNT: Balance = 100_000_000_000_000_000_000_000; // 0.1 NEAR (1 Ⓝ = 1e24 yoctoⓃ)

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[derive(PartialEq, Eq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub enum Bet {
    Big,
    Small,
}

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct BetItem {
    u: AccountId,
    bet: Bet,
    amount: Balance,
    created_at: u64, // unix timestamp
}

type BetResult = bool;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct SimpleTaiXiu {
    pub bets: UnorderedMap<AccountId, BetItem>,
    pub result_bets: UnorderedMap<AccountId, BetResult>,
    pub win_bet: Option<Bet>,
}

impl Default for SimpleTaiXiu {
    fn default() -> Self {
        Self {
            bets: UnorderedMap::new(b"b".to_vec()),
            result_bets: UnorderedMap::new(b"w".to_vec()),
            win_bet: None,
        }
    }
}

#[near_bindgen]
impl SimpleTaiXiu {

    /// User can bet only once
    /// before we call open_result
    ///
    /// Usage by near-cli:
    ///     near call --accountId $ID $N_CONTRACT bet '{"bet": "Big"}'  --deposit 0.1
    ///     near call --accountId $ID $N_CONTRACT get_my_bets
    ///     near view $N_CONTRACT get_bet_results
    ///     near view-state $N_CONTRACT --finality final
    ///
    #[payable]
    pub fn bet(&mut self, bet: Bet) {
        let amount = env::attached_deposit();
        assert!(
            amount >= MIN_BET_AMOUNT,
            "Attached deposit must be greater than MIN_BET_AMOUNT",
        );

        let account_id = env::signer_account_id();
        match self.bets.get(&account_id) {
            Some(value) => {
                assert!(false, "Already bet, please wait after the result revealing")
            },
            None => {
                let bet_item = BetItem {
                    u: account_id.clone(),
                    bet,
                    amount,
                    created_at: env::block_timestamp(),
                };
                self.bets.insert(&account_id, &bet_item);
            }
        }
    }

    /// Reveal the result and calculate the bet result
    ///
    /// Must call this fn by contract owner:
    ///     near call --accountId $N_CONTRACT $N_CONTRACT reveal_result
    ///
    #[private]
    pub fn reveal_result(&mut self) {
        assert!(self.win_bet.is_none(), "Please start a new match");

        let random_bool = (env::block_timestamp() / 100) as u64 % 2 > 0;
        let win_bet = match random_bool {
            true => Bet::Big,
            false => Bet::Small,
        };
        self.win_bet = Some(win_bet.clone());

        // TODO: Bench & Optimize this
        for (k, v) in self.bets.to_vec() {
            let win = v.bet == win_bet;
            self.result_bets.insert(&k, &win);
            if win {
                // transfer money back to user
                // TODO: How to handle large user quantity
                let payout: Balance = v.amount + v.amount * PAYOUT_PERCENT as u128 / 100;
                Promise::new(k).transfer(payout); // TODO: Await contract
            }
        }
    }

    /// Start a new match
    ///
    /// Must call this fn by contract owner:
    ///     near call --accountId $N_CONTRACT $N_CONTRACT start_new_match
    ///
    #[private]
    pub fn start_new_match(&mut self) {
        self.bets = UnorderedMap::new(b"b".to_vec());
        self.result_bets = UnorderedMap::new(b"w".to_vec());
        self.win_bet = None;
    }

    pub fn get_bet_results(&self) -> Vec<(BetItem, Option<BetResult>)> {
        let mut r: Vec<(BetItem, Option<BetResult>)> = vec![];
        for (k, v) in self.bets.to_vec() {
            let win = match self.win_bet.is_some() {
                true => Some(self.result_bets.get(&k).unwrap()),
                false => None,
            };
            r.push((v, win));
        }

        r
    }

    pub fn get_my_bets(&self) -> Option<(BetItem, Option<BetResult>)> {
        let k = env::signer_account_id();
        let a = self.bets.get(&k);
        if a.is_none() {
            return None;
        }

        let v = a.unwrap();
        let win = match self.win_bet.is_some() {
            true => Some(self.result_bets.get(&k).unwrap()),
            false => None,
        };

        Some((v, win))
    }
}

/*
 * the rest of this file sets up unit tests
 * to run these, the command will be:
 * cargo test --package rust-template -- --nocapture
 * Note: 'rust-template' comes from Cargo.toml's 'name' key
 */

// use the attribute below for unit tests
#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{
        // get_logs,
        VMContextBuilder,
    };
    use near_sdk::{testing_env, AccountId};

    // part of writing unit tests is setting up a mock context
    // provide a `predecessor` here, it'll modify the default context
    fn get_context() -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();

        /*
        current_account_id: "contract_near".to_string(),
        signer_account_id: "dev_near".to_string(),
        signer_account_pk: vec![0, 1, 2],
        predecessor_account_id: "other_contract_near".to_string(),
        input,
        block_index: 0,
        block_timestamp: 0,
        account_balance: 0,
        account_locked_balance: 0,
        storage_usage: 0,
        attached_deposit: 1_000_000_000_000_000_000_000_000,
        prepaid_gas: 10u64.pow(18),
        random_seed: vec![0, 1, 2],
        is_view,
        output_data_receivers: vec![],
        epoch_height: 19,
         */
        builder.current_account_id("contract_near".to_string().parse().unwrap());
        builder.signer_account_id("dev.luatnd".to_string().parse().unwrap());
        builder.is_view(false);
        builder.attached_deposit(1_000_000_000_000_000_000_000_000);

        builder
    }

    // TESTS HERE
    #[test]
    fn debug_get_hash() {
        // Basic set up for a unit test
        testing_env!(VMContextBuilder::new().build());

        // Using a unit test to rapidly debug and iterate
        let debug_solution = "near nomicon ref finance";
        let debug_hash_bytes = env::sha256(debug_solution.as_bytes());
        let debug_hash_string = hex::encode(debug_hash_bytes);
        println!("Let's debug: {:?}", debug_hash_string);
    }

    // #[test]
    // fn user_can_bet() {
    //     // Basic set up for a unit test
    //     testing_env!(VMContextBuilder::new().build());
    //
    //     let mut contract = SimpleVnLottery::default();
    //     let count: u8 = 16;
    //     let bet_amount: u8 = 16 as u8 * count;  // compile error: ^^^^^^^^^^^^^^^^ attempt to compute `16_u8 * 16_u8`, which would overflow
    //     println!("Let's debug: {:?}", bet_amount);
    // }

    #[test]
    fn user_can_bet() {
        // Basic set up for a unit test
        testing_env!(get_context().build());

        let mut contract = SimpleTaiXiu::default();
        contract.bet(Bet::Big);
        assert!(
            contract.get_my_bets().is_some(),
            "Cannot place bet"
        );
    }

    #[test]
    #[should_panic]
    fn user_cannot_bet_twice() {
        // Basic set up for a unit test
        testing_env!(get_context().build());

        let mut contract = SimpleTaiXiu::default();
        contract.bet(Bet::Big);
        contract.bet(Bet::Big);
    }

    // #[test]
    // #[should_panic]
    // fn user_cannot_bet_smaller_min_vol() {
    //     // Basic set up for a unit test
    //     testing_env!(get_context().build());
    //
    //     let mut contract = SimpleTaiXiu::default();
    //     // TODO: How to attach Deposit?
    //     contract.bet(Bet::Big);
    // }
}
