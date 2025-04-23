// Commented imports removed for cleaner code
use {
    async_trait::async_trait,
    carbon_core::{
        error::CarbonResult,
        instruction::{DecodedInstruction, InstructionMetadata, NestedInstructions},
        metrics::MetricsCollection,
        processor::Processor,
    },
    carbon_log_metrics::LogMetrics,
    carbon_meteora_dlmm_decoder::{
        MeteoraDlmmDecoder, PROGRAM_ID as METEORA_PROGRAM_ID, instructions::MeteoraDlmmInstruction,
    },
    carbon_rpc_transaction_crawler_datasource::{Filters, RpcTransactionCrawler},
    log::{debug, error, info},
    once_cell::sync::Lazy,
    serde::{Deserialize, Serialize},
    solana_sdk::commitment_config::CommitmentConfig,
    std::{env, fs::File, io::BufReader, sync::Arc, time::Duration},
};

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

/// Global static collection of LP wallet addresses
static LP_WALLETS: Lazy<Vec<String>> = Lazy::new(|| read_lp_wallets_config("config.json"));

/// Main application entry point
#[tokio::main]
pub async fn main() -> CarbonResult<()> {
    // Initialize logging and environment variables
    env_logger::init();
    dotenv::dotenv().ok();

    info!("Starting Meteora DLMM transaction processor");

    // Configure transaction crawler
    let filters = Filters::new(None, None, None);
    let transaction_crawler = RpcTransactionCrawler::new(
        env::var("RPC_URL").unwrap_or_default(), // RPC URL
        METEORA_PROGRAM_ID,                      // Program ID to monitor
        10,                                      // Batch limit
        Duration::from_secs(5),                  // Polling interval
        filters,                                 // Filters
        Some(CommitmentConfig::finalized()),     // Commitment config
        1,                                       // Max Concurrent Requests
    );

    info!("Configured transaction crawler for Meteora DLMM program");

    // Build and run the processing pipeline
    carbon_core::pipeline::Pipeline::builder()
        .datasource(transaction_crawler)
        .metrics(Arc::new(LogMetrics::new()))
        .metrics_flush_interval(3)
        .instruction(MeteoraDlmmDecoder, MeteoraInstructionProcessor)
        .build()?
        .run()
        .await?;

    info!("Pipeline completed successfully");
    Ok(())
}

/// Processor for Meteora DLMM instructions
pub struct MeteoraInstructionProcessor;

#[async_trait]
impl Processor for MeteoraInstructionProcessor {
    type InputType = (
        InstructionMetadata,
        DecodedInstruction<MeteoraDlmmInstruction>,
        NestedInstructions,
    );

    async fn process(
        &mut self,
        data: Self::InputType,
        _metrics: Arc<MetricsCollection>,
    ) -> CarbonResult<()> {
        let (_instruction_metadata, decoded_instruction, _nested_instructions) = data;

        debug!(
            "Decoded instruction data: {}",
            serde_json::to_string(&decoded_instruction.data)
                .unwrap_or("json decode error".to_string())
        );

        let transaction_metadata = &_instruction_metadata.transaction_metadata;
        let account_keys = transaction_metadata.message.static_account_keys();
        let fee_payer = transaction_metadata.fee_payer;

        // Check if fee_payer is in LP_WALLETS
        let fee_payer_is_lp = LP_WALLETS
            .iter()
            .any(|wallet| wallet == &fee_payer.to_string());

        // Check if any account_key is in LP_WALLETS
        let mut lp_account_keys = Vec::new();
        for acc in account_keys {
            if LP_WALLETS.iter().any(|wallet| wallet == &acc.to_string()) {
                lp_account_keys.push(acc);
            }
        }

        // If fee_payer or any account_key is in LP_WALLETS, log information
        if fee_payer_is_lp || !lp_account_keys.is_empty() {
            info!("LP wallet detected in transaction!");
            info!("Transaction signature: {}", transaction_metadata.signature);

            if fee_payer_is_lp {
                info!("LP fee_payer: {}", fee_payer);
            }

            if !lp_account_keys.is_empty() {
                info!("LP account keys:");
                for acc in &lp_account_keys {
                    info!("  - {}", acc);
                }
            }

            match &decoded_instruction.data {
                MeteoraDlmmInstruction::AddLiquidityEvent(event) => {
                    info!("AddLiquidityEvent details:");
                    info!("  lb_pair: {}", event.lb_pair);
                    info!("  from: {}", event.from);
                    info!("  position: {}", event.position);
                    info!("  amounts: [{}, {}]", event.amounts[0], event.amounts[1]);
                    info!("  active_bin_id: {}", event.active_bin_id);
                }
                MeteoraDlmmInstruction::RemoveLiquidityEvent(event) => {
                    info!("RemoveLiquidityEvent details:");
                    info!("  lb_pair: {}", event.lb_pair);
                    info!("  from: {}", event.from);
                    info!("  position: {}", event.position);
                    info!("  amounts: [{}, {}]", event.amounts[0], event.amounts[1]);
                    info!("  active_bin_id: {}", event.active_bin_id);
                }
                _ => {
                    info!(
                        "Instruction type: {}",
                        get_instruction_name(&decoded_instruction.data)
                    );
                }
            }
        }

        if let Some(_inner_instructions) = &transaction_metadata.meta.inner_instructions {
            info!("Transaction signature: {}", transaction_metadata.signature);
        } else {
            info!("This transaction has no inner instructions");
        }

        // Helper function to get the instruction name without listing all types
        fn get_instruction_name(instruction: &MeteoraDlmmInstruction) -> String {
            match instruction {
                MeteoraDlmmInstruction::AddLiquidityEvent(_) => "AddLiquidityEvent".to_string(),
                MeteoraDlmmInstruction::RemoveLiquidityEvent(_) => {
                    "RemoveLiquidityEvent".to_string()
                }
                _ => format!("{:?}", instruction)
                    .split("(")
                    .next()
                    .unwrap_or("Unknown")
                    .to_string(),
            }
        }

        Ok(())
    }
}
