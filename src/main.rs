use std::env;

use serenity::all::{
    Command, CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage, EmojiId,
    Interaction, ReactionType,
};
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;

const BEAR_EMOJI_ID: u64 = 1519568644827250749;
const SIX_SEVEN_EMOJI_ID: u64 = 1495252839918993569;

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

/// Removes mentions, custom emoji, timestamps and similar markup from raw message
/// content. Their IDs aren't visible to users, so they shouldn't count as text.
fn strip_discord_markup(text: &str) -> String {
    const MARKUP_PREFIXES: [&str; 7] = ["<@", "<#", "<:", "<a:", "<t:", "</", "<id:"];
    let mut stripped = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('<') {
        stripped.push_str(&rest[..start]);
        let tail = &rest[start..];
        let is_markup = MARKUP_PREFIXES
            .iter()
            .any(|prefix| tail.starts_with(prefix));
        match tail.find('>') {
            Some(end) if is_markup => rest = &tail[end + 1..],
            _ => {
                stripped.push('<');
                rest = &tail[1..];
            }
        }
    }
    stripped.push_str(rest);
    stripped
}

/// Returns true if the visible text contains "67" or "6 7".
fn mentions_six_seven(text: &str) -> bool {
    let visible = strip_discord_markup(text);
    visible.contains("67") || visible.contains("6 7")
}

async fn react(ctx: &Context, msg: &Message, emoji: ReactionType) {
    if let Err(why) = msg.react(&ctx.http, emoji).await {
        eprintln!("Failed to react to message {}: {why:?}", msg.id);
    }
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }
        if mentions_bear(&msg.content) {
            let bear = ReactionType::Custom {
                animated: false,
                id: EmojiId::new(BEAR_EMOJI_ID),
                name: Some("Bear".to_string()),
            };
            react(&ctx, &msg, bear).await;
        }
        if mentions_six_seven(&msg.content) {
            let six_seven = ReactionType::Custom {
                animated: true,
                id: EmojiId::new(SIX_SEVEN_EMOJI_ID),
                name: Some("67".to_string()),
            };
            react(&ctx, &msg, six_seven).await;
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
    use super::{mentions_bear, mentions_six_seven};

    #[test]
    fn detects_bear_variants() {
        for s in [
            "bear",
            "BEAR",
            "Bears are cool",
            "teddybear",
            "b3ar",
            "be@r",
            "8ear",
            "🐻",
        ] {
            assert!(mentions_bear(s), "{s}");
        }
    }

    #[test]
    fn ignores_non_bear() {
        for s in ["hello", "bee", "b e a r", "bar"] {
            assert!(!mentions_bear(s), "{s}");
        }
    }

    #[test]
    fn detects_six_seven() {
        for s in ["67", "6 7", "lol 67 lol", "1670", "<@123> 67", "a < b 67"] {
            assert!(mentions_six_seven(s), "{s}");
        }
    }

    #[test]
    fn ignores_non_six_seven() {
        for s in [
            "hello",
            "6",
            "76",
            "6  7",
            "6-7",
            "<@126789>",
            "<#167>",
            "<:smile:1670>",
            "<a:wave:1670>",
            "<t:1767225600:R>",
        ] {
            assert!(!mentions_six_seven(s), "{s}");
        }
    }
}
