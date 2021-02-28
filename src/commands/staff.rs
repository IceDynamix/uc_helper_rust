use serenity::framework::standard::{macros::command, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::discord::util::reply;

#[command]
async fn staff_ping(ctx: &Context, msg: &Message) -> CommandResult {
    reply(ctx, msg, "Pong!").await;
    Ok(())
}
