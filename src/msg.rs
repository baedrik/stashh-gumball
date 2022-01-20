#![allow(clippy::large_enum_variant)]
use crate::contract_info::ContractInfo;
use crate::snip721::NftDossierForListing;
use cosmwasm_std::{HumanAddr, Uint128};
use schemars::JsonSchema;
use secret_toolkit::permit::Permit;
use serde::{Deserialize, Serialize};

/// Instantiation message
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InitMsg {
    /// address and code hash of the collection contract
    pub nft_contract: ContractInfo,
    /// entropy used for random viewing key generation
    pub entropy: String,
}

/// Handle messages
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    /// BatchReceiveNft is called when the gumball is sent NFTs.  This signifies the
    /// desire to include those NFTs in the possible random selections
    BatchReceiveNft {
        /// address of the previous owner of the tokens being added to the gumball
        from: HumanAddr,
        /// list of tokens sent
        token_ids: Vec<String>,
    },
    /// ReceiveNft is only included to maintain CW721 compliance.  Hopefully everyone uses the
    /// superior BatchReceiveNft process.  ReceiveNft is called when the gumball is sent an NFT.
    /// This signifies the desire to include that NFT as a possible random selection
    ReceiveNft {
        /// address of the previous owner of the token being sent to the gumball
        sender: HumanAddr,
        /// the token sent
        token_id: String,
    },
    /// Create a viewing key
    CreateViewingKey { entropy: String },
    /// Set a viewing key
    SetViewingKey {
        key: String,
        // optional padding can be used so message length doesn't betray key length
        padding: Option<String>,
    },
    /// allows an admin to add more admins
    AddAdmins {
        /// list of address to grant admin priveleges
        admins: Vec<HumanAddr>,
    },
    /// allows an admin to remove admin addresses
    RemoveAdmins {
        /// list of address to revoke admin priveleges from
        admins: Vec<HumanAddr>,
    },
    /// add to whitelist.  This whitelist is for who is permitted to receive a free random NFT.
    /// Whitelisting for purchases must be done on the listing level, or a single non-whitelisted
    /// purchaser will cause all whitelisted purchases in the same batch to fail
    AddToWhitelist {
        /// whitelisted addresses
        addresses: Vec<HumanAddr>,
    },
    /// remove from whitelist.  This whitelist is for who is permitted to receive a free random NFT.
    /// Whitelisting for purchases must be done on the listing level, or a single non-whitelisted
    /// purchaser will cause all whitelisted purchases in the same batch to fail
    RemoveFromWhitelist {
        /// de-whitelisted addresses
        addresses: Vec<HumanAddr>,
    },
    /// Mint an NFT for each buyer.  This can only be called by a listing contract this minter has created,
    /// an admin, or a whitelisted address
    Mint {
        /// the addresses that should receive NFTs
        buyers: Vec<HumanAddr>,
        /// string used for entropy
        entropy: String,
    },
    /// create a gumball listing
    CreateListing {
        /// String label for the listing
        label: String,
        /// optional address to send proceeds to if not the creator
        payment_address: Option<HumanAddr>,
        /// factory contract code hash and address
        factory_contract: ContractInfo,
        /// purchase contract code hash and address
        buy_contract: ContractInfo,
        /// true if purchasing token implements BatchSend
        batch_send: bool,
        /// listing price
        price: Uint128,
        /// timestamp after which the operator may close the listing.
        /// Timestamp is in seconds since epoch 01/01/1970
        closes_at: u64,
        /// Optional free-form description of the listing
        description: Option<String>,
        /// entropy used for random viewing key generation
        entropy: String,
    },
    /// register a listing address that will be allowed to request minting.  This will only be accepted
    /// from the factory address just called when doing the CreateListing
    RegisterListing { listing_address: HumanAddr },
    /// disallow the use of a permit
    RevokePermit {
        /// name of the permit that is no longer valid
        permit_name: String,
    },
    /// set a viewing key with an nft contract that is different from the contract this gumball was created for.
    /// This can only be called by an admin and can only be used on contracts that are not the nft contract
    /// specified during instantiation, because that would allow an admin to see what nfts are still left in the gumball.
    /// This is only meant to facilitate in the retrieval of an nft accidentally sent to the gumball
    SetViewingKeyWithCollection {
        /// the code hash and address of the other nft contract that controls an nft that was accidentally sent
        /// to the gumball
        nft_contract: ContractInfo,
        /// viewing key to set with that nft contract
        viewing_key: String,
    },
    /// retrieve an nft that is from a different contract than what the gumball was created for, but was
    /// accidentally sent to the gumball.  This can only be called by an admin and can only be used on contracts
    /// that are not the nft contract specified during instantiation, because that would allow an admin to handpick rare
    /// nfts instead of getting a random selection
    RetrieveNft {
        /// the code hash and address of the other nft contract that controls an nft that was accidentally sent
        /// to the gumball
        nft_contract: ContractInfo,
        /// ids of the tokens to transfer to the admin doing this tx
        token_ids: Vec<String>,
    },
}

/// Responses from handle functions
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleAnswer {
    /// response from creating a viewing key
    ViewingKey { key: String },
    /// response from adding/removing admins
    AdminsList {
        // current admins
        admins: Vec<HumanAddr>,
    },
    /// response from revoking a permit
    RevokePermit { status: String },
    /// response from adding to a whitelist
    AddToWhitelist { status: String },
    /// response from removing from a whitelist
    RemoveFromWhitelist { status: String },
    /// response from retrieving nfts from the wrong collection
    RetrieveNft { status: String },
}

/// Queries
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// lists the admins for this minter
    Admins {
        /// optional address and viewing key of an admin
        viewer: Option<ViewerInfo>,
        /// optional permit used to verify admin identity.  If both viewer and permit
        /// are provided, the viewer will be ignored
        permit: Option<Permit>,
    },
    /// display the public info of an example NFT.  This is used for a universal minter query that
    /// listings will use
    NftListingDisplay {},
    /// display the counts of how many NFTs are currently available and how many have been
    /// released by the gumball
    Counts {},
    /// display the address and code hash of the nft contract this gumball is used with
    NftContract {},
}

/// responses to queries
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryAnswer {
    /// response listing the current admin addresses
    Admins {
        // current admins
        admins: Vec<HumanAddr>,
    },
    /// display the public info of an example NFT
    NftListingDisplay {
        /// the nft fields of interest
        nft_info: NftDossierForListing,
        /// nft contract address that will contain this token,
        nft_contract_address: HumanAddr,
        /// true if this minting option can mint one more nft
        mintable: bool,
    },
    /// display the gumball counts
    Counts {
        /// count of available NFTs
        available: u32,
        /// number of NFTs released
        released: u64,
    },
    /// display the address and code hash of the nft contract this gumball is used with
    NftContract {
        code_hash: String,
        address: HumanAddr,
    },
}

/// the address and viewing key making an authenticated query request
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ViewerInfo {
    /// querying address
    pub address: HumanAddr,
    /// authentication key string
    pub viewing_key: String,
}
