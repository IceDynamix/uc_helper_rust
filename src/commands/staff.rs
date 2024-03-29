use serenity::framework::standard::{macros::command, Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::database::tournaments::RegistrationError;
use crate::database::DatabaseError;
use crate::discord::util::*;

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

#[command]
async fn update_registered(ctx: &Context, msg: &Message) -> CommandResult {
    let typing = msg.channel_id.start_typing(&ctx.http)?;
    let db = crate::discord::get_database(&ctx).await;
    let tour = db.tournaments.get_active().unwrap().unwrap();

    match db.players.update_registered(tour) {
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

#[command]
async fn set_active(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let db = crate::discord::get_database(&ctx).await;
    match db.tournaments.set_active(args.current()) {
        Ok(entry) => {
            react_confirm(&ctx, &msg).await;
            if entry.is_none() {
                msg.channel_id
                    .say(&ctx.http, "Set all tournaments to inactive")
                    .await?;
            }
        }
        Err(err) => {
            tracing::warn!("{}", err);
            msg.channel_id.say(&ctx.http, err).await?;
        }
    }

    Ok(())
}

#[command]
async fn staff_register(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let discord_account_to_link = match args.current() {
        Some(arg) => {
            let discord_id = serenity::utils::parse_mention(arg);
            match discord_id {
                Some(discord_id) => {
                    if msg
                        .guild_id
                        .unwrap()
                        .member(&ctx.http, discord_id)
                        .await
                        .is_err()
                    {
                        msg.channel_id
                            .say(&ctx.http, "Mentioned user is not in the server!")
                            .await?;
                        return Ok(());
                    }

                    discord_id
                }
                None => {
                    msg.channel_id
                        .say(
                            &ctx.http,
                            "Discord user provided was not valid (use a mention/ping)",
                        )
                        .await?;
                    return Ok(());
                }
            }
        }
        None => {
            msg.channel_id
                .say(&ctx.http, "No Discord user provided (use a mention/ping)")
                .await?;
            return Ok(());
        }
    };

    args.advance();

    let db = crate::discord::get_database(&ctx).await;
    let reply = match db.tournaments.register_to_active(
        &db.players,
        args.current(),
        discord_account_to_link,
        true,
    ) {
        Ok(entry) => {
            react_confirm(&ctx, &msg).await;
            Some(
                msg.channel_id
                    .send_message(&ctx.http, |m| m.set_embed(player_data_to_embed(&entry)))
                    .await?,
            )
        }
        Err(err) => {
            react_deny(&ctx, &msg).await;
            let reply = match err {
                RegistrationError::AlreadyRegistered => {
                    "The player is already registered!".to_string()
                }
                RegistrationError::DatabaseError(err) => match err {
                    DatabaseError::DuplicateDiscordEntry => {
                        "The user is already linked!".to_string()
                    }
                    DatabaseError::DuplicateTetrioEntry => {
                        "Someone else has already linked this user!".to_string()
                    }
                    DatabaseError::NotFound => "Could not find specified user!".to_string(),
                    _ => format!("{:?}", err),
                },
                _ => format!("{:?}", err),
            };

            Some(
                msg.channel_id
                    .say(&ctx.http, format!("<@{}> {}", msg.author.id, reply))
                    .await?,
            )
        }
    };

    delay_delete(&ctx, reply).await?;

    Ok(())
}

#[command]
async fn staff_unregister(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let db = crate::discord::get_database(&ctx).await;
    let username = match args.current() {
        Some(username) => username,
        None => {
            msg.channel_id
                .say(&ctx.http, "No username provided")
                .await?;
            return Ok(());
        }
    };

    match db.tournaments.unregister_by_tetrio(&db.players, username) {
        Ok(_) => {
            react_confirm(&ctx, &msg).await;
        }
        Err(err) => {
            react_deny(&ctx, &msg).await;
            msg.channel_id.say(&ctx.http, err).await?;
        }
    };

    Ok(())
}

#[command]
async fn staff_link(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let db = crate::discord::get_database(&ctx).await;

    let discord_id = match args.current() {
        Some(arg) => match serenity::utils::parse_mention(arg) {
            Some(discord_id) => discord_id,
            None => {
                react_deny(&ctx, &msg).await;
                msg.channel_id
                    .say(
                        &ctx.http,
                        "First argument was not a mention (`.staff_link <mention> <username>`)",
                    )
                    .await?;
                return Ok(());
            }
        },
        None => {
            react_deny(&ctx, &msg).await;
            msg.channel_id
                .say(
                    &ctx.http,
                    "Discord mention/ping missing (`.staff_link <mention> <username>`)",
                )
                .await?;
            return Ok(());
        }
    };

    args.advance();

    let username = match args.current() {
        Some(username) => username,
        None => {
            react_deny(&ctx, &msg).await;
            msg.channel_id
                .say(
                    &ctx.http,
                    "Username missing (`.staff_link <mention> <username>`)",
                )
                .await?;
            return Ok(());
        }
    };

    match db.players.link(discord_id, username) {
        Ok(_) => {
            react_confirm(&ctx, &msg).await;
        }
        Err(err) => {
            react_deny(&ctx, &msg).await;
            msg.channel_id.say(&ctx.http, err).await?;
        }
    }

    Ok(())
}

#[command]
async fn staff_unlink(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let db = crate::discord::get_database(&ctx).await;

    match args.current() {
        None => {
            msg.channel_id
                .say(&ctx.http, "No username or mention provided")
                .await?;
        }
        Some(arg) => match serenity::utils::parse_mention(arg) {
            Some(discord_id) => match db.players.unlink_by_discord(discord_id) {
                Ok(_) => {
                    react_confirm(&ctx, &msg).await;
                }
                Err(err) => {
                    react_deny(&ctx, &msg).await;
                    msg.channel_id.say(&ctx.http, err).await?;
                }
            },
            None => match db.players.unlink_by_tetrio(arg) {
                Ok(_) => {
                    react_confirm(&ctx, &msg).await;
                }
                Err(err) => {
                    react_deny(&ctx, &msg).await;
                    msg.channel_id.say(&ctx.http, err).await?;
                }
            },
        },
    }

    Ok(())
}
