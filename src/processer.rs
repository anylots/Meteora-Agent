use carbon_meteora_dlmm_decoder::instructions::remove_liquidity::RemoveLiquidity;
use log::{debug, warn};
use solana_sdk::pubkey::Pubkey;

use crate::{
    message::TelegramService,
    token::get_token_metadata,
    utils::{CLIENT_ACCOUNT_FILTERING, LP_WALLETS},
};
use {
    async_trait::async_trait,
    carbon_core::{
        deserialize::ArrangeAccounts,
        error::CarbonResult,
        instruction::{DecodedInstruction, InstructionMetadata, NestedInstructions},
        metrics::MetricsCollection,
        processor::Processor,
    },
    carbon_meteora_dlmm_decoder::instructions::{
        MeteoraDlmmInstruction, add_liquidity::AddLiquidity, swap::Swap,
    },
    log::{error, info},
    std::sync::Arc,
};
/// Processor for Meteora DLMM instructions
pub struct MeteoraInstructionProcessor {
    telegram_service: Arc<TelegramService>,
}

impl MeteoraInstructionProcessor {
    pub fn new(telegram_service: Arc<TelegramService>) -> Self {
        Self { telegram_service }
    }
}

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
        let fee_payer = transaction_metadata.fee_payer;
        let account_keys = transaction_metadata.message.static_account_keys();
        if *CLIENT_ACCOUNT_FILTERING && !check_accounts_in_client(fee_payer, account_keys) {
            warn!("  CLIENT_ACCOUNT_FILTERING, check_accounts_in_client false");
            return Ok(());
        }

        // Check if fee_payer is in LP_WALLETS

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
            MeteoraDlmmInstruction::AddLiquidity(_liquidity_parameter) => {
                let accounts = AddLiquidity::arrange_accounts(&decoded_instruction.accounts);
                if let Some(accounts) = accounts {
                    info!("AddLiquidity Instruction details:");
                    let token_x = accounts.token_x_mint;
                    let token_y = accounts.token_y_mint;
                    // fetch token metadata
                    match get_token_metadata(token_x).await {
                        Ok((_, symbol)) => {
                            info!("  symbol_x: {}", symbol);
                        }
                        Err(e) => {
                            error!("  Failed to fetch token_x metadata: {}", e);
                        }
                    };

                    match get_token_metadata(token_y).await {
                        Ok((_, symbol)) => {
                            info!("  symbol_y: {}", symbol);
                        }
                        Err(e) => {
                            error!("  Failed to fetch token_y metadata: {}", e);
                        }
                    };
                    let amount_x = _liquidity_parameter.liquidity_parameter.amount_x;
                    info!("  amount_x: {}", amount_x);
                    let amount_y = _liquidity_parameter.liquidity_parameter.amount_x;
                    info!("  amount_y: {}", amount_y);
                }
            }
            MeteoraDlmmInstruction::RemoveLiquidity(_liquidity_parameter) => {
                let accounts = RemoveLiquidity::arrange_accounts(&decoded_instruction.accounts);
                if let Some(accounts) = accounts {
                    info!("RemoveLiquidity Instruction details:");
                    let token_x = accounts.token_x_mint;
                    let token_y = accounts.token_y_mint;
                    // fetch token metadata
                    match get_token_metadata(token_x).await {
                        Ok((_, symbol)) => {
                            info!("  symbol_x: {}", symbol);
                        }
                        Err(e) => {
                            error!("  Failed to fetch token_x metadata: {}", e);
                        }
                    };

                    match get_token_metadata(token_y).await {
                        Ok((_, symbol)) => {
                            info!("  symbol_y: {}", symbol);
                        }
                        Err(e) => {
                            error!("  Failed to fetch token_y metadata: {}", e);
                        }
                    };
                    let bin_liquidity_removal = &_liquidity_parameter.bin_liquidity_removal;
                    info!(
                        "  bin_liquidity_removal_len: {}",
                        bin_liquidity_removal.len()
                    );
                }
            }
            MeteoraDlmmInstruction::Swap(swap_parameters) => {
                let accounts = Swap::arrange_accounts(&decoded_instruction.accounts);
                if let Some(accounts) = accounts {
                    info!("=======>Swap Instruction details:");
                    let token_x = accounts.token_x_mint;
                    let token_y = accounts.token_y_mint;
                    // fetch token metadata
                    let symbol_x = match get_token_metadata(token_x).await {
                        Ok((_, symbol)) => {
                            info!("  token_x: {}", symbol);
                            Some(symbol)
                        }
                        Err(e) => {
                            error!("  Failed to fetch user_token_in metadata: {}", e);
                            None
                        }
                    };
                    let symbol_y = match get_token_metadata(token_y).await {
                        Ok((_, symbol)) => {
                            info!("  token_y: {}", symbol);
                            Some(symbol)
                        }
                        Err(e) => {
                            error!("  Failed to fetch token_y metadata: {}", e);
                            None
                        }
                    };
                    let amount_in = swap_parameters.amount_in;
                    info!("  amount_in: {}", amount_in);
                    let min_amount_out = swap_parameters.min_amount_out;
                    info!("  min_amount_out: {}", min_amount_out);

                    if symbol_x.is_some() && symbol_y.is_some() {
                        let message = format!(
                            "Swap Instruction:\nToken X: {}\nToken Y: {}\nAmount In: {}\nMin Amount Out: {}",
                            symbol_x.unwrap(),
                            symbol_y.unwrap(),
                            amount_in,
                            min_amount_out
                        );
                        self.telegram_service.send_message(&message).await.unwrap();
                    }
                }
            }
            _ => {
                info!(
                    "Instruction type: {}",
                    get_instruction_name(&decoded_instruction.data)
                );
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
fn check_accounts_in_client(fee_payer: Pubkey, account_keys: &[Pubkey]) -> bool {
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

        if fee_payer_is_lp {
            info!("LP fee_payer: {}", fee_payer);
        }

        if !lp_account_keys.is_empty() {
            info!("LP account keys:");
            for acc in &lp_account_keys {
                info!("  - {}", acc);
            }
        }
        return true;
    }
    return false;
}
