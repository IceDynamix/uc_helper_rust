use std::sync::Arc;
use std::time::Duration;

use serenity::collector::ReactionAction;
use serenity::framework::standard::{macros::command, Args, CommandResult};
use serenity::futures::StreamExt;
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::database::tournaments::{RegistrationError, TournamentEntry};
use crate::database::{DatabaseError, LocalDatabase};
use crate::discord::util::*;
use crate::discord::CONFIRM_EMOJI;

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

    let tournament = match db.tournaments.get_active() {
        Ok(tournament) => match tournament {
            Some(tournament) => tournament,
            None => {
                react_deny(&ctx, &msg).await;
                msg.channel_id
                    .say(&ctx.http, "No active tournament")
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

    let check_in_msg = msg
        .channel_id
        .send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.title(format!("{}: Check-in", tournament.shorthand))
                    .description(format!(
                        "React to this message with {} in order to check-in!",
                        crate::discord::CONFIRM_EMOJI
                    ))
            })
        })
        .await?;

    if let Err(err) = db
        .tournaments
        .set_check_in_msg(&tournament.shorthand, check_in_msg.id.0)
    {
        react_deny(&ctx, &msg).await;
        msg.channel_id
            .say(
                &ctx.http,
                format!(
                    "Could not set check-in message in tournament db ({:?})",
                    err
                ),
            )
            .await?;
    } else {
        msg.delete(&ctx.http).await?;
        init_checkin_reaction_handling(&ctx, db, tournament, &check_in_msg).await?;
    }

    Ok(())
}

async fn init_checkin_reaction_handling(
    ctx: &Context,
    db: Arc<LocalDatabase>,
    tournament: TournamentEntry,
    check_in_msg: &Message,
) -> CommandResult {
    react_confirm(&ctx, &check_in_msg).await;
    let mut reaction_collector = check_in_msg
        .await_reactions(&ctx)
        .added(true)
        .removed(true)
        .await;

    while let Some(action) = reaction_collector.next().await {
        if let Err(e) = handle_checkin_reaction(&ctx, &db, &tournament, action).await {
            tracing::error!("Error during check-in handling: {}", e);
        }
    }

    Ok(())
}

async fn handle_checkin_reaction(
    ctx: &Context,
    db: &Arc<LocalDatabase>,
    tournament: &TournamentEntry,
    action: Arc<ReactionAction>,
) -> CommandResult {
    let confirm_emoji = ReactionType::Unicode(CONFIRM_EMOJI.to_string());
    match action.as_ref() {
        ReactionAction::Added(reaction) | ReactionAction::Removed(reaction)
            if reaction.emoji == confirm_emoji =>
        {
            let dms = reaction
                .user_id
                .unwrap()
                .create_dm_channel(&ctx.http)
                .await
                .unwrap();

            let player = match db
                .players
                .get_player_by_discord(reaction.user_id.unwrap().0)
            {
                Ok(player) => match player {
                    Some(player) => player,
                    None => {
                        dms.say(&ctx.http, "Your discord user is not linked to a Tetrio account! You most likely haven't registered at all.").await?;
                        return Ok(());
                    }
                },
                Err(err) => {
                    dms.say(&ctx.http, err).await?;
                    return Ok(());
                }
            };

            let reply = if tournament.player_is_registered(&player) {
                match action.as_ref() {
                        ReactionAction::Added(_) => "You have checked-in successfully. Please stand by until the tournament begins. Instructions on how to play in the tournament will be posted once the bracket is finalized.",
                        ReactionAction::Removed(_) => "You have checked-out successfully."
                    }
            } else {
                "You weren't registered! Please do keep in mind that registering *(which happens in the week before the tournament)* and checking in *(which happens just before the tournament)* are two different processes."
            };

            dms.say(&ctx.http, reply).await?;
        }
        _ => {}
    }

    Ok(())
}
