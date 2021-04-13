use std::sync::Arc;
use std::time::Duration;

use serenity::framework::standard::{macros::command, Args, CommandResult};
use serenity::futures::StreamExt;
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::{collector::ReactionAction, http::AttachmentType};

use crate::database::tournaments::{RegistrationError, TournamentEntry};
use crate::database::{DatabaseError, LocalDatabase};
use crate::discord::util::*;
use crate::discord::IdCollection;
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
        false,
    ) {
        Ok(entry) => {
            react_confirm(&ctx, &msg).await;
            super::player::rename_user_to_tetrio(&ctx, msg, &entry).await?;
            Some(
                msg.channel_id
                    .send_message(&ctx.http, |m| m.set_embed(player_data_to_embed(&entry)))
                    .await?,
            )
        }
        Err(err) => {
            react_deny(&ctx, &msg).await;
            let reply = match err {
                RegistrationError::MissingArgument(_) =>
                    "There is no Tetr.io account linked to you right now, please provide a username. `.register [username]`".to_string(),
                RegistrationError::AlreadyRegistered => "You're already registered!".to_string(),
                RegistrationError::RdTooHigh { .. } // TODO: refer to a faq command for rd
                | RegistrationError::NoTournamentActive
                | RegistrationError::CurrentRankTooHigh { .. }
                | RegistrationError::AnnouncementRankTooHigh { .. }
                | RegistrationError::NotEnoughGames { .. }
                | RegistrationError::UnrankedOnAnnouncementDay(_) => format!("{}", err),
                RegistrationError::DatabaseError(err) => match err {
                    DatabaseError::DuplicateDiscordEntry => "You're already linked to someone else! Use the `unlink` command if you'd like to link to someone else.".to_string(),
                    DatabaseError::DuplicateTetrioEntry => "Someone else has already linked this user!".to_string(),
                    DatabaseError::NotFound => "Could not find specified user!".to_string(),
                    _ => format!("{:?}", err)
                },
                _ => format!("{:?}", err)
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

            let mut replies = vec![
                msg.channel_id
                    .say(
                        &ctx.http,
                        "Updating all player stats, could take a few minutes...",
                    )
                    .await?,
            ];

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
                        "React to this message with {} in order to check-in! Unreact to check-out.",
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
        init_checkin_reaction_handling(&ctx, db, tournament, &msg, &check_in_msg).await?;
    }

    Ok(())
}

#[command]
#[owners_only]
async fn resume_check_in(ctx: &Context, msg: &Message) -> CommandResult {
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

    let check_in_msg = match tournament.check_in_msg {
        Some(msg_id) => msg_id,
        None => {
            react_deny(&ctx, &msg).await;
            msg.channel_id
                .say(&ctx.http, "No check-in message found")
                .await?;
            return Ok(());
        }
    };

    // TODO: hardcoded IDs
    let check_in_msg = ctx
        .http
        .get_message(822933717453504562, check_in_msg)
        .await?;

    init_checkin_reaction_handling(&ctx, db, tournament, &msg, &check_in_msg).await
}

async fn init_checkin_reaction_handling(
    ctx: &Context,
    db: Arc<LocalDatabase>,
    tournament: TournamentEntry,
    msg: &Message,
    check_in_msg: &Message,
) -> CommandResult {
    react_confirm(&ctx, &check_in_msg).await;
    let mut reaction_collector = check_in_msg
        .await_reactions(&ctx)
        .added(true)
        .removed(true)
        .await;

    let channels = msg
        .guild_id
        .expect("Guild not cached")
        .channels(&ctx.http)
        .await
        .expect("Could not get channels");

    let mut log_channel = None;

    for (_, channel) in channels.iter() {
        if channel.name == "check-in-log" {
            log_channel = Some(channel.clone());
        }
    }

    if let Some(log_channel) = log_channel {
        while let Some(action) = reaction_collector.next().await {
            if let Err(e) =
                handle_checkin_reaction(&ctx, &db, &tournament, &log_channel, action).await
            {
                tracing::error!("Error during check-in handling: {}", e);
            }
        }
    }

    Ok(())
}

async fn handle_checkin_reaction(
    ctx: &Context,
    db: &Arc<LocalDatabase>,
    tournament: &TournamentEntry,
    log_channel: &GuildChannel,
    action: Arc<ReactionAction>,
) -> CommandResult {
    let confirm_emoji = ReactionType::Unicode(CONFIRM_EMOJI.to_string());

    let data_read = ctx.data.read().await;
    let mut invalid_checked_in = data_read
        .get::<IdCollection>()
        .expect("Expected database in TypeMap")
        .lock()
        .await;

    match action.as_ref() {
        ReactionAction::Added(reaction) | ReactionAction::Removed(reaction)
            if reaction.emoji == confirm_emoji =>
        {
            let discord_id = reaction.user_id.unwrap().0;

            // Prevent rate limit from unregistered people spamming reactions
            if invalid_checked_in.0.contains(&discord_id) {
                return Ok(());
            }

            let player = match db.players.get_player_by_discord(discord_id) {
                Ok(player) => match player {
                    Some(player) => player,
                    None => {
                        log_channel.say(&ctx.http, format!("<@{}> Your Discord user is not linked to a Tetrio account! You most likely haven't registered at all.", discord_id)).await?;
                        invalid_checked_in.0.insert(discord_id);
                        return Ok(());
                    }
                },
                Err(err) => {
                    log_channel.say(&ctx.http, err).await?;
                    return Ok(());
                }
            };

            let player_is_registered = tournament.player_is_registered(&player);

            let reply = match action.as_ref() {
                ReactionAction::Added(_) if player_is_registered => Some("You have checked-in successfully. Please stand by until the tournament begins. Instructions on how to play in the tournament will be posted once the bracket is finalized."),
                ReactionAction::Added(_) if !player_is_registered => {
                    invalid_checked_in.0.insert(discord_id);
                    Some("You weren't registered! Please do keep in mind that registering *(which happens in the week before the tournament)* and checking in *(which happens just before the tournament)* are two different processes.")
                },
                ReactionAction::Removed(_) if player_is_registered => Some("You have checked-out successfully. If you'd like to check back in, then react to the check-in message again."),
                _ => None
            };

            if let Some(reply) = reply {
                log_channel
                    .say(&ctx.http, format!("<@{}> {}", discord_id, reply))
                    .await?;
            }
        }
        _ => {}
    }

    Ok(())
}

#[command]
#[owners_only]
async fn export_check_in(ctx: &Context, msg: &Message) -> CommandResult {
    let db = crate::discord::get_database(&ctx).await;
    let tournament = match db.tournaments.get_active() {
        Ok(tournament) => match tournament {
            Some(tournament) => tournament,
            None => {
                msg.channel_id
                    .say(&ctx.http, "No active tournament")
                    .await?;
                return Ok(());
            }
        },
        Err(err) => {
            msg.channel_id.say(&ctx.http, err).await?;
            return Ok(());
        }
    };

    let confirm_emoji = ReactionType::Unicode(CONFIRM_EMOJI.to_string());

    let channel_id = 822933717453504562; // TODO: this is hardcoded but im lazy
    let message_id = match tournament.check_in_msg {
        Some(msg_id) => msg_id,
        None => {
            msg.channel_id
                .say(&ctx.http, "No check-in message found")
                .await?;
            return Ok(());
        }
    };

    let message = ctx.http.get_message(channel_id, message_id).await?;

    let mut users = Vec::new();
    const PAGE_SIZE: u8 = 100;

    loop {
        let mut page = message
            .reaction_users(
                &ctx.http,
                confirm_emoji.clone(),
                Some(PAGE_SIZE),
                users.last().map(|u: &User| u.id),
            )
            .await?;

        let is_incomplete_page = page.len() < PAGE_SIZE.into();
        users.append(&mut page);
        if is_incomplete_page {
            break;
        }
    }

    let user_ids: Vec<String> = users.iter().map(|u| u.id.0.to_string()).collect();
    let line_separated = user_ids.join("\n");

    // Send as txt file
    let attachment = AttachmentType::from((line_separated.as_bytes(), "checked_in.txt"));
    msg.channel_id
        .send_files(&ctx.http, vec![attachment], |m| m)
        .await?;

    Ok(())
}
