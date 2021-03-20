use std::time::Duration;

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

#[command]
#[owners_only]
async fn add_snapshot(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    match args.current() {
        None => {
            msg.channel_id
                .say(&ctx.http, "Missing argument (tournament)")
                .await?;
        }
        Some(arg) => {
            let db = crate::discord::get_database(&ctx).await;

            let tournament = match db.tournaments.get_tournament(arg) {
                Ok(tournament_option) => match tournament_option {
                    Some(tournament) => tournament,
                    None => {
                        react_deny(&ctx, &msg).await;
                        msg.channel_id
                            .say(&ctx.http, "Tournament not found")
                            .await?;
                        return Ok(());
                    }
                },
                Err(err) => {
                    react_deny(&ctx, &msg).await;
                    msg.channel_id.say(&ctx.http, err).await?;
                    return Ok(());
                }
            };

            let mut replies = Vec::new();

            replies.push(
                msg.channel_id
                    .say(
                        &ctx.http,
                        "Updating all player stats, could take a few minutes...",
                    )
                    .await?,
            );

            let typing = msg.channel_id.start_typing(&ctx.http)?;
            let update_result = db.players.update_from_leaderboard();
            typing.stop();

            if let Err(err) = update_result {
                react_deny(&ctx, &msg).await;
                msg.channel_id.say(&ctx.http, err).await?;
                return Ok(());
            }

            replies.push(
                msg.channel_id
                    .say(&ctx.http, "Finished updating all players")
                    .await?,
            );

            replies.push(
                msg.channel_id
                    .say(&ctx.http, "Creating snapshot...")
                    .await?,
            );

            match db.tournaments.add_snapshot(&tournament.shorthand) {
                Ok(_) => {
                    react_confirm(&ctx, &msg).await;
                    tokio::time::sleep(Duration::from_secs(10)).await;
                    for reply in replies {
                        reply.delete(&ctx.http).await?;
                    }
                }
                Err(err) => {
                    react_deny(&ctx, &msg).await;
                    msg.channel_id.say(&ctx.http, err).await?;
                }
            }
        }
    };

    Ok(())
}

#[command]
#[owners_only]
async fn create_check_in(ctx: &Context, msg: &Message) -> CommandResult {
    let db = crate::discord::get_database(&ctx).await;
    const CHECK_IN_CHANNEL_NAME: &str = "check-in";

    match db.tournaments.get_active() {
        Ok(tournament) => match tournament {
            Some(tournament) => {
                let guild = msg.guild(&ctx.cache).await.unwrap();
                match guild
                    .channel_id_from_name(&ctx.cache, CHECK_IN_CHANNEL_NAME)
                    .await
                {
                    Some(channel) => {
                        let check_in_msg = channel
                            .send_message(&ctx.http, |m| {
                                m.embed(|e| {
                                    e.title(format!("{}: Check-in", tournament.shorthand));
                                    e.description(format!(
                                        "React to this message with {} in order to check-in!",
                                        crate::discord::CONFIRM_EMOJI
                                    ));
                                    e
                                });
                                m
                            })
                            .await?;

                        react_confirm(&ctx, &check_in_msg).await;

                        if let Err(err) = db
                            .tournaments
                            .set_check_in_msg(&tournament.shorthand, check_in_msg.id.0)
                        {
                            react_deny(&ctx, &msg).await;
                            msg.channel_id
                                .say(
                                    &ctx.http,
                                    format!(
                                        "Could not set check-in message in tournament db ({})",
                                        err
                                    ),
                                )
                                .await?;
                        } else {
                            react_confirm(&ctx, &msg).await;
                        }
                    }
                    None => {
                        react_deny(&ctx, &msg).await;
                        msg.channel_id
                            .say(
                                &ctx.http,
                                format!("No #{} channel found", CHECK_IN_CHANNEL_NAME),
                            )
                            .await?;
                        return Ok(());
                    }
                };
            }
            None => {
                react_deny(&ctx, &msg).await;
                msg.channel_id
                    .say(&ctx.http, "No active tournament")
                    .await?;
            }
        },
        Err(err) => {
            react_deny(&ctx, &msg).await;
            msg.channel_id.say(&ctx.http, err).await?;
        }
    }

    Ok(())
}
