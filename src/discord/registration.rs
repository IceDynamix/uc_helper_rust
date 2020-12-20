use serenity::framework::standard::{macros::command, Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::database::DatabaseError;
use crate::database::*;

#[command]
#[only_in(guilds)]
#[description("Registers you to the current tournament")]
#[max_args(1)]
async fn register(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let (response, retry) = register_wrapped(ctx, msg, &args).await;
    msg.channel_id.say(&ctx.http, response).await?;
    if retry {
        let (response, _) = register_wrapped(ctx, msg, &args).await;
        msg.channel_id.say(&ctx.http, response).await?;
    }

    Ok(())
}

async fn register_wrapped(ctx: &Context, msg: &Message, args: &Args) -> (String, bool) {
    match discord::get_from_discord_id(msg.author.id.0).await {
        Ok(entry) => {
            let tetrio_data = players::get(&entry.tetrio_id)
                .await
                .expect("Data of linked account could not be found?");

            if let Err(e) = tetrio_data.can_participate() {
                return (format!("🟥 {}", e.to_string()), false);
            }

            if args.is_empty() || tetrio_data.username == args.rest() {
                match registration::register(msg.author.id.0, &tetrio_data._id).await {
                    Ok(_) => (
                        format!(
                            "🟩 Successfully registered {} for the tournament",
                            tetrio_data.username
                        ),
                        false,
                    ),
                    Err(DatabaseError::DuplicateEntry) => (
                        "🟥 Someone else with that tetrio username has already registered!"
                            .to_string(),
                        false,
                    ),
                    Err(e) => (e.to_string(), false),
                }
            } else {
                (
                    "🟥 The linked Tetrio account is different to the provided username, please relink `.link <username>` if necessary!"
                        .to_string(),
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
    let response = match registration::unregister_discord(msg.author.id.0).await {
        Ok(_) => "Unlinked Discord user from Tetrio user",
        Err(DatabaseError::NotFound) => "User not registered",
        Err(_) => "Connection to database failed",
    };

    msg.channel_id.say(&ctx.http, response).await?;

    Ok(())
}

#[command]
#[only_in(guilds)]
#[description("Unregisters you from the current tournament")]
#[num_args(1)]
#[owners_only]
pub async fn staff_unregister(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let response = match registration::unregister_tetrio(args.rest()).await {
        Ok(_) => "Unlinked Discord user from Tetrio user",
        Err(DatabaseError::NotFound) => "User not registered",
        Err(_) => "Connection to database failed",
    };

    msg.channel_id.say(&ctx.http, response).await?;

    Ok(())
}

#[command]
#[only_in(guilds)]
#[description("Unregisters you from the current tournament")]
pub async fn can_participate(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let response = if args.is_empty() {
        format!("🟥 {}", "Please provide a username!")
    } else {
        let tetrio_data = players::get(args.rest()).await;
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
    msg.channel_id.say(&ctx.http, response).await?;
    Ok(())
}
