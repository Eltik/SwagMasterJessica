use std::env;

use serenity::all::{
    Command, CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage,
    EmojiId, Interaction, ReactionType,
};
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;

const BEAR_EMOJI_ID: u64 = 1519568644827250749;

/// Returns true if the text contains "bear" in any casing, including common
/// look-alike spellings ("b3ar", "be@r", "BeAr", "bears", "teddybear") or the 🐻 emoji.
fn mentions_bear(text: &str) -> bool {
    if text.contains('🐻') {
        return true;
    }
    let normalized: String = text
        .chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            '3' => 'e',
            '4' | '@' => 'a',
            '8' => 'b',
            _ => c,
        })
        .collect();
    normalized.contains("bear")
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot || !mentions_bear(&msg.content) {
            return;
        }
        let bear = ReactionType::Custom {
            animated: false,
            id: EmojiId::new(BEAR_EMOJI_ID),
            name: Some("Bear".to_string()),
        };
        if let Err(why) = msg.react(&ctx.http, bear).await {
            eprintln!("Failed to react to message {}: {why:?}", msg.id);
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let Interaction::Command(command) = interaction else {
            return;
        };
        if command.data.name == "swag" {
            let response = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().content("Swag"),
            );
            if let Err(why) = command.create_response(&ctx.http, response).await {
                eprintln!("Failed to respond to /swag: {why:?}");
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected", ready.user.name);
        let swag = CreateCommand::new("swag").description("Swag");
        match Command::create_global_command(&ctx.http, swag).await {
            Ok(_) => println!("Registered /swag"),
            Err(why) => eprintln!("Failed to register /swag: {why:?}"),
        }
    }
}

#[tokio::main]
async fn main() {
    // Load .env if present; real environment variables take precedence.
    dotenvy::dotenv().ok();
    let token = env::var("DISCORD_TOKEN").expect("Set DISCORD_TOKEN in .env or the environment");
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Failed to create client");

    if let Err(why) = client.start().await {
        eprintln!("Client error: {why:?}");
    }
}

#[cfg(test)]
mod tests {
    use super::mentions_bear;

    #[test]
    fn detects_bear_variants() {
        for s in ["bear", "BEAR", "Bears are cool", "teddybear", "b3ar", "be@r", "8ear", "🐻"] {
            assert!(mentions_bear(s), "{s}");
        }
    }

    #[test]
    fn ignores_non_bear() {
        for s in ["hello", "bee", "b e a r", "bar"] {
            assert!(!mentions_bear(s), "{s}");
        }
    }
}
