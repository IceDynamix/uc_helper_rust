use serenity::framework::standard::{macros::command, Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;

#[command]
async fn echo(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let reply = args.current().unwrap_or("Nothing");
    if let Err(e) = msg
        .channel_id
        .say(&ctx.http, format!("echo: {:?}", reply))
        .await
    {
        println!("Error sending message: {}", e);
    }

    Ok(())
}
