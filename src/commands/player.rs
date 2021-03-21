use serde::Deserialize;
use serenity::framework::standard::{macros::command, Args, CommandResult};
use serenity::futures::StreamExt;
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

#[derive(Deserialize)]
struct FaqField {
    name: String,
    value: String,
}

#[derive(Deserialize)]
struct FaqEntry {
    title: String,
    description: String,
    fields: Option<Vec<FaqField>>,
}

const FAQ_FILE_PATH: &str = "./faq.json";
lazy_static! {
    static ref FAQ_ENTRIES: std::collections::HashMap<String, FaqEntry> = {
        let read_file = std::fs::File::open(FAQ_FILE_PATH).expect("file not there");
        let reader = std::io::BufReader::new(&read_file);
        serde_json::from_reader(reader).expect("bad json")
    };
}

#[command]
#[usage("[query]")]
#[example("apm")]
#[example("pps")]
/// Answers frequently asked questions regarding Tetrio and UC
///
/// Run without any arguments to view all available entries.
async fn faq(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    if let Some(arg) = args.current() {
        if let Some(entry) = FAQ_ENTRIES.get(&*arg.to_lowercase()) {
            msg.channel_id
                .send_message(&ctx.http, |m| {
                    m.embed(|e| {
                        e.title(&entry.title);
                        e.description(&entry.description);
                        if let Some(fields) = &entry.fields {
                            for f in fields {
                                e.field(f.name.clone(), f.value.clone(), false);
                            }
                        }
                        e
                    })
                })
                .await?;
            return Ok(());
        }
    }

    // entry not found or no query passed

    // TODO: Categorize the entries

    let mut keys: Vec<String> = FAQ_ENTRIES.keys().cloned().collect();
    keys.sort();

    msg.channel_id
        .send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.title("Frequently Asked Questions").field(
                    "Available queries",
                    keys.join(", "),
                    false,
                )
            })
        })
        .await?;

    Ok(())
}

#[command("whois")]
#[usage("[tetrio username]")]
#[example("caboozled_pie")]
#[example("icedynamix")]
/// Gets the Discord user linked with a given Tetr.io user and will also say whether the user is present on the server or not
async fn who_is(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let reply = match args.current() {
        Some(args) => {
            let db = crate::discord::get_database(ctx).await;
            match db.players.get_player_by_tetrio(args) {
                Ok(player) => match player {
                    Some(player) => match player.discord_id {
                        Some(discord_id) => {
                            let is_in_guild = msg
                                .guild_id
                                .unwrap()
                                .member(&ctx.http, discord_id)
                                .await
                                .is_ok();

                            if is_in_guild {
                                format!(
                                    "Tetr.io user `{}` is linked to <@{}> and is present on the server",
                                    args, discord_id
                                )
                            } else {
                                format!(
                                    "Tetr.io user `{}` is linked to <@{}> and is **not** present on the server",
                                    args, discord_id
                                )
                            }
                        }
                        None => {
                            format!("Tetr.io user `{}` is not linked to any Discord user", args)
                        }
                    },
                    None => format!("Tetr.io user `{}` was not found", args),
                },
                Err(err) => {
                    tracing::warn!("{}", err);
                    err.to_string()
                }
            }
        }
        None => "No username provided".to_string(),
    };

    msg.channel_id.say(&ctx.http, reply).await?;

    Ok(())
}
