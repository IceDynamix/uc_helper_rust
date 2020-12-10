use serenity::{
    async_trait,
    client::{bridge::gateway::ShardManager, Context, EventHandler},
    framework::{
        standard::{
            macros::{command, group},
            CommandResult,
        },
        StandardFramework,
    },
    http::Http,
    model::{channel::Message, event::ResumedEvent, id::GuildId, prelude::*},
    prelude::*,
    Client,
};
use std::{collections::HashSet, env, error::Error, sync::Arc};

use connections::sheet::Sheet;
use settings::Settings;

use tracing::{error, info};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

mod connections;
mod settings;

#[group]
#[commands(ping)]
struct General;

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, "Pong!").await?;
    Ok(())
}

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

pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // let settings = Settings::from_profile("debug").unwrap();
    // let sheet = Sheet::new(settings.staff_sheet_id).await?;
    // let data = tetrio::User::request(vec!["icedynamix".to_string(), "electroyan".to_string()]).await?;

    // Downloads a few gigabytes of data so use with care
    // let rank_histories = tenchi::PlayerHistory::refresh().await?;
    // let player_history = tenchi::PlayerHistory::from_cache().await?;
    // let username = "icedynamix";
    // println!("all: {:?}", player_history.get_ranks(username).await);
    // println!("highest: {:?}",player_history.get_ranks(username).await.unwrap().iter().max());

    dotenv::dotenv().expect("Failed to load .env file");

    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
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

    // Create the framework
    let framework = StandardFramework::new()
        .configure(|c| c.owners(owners).prefix("!"))
        .group(&GENERAL_GROUP);

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
