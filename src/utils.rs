use log::{error, info};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{env, fs::File, io::BufReader};

pub static SOLANA_RPC: Lazy<String> = Lazy::new(|| env::var("SOLANA_RPC").unwrap_or_default());
pub static CLIENT_ACCOUNT_FILTERING: Lazy<bool> = Lazy::new(|| {
    env::var("CLIENT_ACCOUNT_FILTERING")
        .unwrap_or_default()
        .parse()
        .unwrap_or_default()
});

/// Global static collection of LP wallet addresses
pub static LP_WALLETS: Lazy<Vec<String>> = Lazy::new(|| read_lp_wallets_config("config.json"));
/// Configuration structure for LP wallets
#[derive(Debug, Serialize, Deserialize)]
struct Config {
    lp_wallets: Vec<String>,
}

/// Reads LP wallet addresses from the configuration file
fn read_lp_wallets_config(config_path: &str) -> Vec<String> {
    match File::open(config_path) {
        Ok(file) => {
            let reader = BufReader::new(file);
            match serde_json::from_reader::<_, Config>(reader) {
                Ok(config) => {
                    info!("Loaded {} LP wallets from config", config.lp_wallets.len());
                    config.lp_wallets
                }
                Err(e) => {
                    error!("Error parsing config file: {}", e);
                    Vec::new()
                }
            }
        }
        Err(e) => {
            error!("Error opening config file: {}", e);
            Vec::new()
        }
    }
}
