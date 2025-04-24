use teloxide::RequestError;
use teloxide::adaptors::Throttle;
use teloxide::prelude::*;
use teloxide::types::ChatId;

// --- Service Definition ---
#[derive(Clone)]
pub struct TelegramService {
    bot: Throttle<Bot>, // Bot instance for actual API calls
    group_id: i64,      // Stores the default group_id in the service
}

impl TelegramService {
    /// Create a new TelegramService instance.
    ///  
    /// # Panics  
    /// * If `group_id_str` cannot be parsed as i64.  
    pub fn new() -> Self {
        // Telegram Bot Token.
        let bot_token =
            std::env::var("TELEGRAM_BOT_TOKEN").expect("Can not read TELEGRAM_BOT_TOKEN env");
        // Target group ID (string form, e.g., "-1001234567890").
        let group_id_str =
            std::env::var("TELEGRAM_GROUP_ID").expect("Can not read TELEGRAM_GROUP_ID env");
        let bot = Bot::new(bot_token).throttle(Default::default());
        let group_id: i64 = group_id_str
            .parse()
            .expect("Invalid Group ID format, should be i64 number");

        TelegramService { bot, group_id }
    }

    /// Send a message to the configured default group.  
    ///  
    /// # Arguments  
    /// * `message` - Message text to send.  
    ///  
    /// # Returns  
    /// * `Ok(())` - If the message is sent successfully.  
    /// * `Err(RequestError)` - If there is an error during sending.  
    #[allow(unused)]
    pub async fn send_message(&self, message: &str) -> Result<(), RequestError> {
        self.bot
            .send_message(ChatId(self.group_id), message)
            // Can chain more options, like setting parse mode:
            // .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .await?;
        Ok(())
    }

    /// Send a message to a specified group ID (if sending to a different group is needed).  
    ///  
    /// # Arguments  
    /// * `target_group_id` - Target group ID (i64).  
    /// * `message` - Message text to send.  
    ///  
    /// # Returns  
    /// * `Ok(())` - If the message is sent successfully.  
    /// * `Err(RequestError)` - If there is an error during sending.
    #[allow(unused)]
    pub async fn send_message_to_group(
        &self,
        target_group_id: i64,
        message: &str,
    ) -> Result<(), RequestError> {
        self.bot
            .send_message(ChatId(target_group_id), message)
            .await?;
        Ok(())
    }
}

// --- Example Usage (Multithreading/Multitasking) ---
#[tokio::test]
async fn test_send_msg() {
    use std::sync::Arc;

    // --- Configuration ---
    env_logger::init();
    dotenv::dotenv().ok();

    // Create service instance
    let telegram_service = Arc::new(TelegramService::new());
    // Using Arc wrapper for easy ownership sharing between async tasks
    // If tasks are spawned from same scope, directly cloning telegram_service also works

    println!("Telegram service initialized.");

    // --- Concurrent call example ---
    let mut tasks = vec![];

    for i in 0..3 {
        let service_clone = Arc::clone(&telegram_service); // Clone Arc pointer (cheap)  
        let task = tokio::spawn(async move {
            let message = format!("Message from task {}!", i + 1);
            match service_clone.send_message(&message).await {
                Ok(_) => println!("Task {}: Message sent successfully", i + 1),
                Err(e) => eprintln!("Task {}: Send failed: {}", i + 1, e),
            }
            // Example: Send to another group (assuming ID is -1009876543210)
            // match service_clone.send_message_to_group(-1009876543210, &message).await {
            //     Ok(_) => println!("Task {}: Message sent to another group successfully", i + 1),
            //     Err(e) => eprintln!("Task {}: Send to another group failed: {}", i + 1, e),
            // }
        });
        tasks.push(task);
    }

    // Wait for all send tasks to complete
    for task in tasks {
        let _ = task.await; // Handle join result (ignored here)  
    }

    println!("All sending tasks completed.");
}
