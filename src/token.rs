use mpl_token_metadata::ID as TOKEN_METADATA_PROGRAM_ID;
use mpl_token_metadata::accounts::Metadata;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use thiserror::Error;

use crate::utils::SOLANA_RPC;

#[derive(Error, Debug)]
pub enum FetchMetadataError {
    #[error("Solana RPC client error: {0}")]
    RpcClientError(#[from] solana_client::client_error::ClientError),
    #[error("Invalid public key string: {0}")]
    InvalidPublicKey(#[from] solana_sdk::pubkey::ParsePubkeyError),
    #[error("Failed to deserialize metadata account: {0}")]
    DeserializationError(#[from] std::io::Error), // Borsh deserialize error wraps io::Error,
}

pub async fn get_token_metadata(
    mint_pubkey: Pubkey,
) -> Result<(String, String), FetchMetadataError> {
    // 1. Create RPC client
    let rpc_client = RpcClient::new(SOLANA_RPC.to_string());

    // 2. Calculate Metadata PDA
    // Seeds for Metaplex Token Metadata PDA are "metadata", program ID, mint Pubkey
    let metadata_seeds = &[
        b"metadata".as_ref(),
        TOKEN_METADATA_PROGRAM_ID.as_ref(),
        mint_pubkey.as_ref(),
    ];
    let (metadata_pda, _bump_seed) =
        Pubkey::find_program_address(metadata_seeds, &TOKEN_METADATA_PROGRAM_ID);
    log::debug!("Derived Metadata PDA: {}", metadata_pda);

    // 3. Get Metadata account information
    let metadata_account = rpc_client.get_account(&metadata_pda);

    let account_data = match metadata_account {
        Ok(account) => {
            // Check if account owner is the Token Metadata Program (optional but recommended)
            if account.owner != TOKEN_METADATA_PROGRAM_ID {
                log::warn!(
                    "Warning: Account owner ({}) is not the Token Metadata Program ID ({}).",
                    account.owner,
                    TOKEN_METADATA_PROGRAM_ID
                );
            }
            account.data
        }
        Err(e) => {
            // Other RPC errors
            return Err(FetchMetadataError::RpcClientError(e));
        }
    };

    // 4. Deserialize account data
    // Metaplex's Metadata structure implements BorshDeserialize
    let metadata = Metadata::from_bytes(&account_data)?;

    // 5. Extract Name and Symbol
    // Note: Borsh serialized strings may have null bytes \0 at the end that need to be removed
    let name = metadata.name.trim_end_matches('\0').to_string();
    let symbol = metadata.symbol.trim_end_matches('\0').to_string();

    Ok((name, symbol))
}

#[tokio::test(flavor = "multi_thread")]
async fn test() {
    use std::str::FromStr;

    // --- Configuration ---
    env_logger::init();
    dotenv::dotenv().ok();

    // Replace with the Mint address of the SPL Token you want to query
    let mint_address = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"; // USDC Mint Address example  

    log::info!("Querying metadata for mint: {}", mint_address);

    // --- Execute query ---
    match get_token_metadata(Pubkey::from_str(mint_address).unwrap()).await {
        Ok((name, symbol)) => {
            log::info!("-------------------------");
            log::info!("Token Name:   {}", name);
            log::info!("Token Symbol: {}", symbol);
            log::info!("-------------------------");
        }
        Err(e) => {
            log::error!("Error fetching token metadata: {}", e);
        }
    }
}
