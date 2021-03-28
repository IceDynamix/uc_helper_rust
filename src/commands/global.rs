use serde::Deserialize;
use serenity::framework::standard::{macros::command, Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::discord;

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
                            let is_in_guild = GuildId(discord::UC_GUILD_ID)
                                .member(&ctx.http, discord_id)
                                .await
                                .is_ok();

                            if is_in_guild {
                                format!(
                                    "Tetr.io user `{}` is linked to <@{}> and is present on the server",
                                    args, discord_id
                                )
                            } else {
                                let mut reply = format!(
                                    "Tetr.io user `{}` is linked to <@{}> and is **not** present on the server",
                                    args, discord_id
                                );

                                if msg.guild_id.is_none() {
                                    reply.push_str(" *or you're using some test environment*")
                                }

                                reply
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
