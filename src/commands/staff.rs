use serenity::framework::standard::{macros::command, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::discord::util::reply;

#[command]
async fn staff_ping(ctx: &Context, msg: &Message) -> CommandResult {
    reply(ctx, msg, "Pong!").await;
    Ok(())
}

#[command]
async fn update_all(ctx: &Context, msg: &Message) -> CommandResult {
    let typing = match msg.channel_id.start_typing(&ctx.http) {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!("Error while starting to type: {}", e);
            reply(
                ctx,
                msg,
                "Something went wrong, please contact the bot owner",
            )
            .await;
            return Ok(());
        }
    };

    let db = crate::discord::get_database(&ctx).await;
    match db.players.update_from_leaderboard() {
        Ok(_) => {
            reply(ctx, msg, "Updated all players successfully").await;
            typing.stop();
            Ok(())
        }
        Err(e) => {
            tracing::warn!("Error while updating all players: {}", e);
            Ok(())
        }
    }
}
