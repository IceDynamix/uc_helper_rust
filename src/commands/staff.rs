use serenity::framework::standard::{macros::command, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::discord::util::*;

#[command]
async fn staff_ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, "Pong!").await?;
    Ok(())
}

#[command]
async fn update_all(ctx: &Context, msg: &Message) -> CommandResult {
    let typing = msg.channel_id.start_typing(&ctx.http)?;

    let db = crate::discord::get_database(&ctx).await;
    match db.players.update_from_leaderboard() {
        Ok(_) => {
            react_confirm(&ctx, &msg).await;
        }
        Err(err) => {
            tracing::warn!("{}", err);
            msg.channel_id.say(&ctx.http, err).await?;
        }
    }

    typing.stop();
    Ok(())
}
