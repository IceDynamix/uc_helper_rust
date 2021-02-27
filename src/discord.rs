use std::collections::HashSet;
use std::sync::Arc;

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

use crate::commands::owner::*;
use crate::database::LocalDatabase;

const PREFIX: &str = ".";

#[group]
#[commands(echo)]
#[owners_only]
struct Owner;

pub async fn new_client(database: LocalDatabase) -> Client {
    let token = std::env::var("DISCORD_TOKEN").expect("No Discord token");
    let owners = get_bot_owners(&token).await;
    let framework = create_framework(owners);

    let client = Client::builder(&token)
        .event_handler(Handler)
        .framework(framework)
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
}

// make database available globally so we only maintain a single connection!
// the data is never actually mutated locally, so no read write lock is necessary
async fn setup_shared_data(database: LocalDatabase, client: &Client) {
    let mut data = client.data.write().await;
    data.insert::<LocalDatabase>(Arc::new(database));
    data.insert::<ShardManagerContainer>(client.shard_manager.clone());
}

async fn _get_database(ctx: &Context) -> Arc<LocalDatabase> {
    let data_read = ctx.data.read().await;
    data_read
        .get::<LocalDatabase>()
        .expect("Expected database in Typemap")
        .clone()
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
    _ctx: &Context,
    _msg: &Message,
    command_name: &str,
    command_result: CommandResult,
) {
    match command_result {
        Ok(()) => info!("Processed command '{}'", command_name),
        Err(why) => warn!("Command '{}' returned error {:?}", command_name, why),
    }
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
