use std::borrow::Cow;
use std::env;
use std::sync::LazyLock;

use regex::Regex;
use serenity::all::{
    Command, CommandType, CreateCommand, CreateInteractionResponse,
    CreateInteractionResponseMessage, EmojiId, GuildId, Interaction, ReactionType,
};
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;

const BEAR_EMOJI_ID: u64 = 1519568644827250749;
const SIX_SEVEN_EMOJI_ID: u64 = 1495252839918993569;

const SWAG_COMMAND: &str = "swag";

/// Matches `scheme://...` links, `www.` addresses and scheme-less `domain.tld/path` links.
/// A domain with no path isn't matched, so "bear.com" or a typo like "bears.they" still
/// counts as text. A link ends at whitespace or an angle bracket, so `<https://...>` is
/// covered too.
static URL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)[a-z][a-z0-9+-]*://[^\s<>]*|\bwww\.[^\s<>]+|\b[a-z0-9-]+(?:\.[a-z0-9-]+)*\.[a-z]{2,}/[^\s<>]*",
    )
    .expect("URL regex is valid")
});

/// Blanks out URLs, whose paths and hostnames shouldn't count as someone saying a word.
/// Each one becomes a space so the text on either side can't join into a false match.
fn strip_urls(text: &str) -> Cow<'_, str> {
    URL.replace_all(text, " ")
}

/// Returns true if the text outside URLs contains "bear" or "ursus" in any casing,
/// including common look-alike spellings ("b3ar", "be@r", "BeAr", "bears", "teddybear")
/// or the 🐻 emoji.
fn mentions_bear(text: &str) -> bool {
    let text = strip_urls(text);
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
    normalized.contains("bear") || normalized.contains("ursus")
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

/// Returns true if the visible text outside URLs contains "67", "6 7" or "six seven"
/// in any casing.
fn mentions_six_seven(text: &str) -> bool {
    let visible = strip_discord_markup(&strip_urls(text)).to_lowercase();
    visible.contains("67") || visible.contains("6 7") || visible.contains("six seven")
}

/// Deletes guild-scoped copies of /swag. Discord lists guild and global commands side by
/// side, so a copy left by an older registration shows up as a second /swag.
async fn remove_guild_swag_commands(ctx: &Context, guild_id: GuildId) {
    let commands = match guild_id.get_commands(&ctx.http).await {
        Ok(commands) => commands,
        Err(why) => {
            eprintln!("Failed to list commands for guild {guild_id}: {why:?}");
            return;
        }
    };
    for command in commands
        .iter()
        .filter(|c| c.kind == CommandType::ChatInput && c.name == SWAG_COMMAND)
    {
        match guild_id.delete_command(&ctx.http, command.id).await {
            Ok(()) => println!("Removed duplicate /swag from guild {guild_id}"),
            Err(why) => eprintln!("Failed to remove /swag from guild {guild_id}: {why:?}"),
        }
    }
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
        if command.data.name == SWAG_COMMAND {
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
        let swag = CreateCommand::new(SWAG_COMMAND).description("Swag");
        match Command::create_global_command(&ctx.http, swag).await {
            Ok(_) => println!("Registered /swag"),
            Err(why) => {
                // Keep any guild copies: they'd be the only working /swag left.
                eprintln!("Failed to register /swag: {why:?}");
                return;
            }
        }
        for guild in &ready.guilds {
            remove_guild_swag_commands(&ctx, guild.id).await;
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
            "<:bear:123>",
            "ursus",
            "URSUS",
            "Ursus arctos",
            "bear https://example.com",
            "https://example.com bear",
            "ursus https://example.com",
            "[bear](https://example.com)",
            "https://example.com/bear bear",
            "bear.https://example.com",
            "bear.com",
            "bears.they rock",
        ] {
            assert!(mentions_bear(s), "{s}");
        }
    }

    #[test]
    fn ignores_non_bear() {
        for s in [
            "hello",
            "bee",
            "b e a r",
            "bar",
            "urs us",
            "https://example.com/bear",
            "HTTPS://BEAR.COM",
            "<https://bear.com>",
            "[link](https://bear.com)",
            "www.bear.com/cubs",
            "https://b3ar.io",
            "https://example.com/🐻",
            "https://ursus.example.com",
            "steam://run/bear",
            "discord.gg/bear",
            "www.example.com/bear",
            "b3ar.io/cubs",
        ] {
            assert!(!mentions_bear(s), "{s}");
        }
    }

    #[test]
    fn detects_six_seven() {
        for s in [
            "67",
            "6 7",
            "lol 67 lol",
            "1670",
            "<@123> 67",
            "a < b 67",
            "six seven",
            "SIX Seven",
            "lol six seven lol",
            "67 https://example.com",
            "https://example.com 67",
            "[67](https://example.com)",
            "https://example.com/67 6 7",
            "awww.67",
            "3.14/67",
            "and/or 67",
            "67.com",
        ] {
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
            "six",
            "seven",
            "seven six",
            "<@126789>",
            "<#167>",
            "<:smile:1670>",
            "<a:wave:1670>",
            "<t:1767225600:R>",
            "https://x.com/user/status/1867",
            "HTTP://EXAMPLE.COM/67",
            "<https://example.com/67>",
            "[link](https://example.com/67)",
            "https://tenor.com/view/six seven",
            "https://tenor.com/view/6-7-gif-67",
            "www.example.com/67",
            "6https://example.com 7",
            "twitter.com/user/status/1867",
            "sub.example.co.uk/67",
        ] {
            assert!(!mentions_six_seven(s), "{s}");
        }
    }
}
