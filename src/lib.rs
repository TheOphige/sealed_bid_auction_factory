//! Sealed-Bid Auction Factory (Commit-Reveal)
//!
//! Deploys commit-reveal style sealed-bid auction instances for NFTs.
//! Pattern intentionally mirrors your Dutch auction factory:
//! - Embedded WASM for the instance contract
//! - CREATE2 via RawDeploy with deterministic salt
//! - Simple registry & counter

#![cfg_attr(not(any(test, feature = "export-abi")), no_main)]
#![cfg_attr(not(any(test, feature = "export-abi")), no_std)]

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;
use stylus_sdk::{
    alloy_primitives::{Address, B256, U256},
    crypto,
    deploy::RawDeploy,
    msg,
    prelude::*,
};

// Import the compiled sealed-bid auction WASM bytecode at compile time
static SEALED_BID_AUCTION_WASM: &[u8] = include_bytes!("sealed_bid_auction.wasm");

sol_storage! {
    #[entrypoint]
    pub struct SealedBidAuctionFactory {
        // monotonically increasing id
        uint256 auction_count;

        // id => instance address
        mapping(uint256 => address) auctions;

        // optional: id => creator
        mapping(uint256 => address) creators;

        // factory owner (admin)
        address owner;

        // optional safety: allow pausing new deployments
        bool paused;
    }
}

#[public]
impl SealedBidAuctionFactory {
    /// Initialize the factory (call once)
    pub fn new(&mut self) -> Result<(), Vec<u8>> {
        if self.owner.get() != Address::ZERO {
            return Err("Already initialized".as_bytes().to_vec());
        }
        self.auction_count.set(U256::ZERO);
        self.owner.set(msg::sender());
        self.paused.set(false);
        Ok(())
    }

    /// Admin: pause new auction deployments
    pub fn pause(&mut self) -> Result<(), Vec<u8>> {
        self.only_owner()?;
        self.paused.set(true);
        Ok(())
    }

    /// Admin: unpause new auction deployments
    pub fn unpause(&mut self) -> Result<(), Vec<u8>> {
        self.only_owner()?;
        self.paused.set(false);
        Ok(())
    }

    /// Deploy a new sealed-bid auction instance (commit-reveal)
    ///
    /// Parameters are validated here, but the instance contract enforces details.
    ///
    /// - `nft_contract`: ERC-721 address
    /// - `token_id`: NFT id
    /// - `reserve_price`: minimum acceptable winning price
    /// - `commit_duration`: seconds for commit phase
    /// - `reveal_duration`: seconds for reveal phase (must be > 0)
    /// - `min_deposit`: wei required to commit (anti-spam / griefing bound)
    pub fn create_auction(
        &mut self,
        nft_contract: Address,
        token_id: U256,
        reserve_price: U256,
        commit_duration: U256,
        reveal_duration: U256,
        min_deposit: U256,
    ) -> Result<Address, Vec<u8>> {
        if self.paused.get() {
            return Err("Factory is paused".as_bytes().to_vec());
        }

        if nft_contract == Address::ZERO {
            return Err("Invalid NFT contract".as_bytes().to_vec());
        }
        if reveal_duration == U256::ZERO {
            return Err("Reveal duration must be > 0".as_bytes().to_vec());
        }
        if commit_duration == U256::ZERO {
            return Err("Commit duration must be > 0".as_bytes().to_vec());
        }
        if min_deposit == U256::ZERO {
            return Err("Min deposit must be > 0".as_bytes().to_vec());
        }

        let next_id = self.auction_count.get() + U256::from(1u8);
        let creator = msg::sender();

        // Deterministic salt: binds instance to creator + asset + timing + id.
        // Feel free to tweak the preimage to match your needs.
        let mut salt_preimage = Vec::new();
        salt_preimage.extend_from_slice(&next_id.as_le_bytes());
        salt_preimage.extend_from_slice(creator.as_slice());
        salt_preimage.extend_from_slice(nft_contract.as_slice());
        salt_preimage.extend_from_slice(&token_id.as_le_bytes());
        salt_preimage.extend_from_slice(&reserve_price.as_le_bytes());
        salt_preimage.extend_from_slice(&commit_duration.as_le_bytes());
        salt_preimage.extend_from_slice(&reveal_duration.as_le_bytes());
        salt_preimage.extend_from_slice(&min_deposit.as_le_bytes());

        let salt = B256::from_slice(&crypto::keccak(salt_preimage)[0..32]);

        // Deploy instance using embedded bytecode and CREATE2
        let deployed = unsafe {
            RawDeploy::new()
                .salt(salt)
                .deploy(SEALED_BID_AUCTION_WASM, U256::ZERO)
                .map_err(|e| {
                    let mut err = "Deployment failed: ".as_bytes().to_vec();
                    err.extend_from_slice(&e);
                    err
                })?
        };

        // Book-keeping
        self.auctions.setter(next_id).set(deployed);
        self.creators.setter(next_id).set(creator);
        self.auction_count.set(next_id);

        Ok(deployed)
    }

    /// Get auction address by id
    pub fn get_auction(&self, id: U256) -> Address {
        self.auctions.get(id)
    }

    /// Get creator by id
    pub fn get_creator(&self, id: U256) -> Address {
        self.creators.get(id)
    }

    /// Count
    pub fn get_auction_count(&self) -> U256 {
        self.auction_count.get()
    }

    /// Owner
    pub fn get_owner(&self) -> Address {
        self.owner.get()
    }

    /// Is paused
    pub fn is_paused(&self) -> bool {
        self.paused.get()
    }

    /// Length of embedded instance bytecode
    pub fn get_bytecode_length(&self) -> U256 {
        U256::from(SEALED_BID_AUCTION_WASM.len())
    }
}

impl SealedBidAuctionFactory {
    fn only_owner(&self) -> Result<(), Vec<u8>> {
        if msg::sender() != self.owner.get() {
            return Err("Only owner".as_bytes().to_vec());
        }
        Ok(())
    }
}
