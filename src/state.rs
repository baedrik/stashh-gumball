use serde::{Deserialize, Serialize};

/// storage key for the token count
pub const COUNT_KEY: &[u8] = b"count";
/// storage key for the admins list
pub const ADMINS_KEY: &[u8] = b"admin";
/// storage key for the nft contract info
pub const COLLECTION_KEY: &[u8] = b"nftctt";
/// storage key for this minter's address
pub const MY_ADDRESS_KEY: &[u8] = b"myaddr";
/// storage key for the example NFT
pub const EXAMPLE_KEY: &[u8] = b"xamp";
/// storage key for prng seed
pub const PRNG_SEED_KEY: &[u8] = b"seed";
/// storage key for the expected factory address that will call to register a listing
pub const EXPECTED_KEY: &[u8] = b"expect";
/// prefix for token id storage
pub const PREFIX_TOKEN_IDS: &[u8] = b"ids";
/// prefix for storage of viewing keys
pub const PREFIX_VIEW_KEY: &[u8] = b"viewkey";
/// prefix for storage of registered listings
pub const PREFIX_LIST_REGISTRY: &[u8] = b"listing";
/// prefix for storage of whitelisted addresses allowed to receive a free random NFT
pub const PREFIX_WHITELIST: &[u8] = b"white";
/// prefix for the storage of revoked permits
pub const PREFIX_REVOKED_PERMITS: &str = "revoke";

/// various counts
#[derive(Serialize, Deserialize)]
pub struct Counts {
    // number of nfts available
    pub available: u32,
    // number of nfts distributed
    pub released: u64,
}
