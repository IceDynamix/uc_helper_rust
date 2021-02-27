use serenity::framework::standard::{macros::command, Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::utils;

use crate::discord;
use crate::discord::util;

#[command]
#[usage("[username/discord ping]")]
#[example("caboozled_pie")]
#[example("@IceDynamix")]
/// Retrieve a players stats by username, Tetrio ID or Discord user ping.
///
/// If neither is passed then it will use the Tetr.io account linked with the current Discord user.
async fn stats(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let database = discord::get_database(&ctx).await;

    let lookup = if let Some(content) = args.current() {
        if let Some(id) = utils::parse_mention(content) {
            (
                database.players.get_player_by_discord(id),
                "Mentioned user is not linked to a Tetr.io user",
            )
        } else {
            (
                database
                    .players
                    .get_player_by_tetrio(&content.to_lowercase()),
                "Player does not exist",
            )
        }
    } else {
        (
            database.players.get_player_by_discord(msg.author.id.0),
            "Your account is not linked to a Tetr.io user",
        )
    };

    match lookup.0.unwrap() {
        None => {
            util::reply(ctx, msg, lookup.1).await;
        }
        Some(entry) => {
            let updated_entry = database.players.update_player(&entry.tetrio_id).unwrap();
            msg.channel_id
                .send_message(&ctx.http, |m| {
                    m.set_embed(util::player_data_to_embed(&updated_entry))
                })
                .await
                .expect("Could not send message");
        }
    }

    Ok(())
}
