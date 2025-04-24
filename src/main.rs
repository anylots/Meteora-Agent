mod message;
mod processer;
mod token;
mod utils;
use {
    anyhow::Result,
    carbon_meteora_dlmm_decoder::{MeteoraDlmmDecoder, PROGRAM_ID as METEORA_PROGRAM_ID},
    carbon_rpc_transaction_crawler_datasource::{Filters, RpcTransactionCrawler},
    log::info,
    message::TelegramService,
    processer::MeteoraInstructionProcessor,
    solana_sdk::commitment_config::CommitmentConfig,
    std::{sync::Arc, time::Duration},
    utils::SOLANA_RPC,
};

/// Main application entry point
#[tokio::main]
pub async fn main() -> Result<()> {
    // Step1. Initialize logging and environment variables
    dotenv::dotenv().ok();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    info!("Starting Meteora DLMM transaction processor");

    // Step2. Configure transaction crawler
    let filters = Filters::new(None, None, None);
    let transaction_crawler = RpcTransactionCrawler::new(
        SOLANA_RPC.to_string(),              // RPC URL
        METEORA_PROGRAM_ID,                  // Program ID to monitor
        10,                                  // Batch limit
        Duration::from_secs(5),              // Polling interval
        filters,                             // Filters
        Some(CommitmentConfig::finalized()), // Commitment config
        1,                                   // Max Concurrent Requests
    );
    info!("Configured transaction crawler for Meteora DLMM program");

    // Step3. Build and run the processing pipeline
    carbon_core::pipeline::Pipeline::builder()
        .datasource(transaction_crawler)
        .metrics_flush_interval(3)
        .instruction(
            MeteoraDlmmDecoder,
            MeteoraInstructionProcessor::new(Arc::new(TelegramService::new())),
        )
        .build()?
        .run()
        .await?;

    info!("Pipeline completed successfully");
    Ok(())
}
