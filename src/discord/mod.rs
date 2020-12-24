use std::{collections::HashSet, env, sync::Arc};

use serenity::framework::standard::{
    help_commands, Args, CommandGroup, CommandResult, HelpOptions,
};
use serenity::{
    async_trait,
    client::{bridge::gateway::ShardManager, Context, EventHandler},
    framework::{
        standard::macros::{group, help, hook},
        StandardFramework,
    },
    http::Http,
    model::{event::ResumedEvent, id::GuildId, prelude::*},
    prelude::*,
    Client,
};
use tracing::{error, info, warn};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

use general::*;
use tetrio::*;
use tournament::*;

mod general;
mod tetrio;
mod tournament;

const BOT_ID: u64 = 776455810683371580;

#[group]
#[commands(ping, echo)]
struct General;

#[group]
#[commands(link, unlink, stats, who_is)]
struct Tetrio;

#[group]
#[commands(register, unregister, can_participate, staff_unregister, player_list)]
struct Tournament;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn cache_ready(&self, _ctx: Context, _guilds: Vec<GuildId>) {}

    async fn ready(&self, _ctx: Context, ready: Ready) {
        info!("Connected as {}", ready.user.name);
    }

    async fn resume(&self, _ctx: Context, _: ResumedEvent) {
        info!("Resumed");
    }
}

#[hook]
async fn before(_ctx: &Context, msg: &Message, command_name: &str) -> bool {
    info!(
        "Got command '{}' by user '{}'",
        command_name, msg.author.name
    );
    true // if `before` returns false, command processing doesn't happen.
}

#[hook]
async fn after(_ctx: &Context, _msg: &Message, command_name: &str, command_result: CommandResult) {
    match command_result {
        Ok(()) => info!("Processed command '{}'", command_name),
        Err(why) => warn!("Command '{}' returned error {:?}", command_name, why),
    }
}

#[help]
#[lacking_ownership(hide)]
#[lacking_permissions(hide)]
#[lacking_role(hide)]
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

pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

pub async fn start() -> serenity::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .compact()
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to start the logger");

    let token = env::var("DISCORD_TOKEN").expect("Missing DISCORD_TOKEN");

    let http = Http::new_with_token(&token);
    let (owners, _bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            owners.insert(info.owner.id);

            (owners, info.id)
        }
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    // TODO: Extract into settings
    let allowed_channels = vec![
        ChannelId(752703502173863966),
        ChannelId(776806403884056616),
        ChannelId(790024599353819187),
    ]
    .into_iter()
    .collect();

    // Create the framework
    let framework = StandardFramework::new()
        .configure(|c| {
            c.prefix(".")
                .owners(owners)
                .allow_dm(false)
                .ignore_bots(true)
                .allowed_channels(allowed_channels)
                .on_mention(Some(UserId(BOT_ID)))
        })
        .before(before)
        .after(after)
        .help(&HELP)
        .group(&GENERAL_GROUP)
        .group(&TETRIO_GROUP)
        .group(&TOURNAMENT_GROUP);

    let mut client = Client::builder(&token)
        .framework(framework)
        .event_handler(Handler)
        .await
        .expect("Failed to initialize client");

    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(client.shard_manager.clone());
    }

    let shard_manager = client.shard_manager.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        shard_manager.lock().await.shutdown_all().await;
    });

    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }

    Ok(())
}
