use serenity::framework::standard::{macros::command, Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::utils;

use crate::database::DatabaseError;
use crate::discord;
use crate::discord::util::*;

#[command]
#[usage("[tetrio username / tetrio id / discord mention]")]
#[example("caboozled_pie")]
#[example("5e47696db7c60f23a497ee6c")]
#[example("@IceDynamix")]
/// Retrieve a players stats by username, Tetrio ID or Discord user ping.
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
            msg.channel_id.say(&ctx.http, lookup.1).await?;
        }
        Some(entry) => {
            let updated_entry = database.players.update_player(&entry.tetrio_id).unwrap();
            msg.channel_id
                .send_message(&ctx.http, |m| {
                    m.set_embed(player_data_to_embed(&updated_entry))
                })
                .await?;
        }
    }

    Ok(())
}

#[command]
#[usage("<tetr.io username or id>")]
#[example("caboozled_pie")]
#[example("5e47696db7c60f23a497ee6c")]
/// Will make the bot "remember" that you are a specified Tetr.io user.
/// Useful for registration or for easy stat/player lookup
/// It will retain the link, even if you change your username
async fn link(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let reply = match args.current() {
        None => {
            react_deny(&ctx, &msg).await;
            Some(
                msg.channel_id
                    .say(
                        &ctx.http,
                        "No tetr.io user was specified, run `help link` for more information",
                    )
                    .await?,
            )
        }
        Some(args) => {
            let db = crate::discord::get_database(ctx).await;
            match db.players.link(msg.author.id.0, args) {
                Ok(entry) => {
                    react_confirm(&ctx, &msg).await;
                    Some(msg.channel_id
                        .send_message(&ctx.http, |m| m.set_embed(player_data_to_embed(&entry)))
                        .await?)
                }
                Err(err) => match err {
                    DatabaseError::DuplicateDiscordEntry => {
                        Some(msg.channel_id
                            .say(&ctx.http, "You're already linked to a Tetr.io user! Use the `unlink` command before linking to another Tetr.io user")
                            .await?)
                    }
                    DatabaseError::DuplicateTetrioEntry => {
                        Some(msg.channel_id
                            .say(&ctx.http, "You're trying to link a user who is already linked to someone else!")
                            .await?)
                    }
                    _ => {
                        tracing::warn!("{}", err);
                        Some(msg.channel_id.say(&ctx.http, err).await?)
                    }
                },
            }
        }
    };

    delay_delete(&ctx, reply).await?;

    Ok(())
}

#[command]
/// Removes the link between you and your linked Tetr.io user
async fn unlink(ctx: &Context, msg: &Message) -> CommandResult {
    let db = crate::discord::get_database(ctx).await;
    let reply = match db.players.unlink_by_discord(msg.author.id.0) {
        Ok(_) => {
            react_confirm(&ctx, &msg).await;
            None
        }
        Err(err) => match err {
            DatabaseError::NotFound => {
                Some(msg.channel_id.say(&ctx.http, "There is no Tetr.io user linked to you right now, use the `link` command to link one").await?)
            }
            _ => {
                tracing::warn!("{}", err);
                Some(msg.channel_id.say(&ctx.http, err).await?)
            }
        },
    };

    // TODO: unregister if registered

    delay_delete(&ctx, reply).await?;

    Ok(())
}
