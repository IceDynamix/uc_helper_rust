use serenity::framework::standard::{macros::command, Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::database::players::PlayerEntry;
use crate::database::tournament::RegistrationEntry;
use crate::database::DatabaseError;
use crate::database::*;
use crate::tetrio::Rank;

#[command]
#[only_in(guilds)]
#[description("Registers you to the current tournament")]
#[max_args(1)]
async fn register(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let (response, retry) = register_wrapped(ctx, msg, &args).await;
    // TODO Replace with message builder
    msg.channel_id
        .say(&ctx.http, format!("<@{}> {}", msg.author.id.0, response))
        .await?;
    if retry {
        let (response, _) = register_wrapped(ctx, msg, &args).await;
        // TODO Replace with message builder
        msg.channel_id
            .say(&ctx.http, format!("<@{}> {}", msg.author.id.0, response))
            .await?;
    }

    Ok(())
}

async fn register_wrapped(ctx: &Context, msg: &Message, args: &Args) -> (String, bool) {
    match discord::get_from_discord_id(msg.author.id.0).await {
        Ok(entry) => {
            let tetrio_data = players::get_player(&entry.tetrio_id)
                .await
                .expect("Data of linked account could not be found?");

            if let Err(e) = tetrio_data.can_participate() {
                return (format!("🟥 {}", e.to_string()), false);
            }

            if args.is_empty() || tetrio_data.username.to_lowercase() == args.rest().to_lowercase()
            {
                match tournament::register(msg.author.id.0, &tetrio_data._id).await {
                    Ok(_) => (
                        format!(
                            "🟩 Successfully registered {} for the tournament",
                            tetrio_data.username
                        ),
                        false,
                    ),
                    Err(DatabaseError::DuplicateEntry) => (
                        format!(
                            "🟥 Someone else with the tetrio username {} has already registered!",
                            tetrio_data.username
                        ),
                        false,
                    ),
                    Err(e) => (e.to_string(), false),
                }
            } else {
                (
                    format!("🟥 The linked Tetrio account is different to the provided username {}, please relink `.link <username>` if necessary!", tetrio_data.username),
                    false
                )
            }
        }

        Err(DatabaseError::NotFound) => {
            if args.is_empty() {
                (
                    "🟥 Please provide a username or link your account with `.link <username>` first!"
                        .to_string(),
                    false,
                )
            } else {
                let (result, user) = super::tetrio::link_action(ctx, msg, &args)
                    .await
                    .unwrap_or(("Something went wrong".to_string(), None));

                (result, user.is_some())
            }
        }

        Err(_) => ("Connection to database failed".to_string(), false),
    }
}

#[command]
#[only_in(guilds)]
#[description("Unregisters you from the current tournament")]
pub async fn unregister(ctx: &Context, msg: &Message) -> CommandResult {
    let response = match tournament::unregister_discord(msg.author.id.0).await {
        Ok(_) => "Unregistered from tournament",
        Err(DatabaseError::NotFound) => "User not registered",
        Err(_) => "Connection to database failed",
    };

    // TODO Replace with message builder
    msg.channel_id
        .say(&ctx.http, format!("<@{}> {}", msg.author.id.0, response))
        .await?;

    Ok(())
}

#[command]
#[only_in(guilds)]
#[description("Unregisters you from the current tournament")]
#[num_args(1)]
#[owners_only]
pub async fn staff_unregister(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let response = match tournament::unregister_tetrio(args.rest()).await {
        Ok(_) => "Unregistered from tournament",
        Err(DatabaseError::NotFound) => "User not registered",
        Err(_) => "Connection to database failed",
    };

    // TODO Replace with message builder
    msg.channel_id
        .say(&ctx.http, format!("<@{}> {}", msg.author.id.0, response))
        .await?;

    Ok(())
}

#[command]
#[only_in(guilds)]
#[description("Unregisters you from the current tournament")]
pub async fn can_participate(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let response = if args.is_empty() {
        format!("🟥 {}", "Please provide a username!")
    } else {
        let tetrio_data = players::get_player(args.rest()).await;
        match tetrio_data {
            Ok(data) => {
                if let Err(e) = data.can_participate() {
                    format!("🟥 {}", e.to_string())
                } else {
                    format!("🟩 {}", "You can participate!")
                }
            }
            Err(DatabaseError::NotFound) => format!("🟥 {}", "User could not be found"),
            Err(e) => format!("🟥 {}", e.to_string()),
        }
    };
    // TODO Replace with message builder
    msg.channel_id
        .say(&ctx.http, format!("<@{}> {}", msg.author.id.0, response))
        .await?;
    Ok(())
}

#[command]
#[only_in(guilds)]
#[description("Shows all players currently registered in the tournament")]
#[aliases("playerlist", "list")]
async fn player_list(ctx: &Context, msg: &Message) -> CommandResult {
    let player_entries = get_all::<PlayerEntry>(players::COLLECTION).await?;
    let registration_entries = get_all::<RegistrationEntry>(tournament::COLLECTION).await?;
    let participant_ids: Vec<String> = registration_entries
        .iter()
        .map(|e| e.tetrio_id.clone())
        .collect();

    // TODO: replace with sorted list
    let participants: Vec<&PlayerEntry> = player_entries
        .iter()
        .filter(|player| participant_ids.contains(&player._id))
        .collect();

    msg.channel_id
        .send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.title("Player List");
                e.description(format!("{} participants", registration_entries.len()));

                for rank in Rank::iter().rev() {
                    let mut valid: Vec<&&PlayerEntry> = participants
                        .iter()
                        .filter(|p| &Rank::from_str(&p.data.league.rank) == rank)
                        .collect();
                    if valid.is_empty() {
                        continue;
                    }
                    let content = if !valid.is_empty() {
                        valid.sort_by_key(|p| Rank::from_str(&p.data.league.rank));
                        valid
                            .iter()
                            .map(|p| p.username.to_owned())
                            .collect::<Vec<String>>()
                            .join(", ")
                    } else {
                        "-".to_string()
                    };
                    let title = format!("{} {}", rank.to_emoji(), valid.len());
                    e.field(title, content, true);
                }
                e
            });
            m
        })
        .await?;

    Ok(())
}
