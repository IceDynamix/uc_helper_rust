//! Discord bot side of things
//!
//! There will be no documentation on defined functions, only for commands
//!
//! # Example
//!
//! ```
//! use uc_helper_rust as uc;
//!
//! let db = uc::database::connect().expect("Failed to connect to database");
//! let mut bot = uc::discord::new_client(db).await;
//!     if let Err(why) = bot.start().await {
//!     println!("Client error: {:?}", why);
//! }
//! ```

use std::collections::HashSet;
use std::sync::Arc;

use serenity::client::bridge::gateway::GatewayIntents;
use serenity::framework::standard::{
    help_commands,
    macros::{group, help, hook},
    Args, CommandGroup, CommandResult, HelpOptions,
};
use serenity::http::Http;
use serenity::model::prelude::*;
use serenity::{
    async_trait, client::bridge::gateway::ShardManager, framework::StandardFramework,
    model::gateway::Ready, prelude::*,
};
use tracing::{info, warn};

use crate::commands::{owner::*, player::*, staff::*, tournament::*};
use crate::database::LocalDatabase;

pub const PREFIX: &str = ".";
pub const CONFIRM_EMOJI: &str = "✅";
pub const ERROR_EMOJI: &str = "❌";

#[group]
#[commands(owner_ping, owner_echo)]
#[owners_only]
struct Owner;

#[group]
#[commands(staff_ping, update_all)]
#[allowed_roles("staff")]
#[help_available(false)]
#[only_in(guilds)]
#[description("Management commands restricted to staff members")]
struct Staff;

#[group]
#[commands(faq, stats, link, unlink)]
#[description("Tetr.io player related commands")]
struct Player;

#[group]
#[commands(register, unregister, add_snapshot)]
#[prefix("tournament")]
#[only_in(guilds)]
#[description("Tournament related commands")]
struct Tournament;

pub async fn new_client(database: LocalDatabase) -> Client {
    let token = std::env::var("DISCORD_TOKEN").expect("No Discord token");
    let owners = get_bot_owners(&token).await;
    let framework = create_framework(owners);

    let client = Client::builder(&token)
        .event_handler(Handler)
        .framework(framework)
        .intents(
            GatewayIntents::GUILDS
                | GatewayIntents::GUILD_MESSAGES
                | GatewayIntents::DIRECT_MESSAGES,
        )
        .await
        .expect("Couldn't create client");

    setup_shared_data(database, &client).await;
    setup_ctrl_c(&client);

    client
}

fn setup_ctrl_c(client: &Client) {
    let shard_manager = client.shard_manager.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        shard_manager.lock().await.shutdown_all().await;
    });
}

async fn get_bot_owners(token: &str) -> HashSet<UserId> {
    let http = Http::new_with_token(&token);

    // We will fetch your bots owners and id
    let (owners, _bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            owners.insert(info.owner.id);
            owners.insert(UserId(287102784954695680)); // Caboozled_Pie
            (owners, info.id)
        }
        Err(why) => panic!("Could not access application info: {:?}", why),
    };
    owners
}

fn create_framework(owners: HashSet<UserId>) -> StandardFramework {
    StandardFramework::new()
        .configure(|c| c.prefix(PREFIX).owners(owners))
        .before(before_command)
        .after(after_command)
        .help(&HELP)
        .group(&OWNER_GROUP)
        .group(&PLAYER_GROUP)
        .group(&STAFF_GROUP)
        .group(&TOURNAMENT_GROUP)
}

// make database available globally so we only maintain a single connection!
// the data is never actually mutated locally, so no read write lock is necessary
async fn setup_shared_data(database: LocalDatabase, client: &Client) {
    let mut data = client.data.write().await;
    data.insert::<LocalDatabase>(Arc::new(database));
    data.insert::<ShardManagerContainer>(client.shard_manager.clone());
}

pub async fn get_database(ctx: &Context) -> Arc<LocalDatabase> {
    let data_read = ctx.data.read().await;
    data_read
        .get::<LocalDatabase>()
        .expect("Expected database in TypeMap")
        .clone()
}

#[help]
#[lacking_ownership("hide")]
#[lacking_permissions("hide")]
#[lacking_role("hide")]
async fn help(
    context: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
    Ok(())
}

#[hook]
async fn before_command(_ctx: &Context, msg: &Message, command_name: &str) -> bool {
    info!(
        "Got command '{}' by user '{}'",
        command_name, msg.author.name
    );
    true // if `before` returns false, command processing doesn't happen.
}

#[hook]
async fn after_command(
    ctx: &Context,
    msg: &Message,
    command_name: &str,
    command_result: CommandResult,
) {
    match command_result {
        Ok(()) => {
            info!("Processed command '{}'", command_name);
        }
        Err(why) => {
            warn!("Command '{}' returned error {:?}", command_name, why);
            msg.react(&ctx.http, ReactionType::Unicode(ERROR_EMOJI.to_string()))
                .await
                .expect("Could not react?");
        }
    };
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
    }

    async fn resume(&self, _ctx: Context, _: ResumedEvent) {
        info!("Resumed");
    }
}

struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

pub mod util {
    use std::str::FromStr;

    use chrono::{TimeZone, Utc};
    use serenity::builder::CreateEmbed;
    use serenity::framework::standard::CommandResult;
    use serenity::model::prelude::*;
    use serenity::prelude::*;
    use tokio::time;

    use crate::database::players::PlayerEntry;
    use crate::discord::{CONFIRM_EMOJI, ERROR_EMOJI};

    pub fn player_data_to_embed(entry: &PlayerEntry) -> CreateEmbed {
        let mut e = CreateEmbed::default();

        if let Some(player) = &entry.tetrio_data {
            e.title(&player.username);
            e.url(format!("https://ch.tetr.io/u/{}", player._id));
            let league = &player.league;
            let rank = crate::tetrio::Rank::from_str(&league.rank).unwrap();
            e.color(u64::from_str_radix(rank.to_color(), 16).unwrap_or(0));
            e.thumbnail(rank.to_img_url());
            e.fields(vec![
                (
                    "Tetra Rating",
                    format!(
                        "{:.0} ± {:.1}",
                        &league.rating,
                        &league.rd.unwrap_or_default()
                    ),
                    false,
                ),
                (
                    "APM",
                    format!("{:.2}", &league.apm.unwrap_or_default()),
                    true,
                ),
                (
                    "PPS",
                    format!("{:.2}", &league.pps.unwrap_or_default()),
                    true,
                ),
                ("VS", format!("{:.2}", &league.vs.unwrap_or_default()), true),
            ]);
        }

        if let Some(cache_data) = &entry.cache_data {
            e.timestamp(Utc.timestamp(cache_data.cached_at / 1000, 0).to_rfc3339());
        }

        e
    }

    pub async fn react_confirm(ctx: &Context, msg: &Message) {
        msg.react(&ctx.http, ReactionType::Unicode(CONFIRM_EMOJI.to_string()))
            .await
            .expect("Could not react?");
    }

    pub async fn react_deny(ctx: &Context, msg: &Message) {
        msg.react(&ctx.http, ReactionType::Unicode(ERROR_EMOJI.to_string()))
            .await
            .expect("Could not react?");
    }

    pub async fn delay_delete(ctx: &Context, reply: Option<Message>) -> CommandResult {
        if let Some(reply) = reply {
            time::sleep(time::Duration::from_secs(10)).await;
            reply.delete(&ctx.http).await?;
        }
        Ok(())
    }
}
