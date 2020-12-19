use serenity::framework::standard::{macros::command, Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::tetrio::database::DatabaseError;
use crate::tetrio::database::*;

#[command]
#[only_in(guilds)]
#[description("Links your Discord User to a Tetrio User")]
#[num_args(1)]
async fn link(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let username = args.rest();
    let result = discord::link(msg.author.id.0, args.rest()).await;
    match result {
        Ok(user) => {
            msg.channel_id
                .say(
                    &ctx.http,
                    format!("Linked Discord user with Tetrio user {}", user.username),
                )
                .await?;

            change_nickname(ctx, msg, &user.username).await?;
        }
        Err(DatabaseError::NotFound) => {
            msg.channel_id
                .say(&ctx.http, format!("User {} not found", username))
                .await?;
        }
        Err(DatabaseError::DuplicateEntry) => {
            msg.channel_id
                .say(
                    &ctx.http,
                    "Another Discord user has already linked this Tetrio user",
                )
                .await?;
        }
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "Connection to database failed")
                .await?;
        }
    };
    Ok(())
}

#[command]
#[only_in(guilds)]
#[description("Unlinks your Discord User from a Tetrio User")]
async fn unlink(ctx: &Context, msg: &Message) -> CommandResult {
    match discord::unlink(msg.author.id.0).await {
        Ok(_) => {
            msg.channel_id
                .say(&ctx.http, "Unlinked Discord user from Tetrio user")
                .await?;
            change_nickname(ctx, msg, "").await?;
        }
        Err(DatabaseError::NotFound) => {
            msg.channel_id.say(&ctx.http, "User not found").await?;
        }
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "Connection to database failed")
                .await?;
        }
    };

    Ok(())
}

async fn change_nickname(ctx: &Context, msg: &Message, nickname: &str) -> CommandResult {
    // Check should make sure it's always in a guild
    let member = msg.member(&ctx.http).await.expect("Not in guild");
    if let Err(e) = member.edit(&ctx.http, |m| m.nickname(nickname)).await {
        msg.channel_id
            .say(
                &ctx.http,
                format!("Could not change nickname, reason: {}", e),
            )
            .await?;
    }
    Ok(())
}
