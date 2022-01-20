use crate::contract::BLOCK_SIZE;
use cosmwasm_std::{Api, CanonicalAddr, HumanAddr, StdResult};
use schemars::JsonSchema;
use secret_toolkit::utils::{HandleCallback, Query};
use serde::{Deserialize, Serialize};

/// data for a single royalty
#[derive(Serialize, Deserialize, JsonSchema, Clone, PartialEq, Debug)]
pub struct Royalty {
    /// address to send royalties to
    pub recipient: Option<HumanAddr>,
    /// royalty rate
    pub rate: u16,
}

impl Royalty {
    /// Returns StdResult<StoredRoyalty> from creating a StoredRoyalty from a
    /// Royalty
    ///
    /// # Arguments
    ///
    /// * `api` - a reference to the Api used to convert human and canonical addresses
    pub fn into_stored<A: Api>(self, api: &A) -> StdResult<StoredRoyalty> {
        Ok(StoredRoyalty {
            recipient: self
                .recipient
                .map(|a| api.canonical_address(&a))
                .transpose()?,
            rate: self.rate,
        })
    }
}

/// all royalty information
#[derive(Serialize, Deserialize, JsonSchema, Clone, PartialEq, Debug)]
pub struct RoyaltyInfo {
    /// decimal places in royalty rates
    pub decimal_places_in_rates: u8,
    /// list of royalties
    pub royalties: Vec<Royalty>,
}

impl RoyaltyInfo {
    /// Returns StdResult<StoredRoyaltyInfo> from creating a StoredRoyaltyInfo from a
    /// RoyaltyInfo
    ///
    /// # Arguments
    ///
    /// * `api` - a reference to the Api used to convert human and canonical addresses
    pub fn into_stored<A: Api>(self, api: &A) -> StdResult<StoredRoyaltyInfo> {
        Ok(StoredRoyaltyInfo {
            decimal_places_in_rates: self.decimal_places_in_rates,
            royalties: self
                .royalties
                .into_iter()
                .map(|r| r.into_stored(api))
                .collect::<StdResult<Vec<StoredRoyalty>>>()?,
        })
    }
}

/// data for storing a single royalty
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct StoredRoyalty {
    /// address to send royalties to
    pub recipient: Option<CanonicalAddr>,
    /// royalty rate
    pub rate: u16,
}

impl StoredRoyalty {
    /// Returns StdResult<Royalty> from creating a displayable Royalty from
    /// a StoredRoyalty
    ///
    /// # Arguments
    ///
    /// * `api` - a reference to the Api used to convert human and canonical addresses
    pub fn into_humanized<A: Api>(self, api: &A) -> StdResult<Royalty> {
        Ok(Royalty {
            recipient: self.recipient.map(|a| api.human_address(&a)).transpose()?,
            rate: self.rate,
        })
    }
}

/// all stored royalty information
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct StoredRoyaltyInfo {
    /// decimal places in royalty rates
    pub decimal_places_in_rates: u8,
    /// list of royalties
    pub royalties: Vec<StoredRoyalty>,
}

impl StoredRoyaltyInfo {
    /// Returns StdResult<RoyaltyInfo> from creating a displayable RoyaltyInfo from
    /// a StoredRoyaltyInfo
    ///
    /// # Arguments
    ///
    /// * `api` - a reference to the Api used to convert human and canonical addresses
    pub fn into_humanized<A: Api>(self, api: &A) -> StdResult<RoyaltyInfo> {
        Ok(RoyaltyInfo {
            decimal_places_in_rates: self.decimal_places_in_rates,
            royalties: self
                .royalties
                .into_iter()
                .map(|r| r.into_humanized(api))
                .collect::<StdResult<Vec<Royalty>>>()?,
        })
    }
}

/// information about the minting of the NFT
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct MintRunInfo {
    /// optional address of the SNIP-721 contract creator
    pub collection_creator: Option<HumanAddr>,
    /// optional address of this NFT's creator
    pub token_creator: Option<HumanAddr>,
    /// optional time of minting (in seconds since 01/01/1970)
    pub time_of_minting: Option<u64>,
    /// optional number of the mint run this token was minted in.  This is
    /// used to serialize identical NFTs
    pub mint_run: Option<u32>,
    /// optional serial number in this mint run.  This is used to serialize
    /// identical NFTs
    pub serial_number: Option<u32>,
    /// optional total number of NFTs minted on this run.  This is used to
    /// represent that this token is number m of n
    pub quantity_minted_this_run: Option<u32>,
}

impl MintRunInfo {
    /// Returns StdResult<StoredMintRunInfo> from creating a StoredMintRunInfo from a
    /// MintRunInfo
    ///
    /// # Arguments
    ///
    /// * `api` - a reference to the Api used to convert human and canonical addresses
    pub fn into_stored<A: Api>(self, api: &A) -> StdResult<StoredMintRunInfo> {
        Ok(StoredMintRunInfo {
            collection_creator: self
                .collection_creator
                .map(|a| api.canonical_address(&a))
                .transpose()?,
            token_creator: self
                .token_creator
                .map(|a| api.canonical_address(&a))
                .transpose()?,
            time_of_minting: self.time_of_minting,
            mint_run: self.mint_run,
            serial_number: self.serial_number,
            quantity_minted_this_run: self.quantity_minted_this_run,
        })
    }
}

/// information about the minting of the NFT
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct StoredMintRunInfo {
    /// optional address of the SNIP-721 contract creator
    pub collection_creator: Option<CanonicalAddr>,
    /// optional address of this NFT's creator
    pub token_creator: Option<CanonicalAddr>,
    /// optional time of minting (in seconds since 01/01/1970)
    pub time_of_minting: Option<u64>,
    /// optional number of the mint run this token was minted in.  This is
    /// used to serialize identical NFTs
    pub mint_run: Option<u32>,
    /// optional serial number in this mint run.  This is used to serialize
    /// identical NFTs
    pub serial_number: Option<u32>,
    /// optional total number of NFTs minted on this run.  This is used to
    /// represent that this token is number m of n
    pub quantity_minted_this_run: Option<u32>,
}

impl StoredMintRunInfo {
    /// Returns StdResult<MintRunInfo> from creating a displayable MintRunInfo from
    /// a StoredMintRunInfo
    ///
    /// # Arguments
    ///
    /// * `api` - a reference to the Api used to convert human and canonical addresses
    pub fn into_humanized<A: Api>(self, api: &A) -> StdResult<MintRunInfo> {
        Ok(MintRunInfo {
            collection_creator: self
                .collection_creator
                .map(|a| api.human_address(&a))
                .transpose()?,
            token_creator: self
                .token_creator
                .map(|a| api.human_address(&a))
                .transpose()?,
            time_of_minting: self.time_of_minting,
            mint_run: self.mint_run,
            serial_number: self.serial_number,
            quantity_minted_this_run: self.quantity_minted_this_run,
        })
    }
}

/// NftDossier info stripped down to the listing contract's fields of interest
#[derive(Serialize, Deserialize, JsonSchema, Clone, PartialEq, Debug)]
pub struct NftDossierForListing {
    /// optional public metadata that can be seen by everyone
    pub public_metadata: Option<Metadata>,
    /// optional royalty information for this token
    pub royalty_info: Option<RoyaltyInfo>,
    /// optional mint run information for this token
    pub mint_run_info: Option<MintRunInfo>,
}

impl NftDossierForListing {
    /// Returns StdResult<StoredNftDossierForListing> from creating a StoredNftDossierForListing from a
    /// NftDossierForListing
    ///
    /// # Arguments
    ///
    /// * `api` - a reference to the Api used to convert human and canonical addresses
    pub fn into_stored<A: Api>(self, api: &A) -> StdResult<StoredNftDossierForListing> {
        Ok(StoredNftDossierForListing {
            public_metadata: self.public_metadata,
            royalty_info: self.royalty_info.map(|r| r.into_stored(api)).transpose()?,
            mint_run_info: self.mint_run_info.map(|m| m.into_stored(api)).transpose()?,
        })
    }
}

/// NftDossier info stripped down to the listing contract's fields of interest
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct StoredNftDossierForListing {
    /// optional public metadata that can be seen by everyone
    pub public_metadata: Option<Metadata>,
    /// optional royalty information for this token
    pub royalty_info: Option<StoredRoyaltyInfo>,
    /// optional mint run information for this token
    pub mint_run_info: Option<StoredMintRunInfo>,
}

impl StoredNftDossierForListing {
    /// Returns StdResult<NftDossierForListing> from creating a displayable NftDossierForListing from
    /// a StoredNftDossierForListing
    ///
    /// # Arguments
    ///
    /// * `api` - a reference to the Api used to convert human and canonical addresses
    pub fn into_humanized<A: Api>(self, api: &A) -> StdResult<NftDossierForListing> {
        Ok(NftDossierForListing {
            public_metadata: self.public_metadata,
            royalty_info: self
                .royalty_info
                .map(|r| r.into_humanized(api))
                .transpose()?,
            mint_run_info: self
                .mint_run_info
                .map(|m| m.into_humanized(api))
                .transpose()?,
        })
    }
}

/// wrapper to deserialize NftDossier responses
#[derive(Serialize, Deserialize, JsonSchema, Clone, PartialEq, Debug)]
pub struct NftDossierResponse {
    pub nft_dossier: NftDossierForListing,
}

/// snip721 handle msgs
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Snip721HandleMsg {
    /// transfer many tokens
    BatchTransferNft {
        /// list of transfers to perform
        transfers: Vec<Transfer>,
    },
}

impl HandleCallback for Snip721HandleMsg {
    const BLOCK_SIZE: usize = BLOCK_SIZE;
}

/// token transfer info used when doing a BatchTransferNft
#[derive(Serialize)]
pub struct Transfer {
    /// recipient of the transferred tokens
    pub recipient: HumanAddr,
    /// tokens being transferred
    pub token_ids: Vec<String>,
    /// memo for the tx
    pub memo: String,
}

/// snip721 query msgs
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Snip721QueryMsg {
    /// displays all the public information about a token
    NftDossier { token_id: String },
}

impl Query for Snip721QueryMsg {
    const BLOCK_SIZE: usize = BLOCK_SIZE;
}

/// token metadata
#[derive(Serialize, Deserialize, JsonSchema, Clone, PartialEq, Debug, Default)]
pub struct Metadata {
    /// optional uri for off-chain metadata.  This should be prefixed with `http://`, `https://`, `ipfs://`, or
    /// `ar://`.  Only use this if you are not using `extension`
    pub token_uri: Option<String>,
    /// optional on-chain metadata.  Only use this if you are not using `token_uri`
    pub extension: Option<Extension>,
}

/// metadata extension
/// You can add any metadata fields you need here.  These fields are based on
/// https://docs.opensea.io/docs/metadata-standards and are the metadata fields that
/// Stashh uses for robust NFT display.  Urls should be prefixed with `http://`, `https://`, `ipfs://`, or
/// `ar://`
#[derive(Serialize, Deserialize, JsonSchema, Clone, PartialEq, Debug, Default)]
pub struct Extension {
    /// url to the image
    pub image: Option<String>,
    /// raw SVG image data (not recommended). Only use this if you're not including the image parameter
    pub image_data: Option<String>,
    /// url to allow users to view the item on your site
    pub external_url: Option<String>,
    /// item description
    pub description: Option<String>,
    /// name of the item
    pub name: Option<String>,
    /// item attributes
    pub attributes: Option<Vec<Trait>>,
    /// background color represented as a six-character hexadecimal without a pre-pended #
    pub background_color: Option<String>,
    /// url to a multimedia attachment
    pub animation_url: Option<String>,
    /// url to a YouTube video
    pub youtube_url: Option<String>,
    /// media files as specified on Stashh that allows for basic authenticatiion and decryption keys.
    /// Most of the above is used for bridging public eth NFT metadata easily, whereas `media` will be used
    /// when minting NFTs on Stashh
    pub media: Option<Vec<MediaFile>>,
    /// a select list of trait_types that are in the private metadata.  This will only ever be used
    /// in public metadata
    pub protected_attributes: Option<Vec<String>>,
}

/// attribute trait
#[derive(Serialize, Deserialize, JsonSchema, Clone, PartialEq, Debug, Default)]
pub struct Trait {
    /// indicates how a trait should be displayed
    pub display_type: Option<String>,
    /// name of the trait
    pub trait_type: Option<String>,
    /// trait value
    pub value: String,
    /// optional max value for numerical traits
    pub max_value: Option<String>,
}

/// media file
#[derive(Serialize, Deserialize, JsonSchema, Clone, PartialEq, Debug, Default)]
pub struct MediaFile {
    /// file type
    /// Stashh currently uses: "image", "video", "audio", "text", "font", "application"
    pub file_type: Option<String>,
    /// file extension
    pub extension: Option<String>,
    /// authentication information
    pub authentication: Option<Authentication>,
    /// url to the file.  Urls should be prefixed with `http://`, `https://`, `ipfs://`, or `ar://`
    pub url: String,
}

/// media file authentication
#[derive(Serialize, Deserialize, JsonSchema, Clone, PartialEq, Debug, Default)]
pub struct Authentication {
    /// either a decryption key for encrypted files or a password for basic authentication
    pub key: Option<String>,
    /// username used in basic authentication
    pub user: Option<String>,
}
