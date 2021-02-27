use std::sync::Arc;

use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::{async_trait, framework::StandardFramework, prelude::*, Client};

use crate::database::LocalDatabase;

pub struct Bot(pub Client);

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content.starts_with("!player ") {
            let arg = msg.content.replace("!player ", "");

            let database = {
                let data_read = ctx.data.read().await;
                data_read
                    .get::<LocalDatabase>()
                    .expect("Expected database in Typemap")
                    .clone()
            };

            let player = database.players.get_player_by_tetrio(&arg).unwrap();

            let reply = match player {
                Some(p) => format!("{:?}", p),
                None => "Not found".to_string(),
            };

            if let Err(e) = msg.channel_id.say(&ctx.http, reply).await {
                println!("{}", e);
            }
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

impl Bot {
    pub async fn new(database: LocalDatabase) -> Bot {
        let token = std::env::var("DISCORD_TOKEN").expect("No Discord token");
        let framework = StandardFramework::new().configure(|c| c.prefix("."));
        let client = Client::builder(&token)
            .event_handler(Handler)
            .framework(framework)
            .await
            .expect("Couldn't create client");

        // make database available globally so we only maintain a single connection!
        // the local database is never actually mutated, so no read write lock is necessary
        {
            let mut data = client.data.write().await;
            data.insert::<LocalDatabase>(Arc::new(database));
        }

        Bot(client)
    }
}
