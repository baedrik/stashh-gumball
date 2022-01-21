use cosmwasm_std::{
    log, to_binary, Api, CanonicalAddr, Env, Extern, HandleResponse, HandleResult, HumanAddr,
    InitResponse, InitResult, Querier, QueryResult, ReadonlyStorage, StdError, StdResult, Storage,
    Uint128,
};
use cosmwasm_storage::{PrefixedStorage, ReadonlyPrefixedStorage};

use secret_toolkit::{
    permit::{validate, Permit, RevokedPermits},
    snip721::{register_receive_nft_msg, set_viewing_key_msg},
    utils::{pad_handle_result, pad_query_result, HandleCallback, Query},
};

use crate::factory_msgs::FactoryHandleMsg;
use crate::msg::{HandleAnswer, HandleMsg, InitMsg, QueryAnswer, QueryMsg, ViewerInfo};
use crate::rand::{extend_entropy, sha_256, Prng};
use crate::snip721::{
    NftDossierForListing, NftDossierResponse, Snip721HandleMsg, Snip721QueryMsg, Transfer,
};
use crate::state::{
    Counts, ADMINS_KEY, COLLECTION_KEY, COUNT_KEY, EXAMPLE_KEY, EXPECTED_KEY, MY_ADDRESS_KEY,
    PREFIX_LIST_REGISTRY, PREFIX_REVOKED_PERMITS, PREFIX_TOKEN_IDS, PREFIX_VIEW_KEY,
    PREFIX_WHITELIST, PRNG_SEED_KEY,
};
use crate::storage::{load, may_load, remove, save};
use crate::viewing_key::{ViewingKey, VIEWING_KEY_SIZE};
use crate::{
    contract_info::{ContractInfo, StoreContractInfo},
    snip721::StoredNftDossierForListing,
};

pub const BLOCK_SIZE: usize = 256;

////////////////////////////////////// Init ///////////////////////////////////////
/// Returns InitResult
///
/// Initializes the minting contract
///
/// # Arguments
///
/// * `deps` - mutable reference to Extern containing all the contract's external dependencies
/// * `env` - Env of contract's environment
/// * `msg` - InitMsg passed in with the instantiation message
pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> InitResult {
    save(
        &mut deps.storage,
        MY_ADDRESS_KEY,
        &deps.api.canonical_address(&env.contract.address)?,
    )?;
    let sender_raw = deps.api.canonical_address(&env.message.sender)?;
    let prng_seed: Vec<u8> = sha_256(base64::encode(msg.entropy.as_bytes()).as_bytes()).to_vec();
    save(&mut deps.storage, PRNG_SEED_KEY, &prng_seed)?;
    let admins = vec![sender_raw];
    save(&mut deps.storage, ADMINS_KEY, &admins)?;
    let counts = Counts {
        available: 0,
        released: 0,
    };
    save(&mut deps.storage, COUNT_KEY, &counts)?;
    let messages = vec![register_receive_nft_msg(
        env.contract_code_hash,
        Some(true),
        None,
        BLOCK_SIZE,
        msg.nft_contract.code_hash.clone(),
        msg.nft_contract.address.clone(),
    )?];
    let contract = msg.nft_contract.into_store(&deps.api)?;
    save(&mut deps.storage, COLLECTION_KEY, &contract)?;

    Ok(InitResponse {
        messages,
        log: vec![],
    })
}

///////////////////////////////////// Handle //////////////////////////////////////
/// Returns HandleResult
///
/// # Arguments
///
/// * `deps` - mutable reference to Extern containing all the contract's external dependencies
/// * `env` - Env of contract's environment
/// * `msg` - HandleMsg passed in with the execute message
pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> HandleResult {
    let response = match msg {
        HandleMsg::BatchReceiveNft { from, token_ids } => {
            try_batch_receive(deps, &env.message.sender, &from, token_ids)
        }
        HandleMsg::ReceiveNft { sender, token_id } => {
            try_batch_receive(deps, &env.message.sender, &sender, vec![token_id])
        }
        HandleMsg::CreateViewingKey { entropy } => try_create_key(deps, &env, &entropy),
        HandleMsg::SetViewingKey { key, .. } => try_set_key(deps, &env.message.sender, key),
        HandleMsg::AddAdmins { admins } => try_add_admins(deps, &env.message.sender, admins),
        HandleMsg::RemoveAdmins { admins } => try_remove_admins(deps, &env.message.sender, admins),
        HandleMsg::Mint { buyers, entropy } => try_mint(deps, &env, buyers, &entropy),
        HandleMsg::RegisterListing { listing_address } => {
            try_register_listing(deps, &env.message.sender, &listing_address)
        }
        HandleMsg::CreateListing {
            label,
            payment_address,
            factory_contract,
            buy_contract,
            batch_send,
            price,
            closes_at,
            description,
            entropy,
        } => try_create_listing(
            deps,
            env,
            label,
            payment_address,
            factory_contract,
            buy_contract,
            batch_send,
            price,
            closes_at,
            description,
            entropy,
        ),
        HandleMsg::RevokePermit { permit_name } => {
            revoke_permit(&mut deps.storage, &env.message.sender, &permit_name)
        }
        HandleMsg::AddToWhitelist { addresses } => {
            try_update_whitelist(deps, &env.message.sender, &addresses, true)
        }
        HandleMsg::RemoveFromWhitelist { addresses } => {
            try_update_whitelist(deps, &env.message.sender, &addresses, false)
        }
        HandleMsg::SetViewingKeyWithCollection {
            nft_contract,
            viewing_key,
        } => try_set_key_with_coll(deps, &env.message.sender, nft_contract, viewing_key),
        HandleMsg::RetrieveNft {
            nft_contract,
            token_ids,
        } => try_retrieve(deps, env, nft_contract, token_ids),
    };
    pad_handle_result(response, BLOCK_SIZE)
}

/// Returns HandleResult
///
/// sets a viewing key with a contract that is NOT the nft contract specified during instantiation.  This is
/// only used to facilitate in the retrieval of an nft accidentally sent from the wrong collection
///
/// # Arguments
///
/// * `deps` - a mutable reference to Extern containing all the contract's external dependencies
/// * `sender` - a reference to the message sender
/// * `nft_contract` - code hash and address of the accidental collection
/// * `viewing_key` - viewing key to set with the accidental collection
fn try_set_key_with_coll<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    sender: &HumanAddr,
    nft_contract: ContractInfo,
    viewing_key: String,
) -> HandleResult {
    // only allow admins to do this
    let admins: Vec<CanonicalAddr> = load(&deps.storage, ADMINS_KEY)?;
    let sender_raw = deps.api.canonical_address(sender)?;
    if !admins.contains(&sender_raw) {
        return Err(StdError::unauthorized());
    }
    let contract =
        load::<StoreContractInfo, _>(&deps.storage, COLLECTION_KEY)?.into_humanized(&deps.api)?;
    if contract.address == nft_contract.address {
        return Err(StdError::generic_err(
            "This may not be called on the gumball contract's collection",
        ));
    }
    let messages = vec![set_viewing_key_msg(
        viewing_key.clone(),
        None,
        BLOCK_SIZE,
        nft_contract.code_hash,
        nft_contract.address,
    )?];
    Ok(HandleResponse {
        messages,
        log: vec![],
        data: Some(to_binary(&HandleAnswer::ViewingKey { key: viewing_key })?),
    })
}

/// Returns HandleResult
///
/// retrieves nfts sent from the wrong contract.  This can only be called on a contract that is NOT the nft
/// contract specified during instantiation
///
/// # Arguments
///
/// * `deps` - a mutable reference to Extern containing all the contract's external dependencies
/// * `env` - the Env of contract's environment
/// * `nft_contract` - code hash and address of the accidental collection
/// * `token_ids` - list of nfts to retrieve
fn try_retrieve<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    nft_contract: ContractInfo,
    token_ids: Vec<String>,
) -> HandleResult {
    // only allow admins to do this
    let admins: Vec<CanonicalAddr> = load(&deps.storage, ADMINS_KEY)?;
    let sender_raw = deps.api.canonical_address(&env.message.sender)?;
    if !admins.contains(&sender_raw) {
        return Err(StdError::unauthorized());
    }
    let contract =
        load::<StoreContractInfo, _>(&deps.storage, COLLECTION_KEY)?.into_humanized(&deps.api)?;
    if contract.address == nft_contract.address {
        return Err(StdError::generic_err(
            "This may not be called on the gumball contract's collection",
        ));
    }
    let transfers = vec![Transfer {
        recipient: env.message.sender,
        token_ids,
        memo: format!("Retrieved from gumball: {}", env.contract.address),
    }];
    let messages = vec![
        Snip721HandleMsg::BatchTransferNft { transfers }.to_cosmos_msg(
            nft_contract.code_hash,
            nft_contract.address,
            None,
        )?,
    ];
    Ok(HandleResponse {
        messages,
        log: vec![],
        data: Some(to_binary(&HandleAnswer::RetrieveNft {
            status: "success".to_string(),
        })?),
    })
}

/// Returns HandleResult
///
/// adds/removes addresses to/from the whitelist
///
/// # Arguments
///
/// * `deps` - a mutable reference to Extern containing all the contract's external dependencies
/// * `sender` - a reference to the message sender
/// * `addresses` - list of whitelisted addresses
/// * `is_add` - true if adding to the whitelist
fn try_update_whitelist<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    sender: &HumanAddr,
    addresses: &[HumanAddr],
    is_add: bool,
) -> HandleResult {
    // only allow admins to do this
    let admins: Vec<CanonicalAddr> = load(&deps.storage, ADMINS_KEY)?;
    let sender_raw = deps.api.canonical_address(sender)?;
    if !admins.contains(&sender_raw) {
        return Err(StdError::unauthorized());
    }
    let mut white_store = PrefixedStorage::new(PREFIX_WHITELIST, &mut deps.storage);
    for addr in addresses.iter() {
        let raw = deps.api.canonical_address(addr)?;
        if is_add {
            save(&mut white_store, raw.as_slice(), &true)?;
        } else {
            remove(&mut white_store, raw.as_slice());
        }
    }
    let status = "success".to_string();
    let resp = if is_add {
        HandleAnswer::AddToWhitelist { status }
    } else {
        HandleAnswer::RemoveFromWhitelist { status }
    };
    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&resp)?),
    })
}

/// Returns HandleResult
///
/// registers a listing address as a valid address to request minting
///
/// # Arguments
///
/// * `deps` - a mutable reference to Extern containing all the contract's external dependencies
/// * `sender` - a reference to the message sender
/// * `listing_address` - a reference to the address of the listing this contract just created
fn try_register_listing<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    sender: &HumanAddr,
    listing_address: &HumanAddr,
) -> HandleResult {
    let factory: HumanAddr = may_load(&deps.storage, EXPECTED_KEY)?.ok_or_else(|| {
        StdError::generic_err("RegisterListing can only be called by the expected factory contract")
    })?;
    if *sender != factory {
        return Err(StdError::generic_err(
            "Message sender does not match the expected factory address",
        ));
    }
    let mut reg_store = PrefixedStorage::new(PREFIX_LIST_REGISTRY, &mut deps.storage);
    let list_raw = deps.api.canonical_address(listing_address)?;
    save(&mut reg_store, list_raw.as_slice(), &true)?;
    remove(&mut deps.storage, EXPECTED_KEY);
    Ok(HandleResponse::default())
}

/// Returns HandleResult
///
/// handles receiving an NFT to place in the gumball machine
///
/// # Arguments
///
/// * `deps` - mutable reference to Extern containing all the contract's external dependencies
/// * `sender` - a reference to the message sender's address
/// * `from` - a reference to the address that owned the NFT
/// * `token_ids` - list of tokens sent
fn try_batch_receive<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    sender: &HumanAddr,
    from: &HumanAddr,
    mut token_ids: Vec<String>,
) -> HandleResult {
    let contract =
        load::<StoreContractInfo, _>(&deps.storage, COLLECTION_KEY)?.into_humanized(&deps.api)?;
    // don't let someone spoof sending the gumball tokens
    if *sender != contract.address {
        return Err(StdError::generic_err(
            "Only the collection contract specified on instantiation may call (Batch)ReceiveNft",
        ));
    }
    let admins: Vec<CanonicalAddr> = load(&deps.storage, ADMINS_KEY)?;
    let from_raw = deps.api.canonical_address(from)?;
    // only allow an admin to add tokens to the gumball
    if !admins.contains(&from_raw) {
        return Err(StdError::unauthorized());
    }
    // 721 contracts should not be doing a Send if there are no tokens sent, but you never know
    // what people will code
    if !token_ids.is_empty() {
        let mut counts: Counts = load(&deps.storage, COUNT_KEY)?;
        // use the public info of the first NFT added to an empty gumball machine
        let save_example = counts.available == 0;
        let mut id_store = PrefixedStorage::new(PREFIX_TOKEN_IDS, &mut deps.storage);
        for id in token_ids.iter() {
            save(&mut id_store, &counts.available.to_le_bytes(), id)?;
            counts.available = counts.available.checked_add(1).ok_or_else(|| {
                StdError::generic_err("Gumball contract has reached its maximum number of NFTs")
            })?;
        }
        save(&mut deps.storage, COUNT_KEY, &counts)?;
        // if the gumball machine was empty
        if save_example {
            // query the first token's info
            let nft_qry = Snip721QueryMsg::NftDossier {
                token_id: token_ids.swap_remove(0),
            };
            let resp: StdResult<NftDossierResponse> =
                nft_qry.query(&deps.querier, contract.code_hash, contract.address);
            let nft_doss = resp.map_or(
                NftDossierForListing {
                    public_metadata: None,
                    royalty_info: None,
                    mint_run_info: None,
                },
                |r| r.nft_dossier,
            );
            let store_doss = nft_doss.into_stored(&deps.api)?;
            save(&mut deps.storage, EXAMPLE_KEY, &store_doss)?;
        }
    }
    Ok(HandleResponse::default())
}

/// Returns HandleResult
///
/// call the factory to create a listing
///
/// # Arguments
///
/// * `deps` - a mutable reference to Extern containing all the contract's external dependencies
/// * `env` - the Env of contract's environment
/// * `label` - the String label of the listing to create
/// * `payment_address` - optional payment address if different than the creator
/// * `factory_contract` - code hash and address of the factory
/// * `buy_contract` - ContractInfo of the purchasing token
/// * `batch_send` - true if the purchasing token implements batch send
/// * `price` - listing price
/// * `closes_at` - seconds since 01/01/1970 in which the listing can be closed by the operator
/// * `description` - optional text description of the listing
/// * `entropy` - String used for entropy when generating viewing keys
#[allow(clippy::too_many_arguments)]
fn try_create_listing<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    label: String,
    payment_address: Option<HumanAddr>,
    factory_contract: ContractInfo,
    buy_contract: ContractInfo,
    batch_send: bool,
    price: Uint128,
    closes_at: u64,
    description: Option<String>,
    entropy: String,
) -> HandleResult {
    // only allow admins to do this
    let admins: Vec<CanonicalAddr> = load(&deps.storage, ADMINS_KEY)?;
    let sender_raw = deps.api.canonical_address(&env.message.sender)?;
    if !admins.contains(&sender_raw) {
        return Err(StdError::unauthorized());
    }
    let contract =
        load::<StoreContractInfo, _>(&deps.storage, COLLECTION_KEY)?.into_humanized(&deps.api)?;
    save(&mut deps.storage, EXPECTED_KEY, &factory_contract.address)?;
    let minter_contract = ContractInfo {
        address: env.contract.address,
        code_hash: env.contract_code_hash,
    };
    let quantity_for_sale = load::<Counts, _>(&deps.storage, COUNT_KEY)?.available;
    let factory_msg = FactoryHandleMsg::CreateMinterListing {
        label,
        creator: env.message.sender,
        payment_address,
        quantity_for_sale,
        minter_contract,
        option_id: "Gumball".to_string(),
        buy_contract,
        batch_send,
        price,
        closes_at,
        description,
        entropy,
        nft_contract_address: contract.address,
        implements_register_listing: true,
    };

    Ok(HandleResponse {
        messages: vec![factory_msg.to_cosmos_msg(
            factory_contract.code_hash,
            factory_contract.address,
            None,
        )?],
        log: vec![],
        data: None,
    })
}

// type of address calling Mint
pub enum MintCaller {
    Listing,
    Admin,
    Whitelist,
}

/// Returns HandleResult
///
/// release a random nft for each buyer
///
/// # Arguments
///
/// * `deps` - a mutable reference to Extern containing all the contract's external dependencies
/// * `env` - a reference to the Env of contract's environment
/// * `buyers` - the nft buyers
/// * `entropy` - string slice used for entropy
fn try_mint<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: &Env,
    buyers: Vec<HumanAddr>,
    entropy: &str,
) -> HandleResult {
    let sender_raw = deps.api.canonical_address(&env.message.sender)?;
    let sender_slice = sender_raw.as_slice();
    let reg_store = ReadonlyPrefixedStorage::new(PREFIX_LIST_REGISTRY, &deps.storage);
    // check if the caller is a listing this contract created
    let caller_type = if may_load::<bool, _>(&reg_store, sender_slice)?.is_none() {
        // check if the caller is a whitelisted address for this template
        let mut white_store = PrefixedStorage::new(PREFIX_WHITELIST, &mut deps.storage);
        if may_load::<bool, _>(&white_store, sender_slice)?.is_none() {
            // check if the caller is an admin
            let admins: Vec<CanonicalAddr> = load(&deps.storage, ADMINS_KEY)?;
            if !admins.contains(&sender_raw) {
                return Err(StdError::unauthorized());
            } else {
                MintCaller::Admin
            }
        } else {
            // whitelist can only mint one
            remove(&mut white_store, sender_slice);
            MintCaller::Whitelist
        }
    } else {
        // listing called
        MintCaller::Listing
    };
    let mint_cnt = buyers.len() as u32;
    if let MintCaller::Whitelist = caller_type {
        if mint_cnt != 1 {
            // whitelisted address must mint exactly 1
            return Err(StdError::generic_err(
                "Whitelisted addresses must mint exactly 1 token",
            ));
        }
    }
    let mut counts: Counts = load(&deps.storage, COUNT_KEY)?;
    if mint_cnt > counts.available {
        return Err(StdError::generic_err(format!(
            "Trying to mint {} tokens, but only {} are available",
            mint_cnt, counts.available
        )));
    }
    let mut prng_seed: Vec<u8> = load(&deps.storage, PRNG_SEED_KEY)?;
    let rng_entropy = extend_entropy(env, entropy.as_bytes());
    let mut rng = Prng::new(&prng_seed, &rng_entropy);
    let mut transfers: Vec<Transfer> = Vec::new();
    let mut distributed: Vec<String> = Vec::new();
    let mut id_store = PrefixedStorage::new(PREFIX_TOKEN_IDS, &mut deps.storage);
    // transfer an nft to each buyer
    for buyer in buyers.into_iter() {
        // draw the winning token
        let winner = rng.next_u64() % (counts.available as u64);
        let winner_key = (winner as u32).to_le_bytes();
        let winner_id: String = may_load(&id_store, &winner_key)?
            .ok_or_else(|| StdError::generic_err("Token ID pool is corrupt"))?;
        distributed.push(winner_id.clone());
        if let Some(xfer) = transfers.iter_mut().find(|t| t.recipient == buyer) {
            // if this address is already getting tokens, just add this id to its list
            xfer.token_ids.push(winner_id);
        } else {
            // first one this address is getting
            let memo = if let MintCaller::Listing = caller_type {
                format!("Purchased from listing {}", &env.message.sender)
            } else {
                format!(
                    "Distributed from gumball contract {}",
                    &env.contract.address
                )
            };
            transfers.push(Transfer {
                recipient: buyer,
                token_ids: vec![winner_id],
                memo,
            });
        }
        let last_idx = counts.available - 1;
        let last_key = last_idx.to_le_bytes();
        // swap_remove if the winner is not at the end
        if winner != last_idx as u64 {
            let last: String = may_load(&id_store, &last_key)?
                .ok_or_else(|| StdError::generic_err("Token ID pool is corrupt"))?;
            save(&mut id_store, &winner_key, &last)?;
        }
        remove(&mut id_store, &last_key);
        counts.available = counts.available.saturating_sub(1);
        counts.released = counts.released.saturating_add(1);
    }
    save(&mut deps.storage, COUNT_KEY, &counts)?;
    prng_seed = rng.rand_bytes().to_vec();
    save(&mut deps.storage, PRNG_SEED_KEY, &prng_seed)?;

    let stored: StoreContractInfo = load(&deps.storage, COLLECTION_KEY)?;
    let contract = stored.into_humanized(&deps.api)?;
    let messages = vec![
        Snip721HandleMsg::BatchTransferNft { transfers }.to_cosmos_msg(
            contract.code_hash,
            contract.address,
            None,
        )?,
    ];
    Ok(HandleResponse {
        messages,
        log: vec![log("distributed", format!("{:?}", &distributed))],
        data: None,
    })
}

/// Returns HandleResult
///
/// remove a list of admins from the list
///
/// # Arguments
///
/// * `deps` - a mutable reference to Extern containing all the contract's external dependencies
/// * `sender` - a reference to the message sender
/// * `admins_to_remove` - list of admin addresses to remove
fn try_remove_admins<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    sender: &HumanAddr,
    admins_to_remove: Vec<HumanAddr>,
) -> HandleResult {
    // only allow admins to do this
    let mut admins: Vec<CanonicalAddr> = load(&deps.storage, ADMINS_KEY)?;
    let sender_raw = deps.api.canonical_address(sender)?;
    if !admins.contains(&sender_raw) {
        return Err(StdError::unauthorized());
    }
    let old_len = admins.len();
    let rem_list = admins_to_remove
        .iter()
        .map(|a| deps.api.canonical_address(a))
        .collect::<StdResult<Vec<CanonicalAddr>>>()?;
    admins.retain(|a| !rem_list.contains(a));
    // only save if the list changed
    if old_len != admins.len() {
        save(&mut deps.storage, ADMINS_KEY, &admins)?;
    }
    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::AdminsList {
            admins: admins
                .iter()
                .map(|a| deps.api.human_address(a))
                .collect::<StdResult<Vec<HumanAddr>>>()?,
        })?),
    })
}

/// Returns HandleResult
///
/// adds a list of admins to the list
///
/// # Arguments
///
/// * `deps` - a mutable reference to Extern containing all the contract's external dependencies
/// * `sender` - a reference to the message sender
/// * `admins_to_add` - list of admin addresses to add
fn try_add_admins<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    sender: &HumanAddr,
    admins_to_add: Vec<HumanAddr>,
) -> HandleResult {
    // only allow admins to do this
    let mut admins: Vec<CanonicalAddr> = load(&deps.storage, ADMINS_KEY)?;
    let sender_raw = deps.api.canonical_address(sender)?;
    if !admins.contains(&sender_raw) {
        return Err(StdError::unauthorized());
    }
    let mut save_it = false;
    for admin in admins_to_add.iter() {
        let raw = deps.api.canonical_address(admin)?;
        if !admins.contains(&raw) {
            admins.push(raw);
            save_it = true;
        }
    }
    // only save if the list changed
    if save_it {
        save(&mut deps.storage, ADMINS_KEY, &admins)?;
    }
    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::AdminsList {
            admins: admins
                .iter()
                .map(|a| deps.api.human_address(a))
                .collect::<StdResult<Vec<HumanAddr>>>()?,
        })?),
    })
}

/// Returns HandleResult
///
/// creates a viewing key
///
/// # Arguments
///
/// * `deps` - mutable reference to Extern containing all the contract's external dependencies
/// * `env` - a reference to the Env of contract's environment
/// * `entropy` - string slice of the input String to be used as entropy in randomization
fn try_create_key<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: &Env,
    entropy: &str,
) -> HandleResult {
    let prng_seed: Vec<u8> = load(&deps.storage, PRNG_SEED_KEY)?;
    let key = ViewingKey::new(env, &prng_seed, entropy.as_ref());
    let message_sender = &deps.api.canonical_address(&env.message.sender)?;
    let mut key_store = PrefixedStorage::new(PREFIX_VIEW_KEY, &mut deps.storage);
    save(&mut key_store, message_sender.as_slice(), &key.to_hashed())?;
    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::ViewingKey { key: key.0 })?),
    })
}

/// Returns HandleResult
///
/// sets the viewing key to the input String
///
/// # Arguments
///
/// * `deps` - mutable reference to Extern containing all the contract's external dependencies
/// * `sender` - a reference to the message sender
/// * `key` - String to be used as the viewing key
fn try_set_key<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    sender: &HumanAddr,
    key: String,
) -> HandleResult {
    let vk = ViewingKey(key.clone());
    let message_sender = &deps.api.canonical_address(sender)?;
    let mut key_store = PrefixedStorage::new(PREFIX_VIEW_KEY, &mut deps.storage);
    save(&mut key_store, message_sender.as_slice(), &vk.to_hashed())?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::ViewingKey { key })?),
    })
}

/// Returns HandleResult
///
/// revoke the ability to use a specified permit
///
/// # Arguments
///
/// * `storage` - mutable reference to the contract's storage
/// * `sender` - a reference to the message sender
/// * `permit_name` - string slice of the name of the permit to revoke
fn revoke_permit<S: Storage>(
    storage: &mut S,
    sender: &HumanAddr,
    permit_name: &str,
) -> HandleResult {
    RevokedPermits::revoke_permit(storage, PREFIX_REVOKED_PERMITS, sender, permit_name);

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::RevokePermit {
            status: "success".to_string(),
        })?),
    })
}

/////////////////////////////////////// Query /////////////////////////////////////
/// Returns QueryResult
///
/// # Arguments
///
/// * `deps` - reference to Extern containing all the contract's external dependencies
/// * `msg` - QueryMsg passed in with the query call
pub fn query<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>, msg: QueryMsg) -> QueryResult {
    let response = match msg {
        QueryMsg::Admins { viewer, permit } => query_admins(deps, viewer, permit),
        QueryMsg::NftListingDisplay {} => query_listing_disp(deps),
        QueryMsg::Counts {} => query_counts(&deps.storage),
        QueryMsg::NftContract {} => query_nft_contract(deps),
    };
    pad_query_result(response, BLOCK_SIZE)
}

/// Returns QueryResult displaying the number of NFTs available and the number of NFTs released
///
/// # Arguments
///
/// * `storage` - a reference to the contract's storage
fn query_counts<S: ReadonlyStorage>(storage: &S) -> QueryResult {
    let counts: Counts = load(storage, COUNT_KEY)?;

    to_binary(&QueryAnswer::Counts {
        available: counts.available,
        released: counts.released,
    })
}

/// Returns QueryResult displaying code hash and address of the nft contract this gumball is used with
///
/// # Arguments
///
/// * `deps` - reference to Extern containing all the contract's external dependencies
fn query_nft_contract<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> QueryResult {
    let contract =
        load::<StoreContractInfo, _>(&deps.storage, COLLECTION_KEY)?.into_humanized(&deps.api)?;

    to_binary(&QueryAnswer::NftContract {
        code_hash: contract.code_hash,
        address: contract.address,
    })
}

/// Returns QueryResult displaying an example NFT's public information
///
/// # Arguments
///
/// * `deps` - reference to Extern containing all the contract's external dependencies
fn query_listing_disp<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> QueryResult {
    let contr_strd: StoreContractInfo = load(&deps.storage, COLLECTION_KEY)?;
    let counts: Counts = load(&deps.storage, COUNT_KEY)?;
    let doss_strd: StoredNftDossierForListing =
        may_load::<StoredNftDossierForListing, _>(&deps.storage, EXAMPLE_KEY)?.unwrap_or(
            StoredNftDossierForListing {
                public_metadata: None,
                royalty_info: None,
                mint_run_info: None,
            },
        );
    to_binary(&QueryAnswer::NftListingDisplay {
        nft_info: doss_strd.into_humanized(&deps.api)?,
        nft_contract_address: deps.api.human_address(&contr_strd.address)?,
        mintable: counts.available > 0,
    })
}

/// Returns QueryResult displaying the admin list
///
/// # Arguments
///
/// * `deps` - reference to Extern containing all the contract's external dependencies
/// * `viewer` - optional address and key making an authenticated query request
/// * `permit` - optional permit with "owner" permission
fn query_admins<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    viewer: Option<ViewerInfo>,
    permit: Option<Permit>,
) -> QueryResult {
    // only allow admins to do this
    let (admins, _) = check_admin(deps, viewer, permit)?;
    to_binary(&QueryAnswer::Admins {
        admins: admins
            .iter()
            .map(|a| deps.api.human_address(a))
            .collect::<StdResult<Vec<HumanAddr>>>()?,
    })
}

/// Returns StdResult<(CanonicalAddr, Option<CanonicalAddr>)> from determining the querying address
/// (if possible) either from a Permit or a ViewerInfo.  Also returns this minter's address if
/// a permit was supplied
///
/// # Arguments
///
/// * `deps` - a reference to Extern containing all the contract's external dependencies
/// * `viewer` - optional address and key making an authenticated query request
/// * `permit` - optional permit with "owner" permission
fn get_querier<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    viewer: Option<ViewerInfo>,
    permit: Option<Permit>,
) -> StdResult<(CanonicalAddr, Option<CanonicalAddr>)> {
    if let Some(pmt) = permit {
        // Validate permit content
        let me_raw: CanonicalAddr = may_load(&deps.storage, MY_ADDRESS_KEY)?
            .ok_or_else(|| StdError::generic_err("Minter contract address storage is corrupt"))?;
        let my_address = deps.api.human_address(&me_raw)?;
        let querier = deps.api.canonical_address(&validate(
            deps,
            PREFIX_REVOKED_PERMITS,
            &pmt,
            my_address,
        )?)?;
        if !pmt.check_permission(&secret_toolkit::permit::Permission::Owner) {
            return Err(StdError::generic_err(format!(
                "Owner permission is required for Stashh minter queries, got permissions {:?}",
                pmt.params.permissions
            )));
        }
        return Ok((querier, Some(me_raw)));
    }
    if let Some(vwr) = viewer {
        let raw = deps.api.canonical_address(&vwr.address)?;
        // load the address' key
        let key_store = ReadonlyPrefixedStorage::new(PREFIX_VIEW_KEY, &deps.storage);
        let load_key: [u8; VIEWING_KEY_SIZE] =
            may_load(&key_store, raw.as_slice())?.unwrap_or_else(|| [0u8; VIEWING_KEY_SIZE]);
        let input_key = ViewingKey(vwr.viewing_key);
        // if key matches
        if input_key.check_viewing_key(&load_key) {
            return Ok((raw, None));
        }
    }
    Err(StdError::unauthorized())
}

/// Returns StdResult<(Vec<CanonicalAddr>, Option<CanonicalAddr>)> which is the admin list
/// and this contract's address if it has been retrieved, and checks if the querier is an admin
///
/// # Arguments
///
/// * `deps` - a reference to Extern containing all the contract's external dependencies
/// * `viewer` - optional address and key making an authenticated query request
/// * `permit` - optional permit with "owner" permission
fn check_admin<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    viewer: Option<ViewerInfo>,
    permit: Option<Permit>,
) -> StdResult<(Vec<CanonicalAddr>, Option<CanonicalAddr>)> {
    let (admin, my_addr) = get_querier(deps, viewer, permit)?;
    // only allow admins to do this
    let admins: Vec<CanonicalAddr> = load(&deps.storage, ADMINS_KEY)?;
    if !admins.contains(&admin) {
        return Err(StdError::unauthorized());
    }
    Ok((admins, my_addr))
}
