use serenity::framework::standard::{macros::command, Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::database::tournaments::RegistrationError;
use crate::database::DatabaseError;
use crate::discord::util::*;

#[command]
#[usage("[Tetr.io username or ID]")]
#[example("caboozled_pie")]
#[example("5e47696db7c60f23a497ee6c")]
/// Will register you to the ongoing tournament.
/// If no account is linked, then it will link you with the provided username.
async fn register(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let db = crate::discord::get_database(&ctx).await;
    let reply = match db.tournaments.register_to_active(
        &db.players,
        args.current(),
        msg.author.id.0,
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
            match err {
                RegistrationError::MissingArgument(_) => {
                    Some(msg.channel_id.say(
                        &ctx.http,
                        "There is no Tetr.io account linked to you right now, please provide a username. Run `help tournament register` for more information.",
                    ).await?)
                }
                RegistrationError::AlreadyRegistered => {
                    Some(msg.channel_id
                        .say(&ctx.http, "You're already registered!")
                        .await?)
                }
                RegistrationError::RdTooHigh { .. } // TODO: refer to a faq command for rd
                | RegistrationError::NoTournamentActive
                | RegistrationError::CurrentRankTooHigh { .. }
                | RegistrationError::AnnouncementRankTooHigh { .. }
                | RegistrationError::NotEnoughGames { .. }
                | RegistrationError::UnrankedOnAnnouncementDay(_) => {
                    Some(msg.channel_id.say(&ctx.http, err).await?)
                }
                RegistrationError::DatabaseError(err) => match err {
                    DatabaseError::DuplicateDiscordEntry => {
                        Some(msg.channel_id
                            .say(&ctx.http, "You're already linked to someone else! Use the `unlink` command if you'd like to link to someone else.")
                            .await?)
                    }
                    DatabaseError::DuplicateTetrioEntry => {
                        Some(msg.channel_id
                            .say(&ctx.http, "Someone else has already linked this user!")
                            .await?)
                    }
                    DatabaseError::NotFound => {
                        Some(msg.channel_id
                            .say(&ctx.http, "Could not find specified user!")
                            .await?)
                    }
                    _ => {
                        tracing::warn!("{}", err);
                        Some(msg.channel_id.say(&ctx.http, err).await?)
                    }
                },
                _ => { None }
            }
        }
    };

    delay_delete(&ctx, reply).await?;

    Ok(())
}

#[command]
/// Unregisters you from the ongoing tournament.
async fn unregister(ctx: &Context, msg: &Message) -> CommandResult {
    let db = crate::discord::get_database(&ctx).await;
    let reply = match db
        .tournaments
        .unregister_by_discord(&db.players, msg.author.id.0)
    {
        Ok(_) => {
            react_confirm(&ctx, &msg).await;
            None
        }
        Err(err) => {
            react_deny(&ctx, &msg).await;
            match err {
                RegistrationError::DatabaseError(err) => match err {
                    DatabaseError::NotFound => Some(msg.channel_id.say(&ctx.http, err).await?),
                    _ => {
                        tracing::warn!("{}", err);
                        Some(msg.channel_id.say(&ctx.http, err).await?)
                    }
                },
                _ => {
                    tracing::warn!("{}", err);
                    Some(msg.channel_id.say(&ctx.http, err).await?)
                }
            }
        }
    };

    delay_delete(&ctx, reply).await?;
    Ok(())
}
