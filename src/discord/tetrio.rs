use serenity::framework::standard::{macros::command, Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::database::players::PlayerEntry;
use crate::database::DatabaseError;
use crate::database::*;
use crate::tetrio;

#[command]
#[only_in(guilds)]
#[description("Links your Discord User to a Tetrio User")]
#[num_args(1)]
pub async fn link(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let (response, _) = link_action(ctx, msg, &args).await?;
    msg.channel_id
        .say(&ctx.http, format!("<@{}> {}", msg.author.id.0, response))
        .await?;
    Ok(())
}

// returning a tuple is not good but im under time pressure
// TODO find something better
pub async fn link_action(
    ctx: &Context,
    msg: &Message,
    args: &Args,
) -> CommandResult<(String, Option<tetrio::User>)> {
    let username = args.rest();
    let result = discord::link(msg.author.id.0, args.rest()).await;
    let response = match result {
        Ok(user) => {
            change_nickname(ctx, msg, args.rest()).await?;
            (
                format!("Linked Discord user with Tetrio user {}", user.username),
                Some(user),
            )
        }
        Err(DatabaseError::NotFound) => (format!("User {} not found", username), None),
        Err(DatabaseError::DuplicateEntry) => (
            "Another Discord user has already linked this Tetrio user".to_string(),
            None,
        ),
        Err(_) => ("Connection to database failed".to_string(), None),
    };

    Ok(response)
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
            .say(&ctx.http, format!("Could not change nickname ({})", e))
            .await?;
    }
    Ok(())
}

#[command]
#[only_in(guilds)]
#[description("Get stats of a particular Tetrio or Discord user")]
#[max_args(1)]
async fn stats(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let data = if args.is_empty() {
        let id = msg.author.id.0;
        let author_name = msg
            .member
            .clone()
            .expect("not in guild")
            .nick
            .unwrap_or_else(|| msg.author.name.clone());

        let lookup_value = match discord::get_from_discord_id(id).await {
            Ok(entry) => entry.tetrio_id,
            Err(_) => author_name,
        };

        lookup(ctx, msg, &lookup_value).await
    } else if let Some(mentioned_id) = serenity::utils::parse_mention(args.rest()) {
        let result = discord::get_from_discord_id(mentioned_id).await;
        match result {
            Ok(entry) => lookup(ctx, msg, &entry.tetrio_id).await,
            Err(DatabaseError::NotFound) => {
                msg.channel_id
                    .say(
                        &ctx.http,
                        "Discord user isn't linked to a Tetrio user, use `.link <username>`",
                    )
                    .await?;
                None
            }
            Err(_) => {
                msg.channel_id
                    .say(&ctx.http, "Connection to database failed")
                    .await?;
                None
            }
        }
    } else {
        lookup(ctx, msg, args.rest()).await
    };

    if let Some(entry) = data {
        msg.channel_id
            .send_message(&ctx.http, |m| m.set_embed(entry.generate_embed()))
            .await?;
    }

    Ok(())
}

async fn lookup(ctx: &Context, msg: &Message, username: &str) -> Option<PlayerEntry> {
    match players::get(username).await {
        Ok(user) => Some(user),
        Err(DatabaseError::NotFound) => {
            msg.channel_id
                .say(&ctx.http, format!("User {} not found", username))
                .await
                .ok()?;
            None
        }
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "Connection to database failed")
                .await
                .ok()?;
            None
        }
    }
}
