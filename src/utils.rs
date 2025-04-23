use once_cell::sync::Lazy;
use std::env;

pub static SOLANA_RPC: Lazy<String> = Lazy::new(|| env::var("SOLANA_RPC").unwrap_or_default());
