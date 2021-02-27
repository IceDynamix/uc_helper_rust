use serenity::framework::standard::{macros::command, Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::discord::util::reply;

#[command]
async fn echo(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    reply(ctx, msg, args.current().unwrap_or("Nothing")).await;
    Ok(())
}
