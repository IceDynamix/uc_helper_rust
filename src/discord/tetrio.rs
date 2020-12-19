use chrono::SecondsFormat;
use serenity::framework::standard::{macros::command, Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::tetrio::database::players::PlayerEntry;
use crate::tetrio::database::DatabaseError;
use crate::tetrio::database::*;

#[command]
#[only_in(guilds)]
#[description("Links your Discord User to a Tetrio User")]
#[num_args(1)]
async fn link(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let username = args.rest();
    let result = discord::link(msg.author.id.0, args.rest()).await;
    match result {
        Ok(user) => {
            msg.channel_id
                .say(
                    &ctx.http,
                    format!("Linked Discord user with Tetrio user {}", user.username),
                )
                .await?;

            change_nickname(ctx, msg, &user.username).await?;
        }
        Err(DatabaseError::NotFound) => {
            msg.channel_id
                .say(&ctx.http, format!("User {} not found", username))
                .await?;
        }
        Err(DatabaseError::DuplicateEntry) => {
            msg.channel_id
                .say(
                    &ctx.http,
                    "Another Discord user has already linked this Tetrio user",
                )
                .await?;
        }
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "Connection to database failed")
                .await?;
        }
    };
    Ok(())
}

#[command]
#[only_in(guilds)]
#[description("Unlinks your Discord User from a Tetrio User")]
async fn unlink(ctx: &Context, msg: &Message) -> CommandResult {
    match discord::unlink(msg.author.id.0).await {
        Ok(_) => {
            msg.channel_id
                .say(&ctx.http, "Unlinked Discord user from Tetrio user")
                .await?;
            change_nickname(ctx, msg, "").await?;
        }
        Err(DatabaseError::NotFound) => {
            msg.channel_id.say(&ctx.http, "User not found").await?;
        }
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "Connection to database failed")
                .await?;
        }
    };

    Ok(())
}

async fn change_nickname(ctx: &Context, msg: &Message, nickname: &str) -> CommandResult {
    // Check should make sure it's always in a guild
    let member = msg.member(&ctx.http).await.expect("Not in guild");
    if let Err(e) = member.edit(&ctx.http, |m| m.nickname(nickname)).await {
        msg.channel_id
            .say(
                &ctx.http,
                format!("Could not change nickname, reason: {}", e),
            )
            .await?;
    }
    Ok(())
}

#[command]
#[only_in(guilds)]
#[description("Get stats of a particular Tetrio or Discord user")]
#[max_args(1)]
async fn stats(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let data = if args.is_empty() {
        let id = msg.author.id.0;
        let author_name = msg
            .member
            .clone()
            .expect("not in guild")
            .nick
            .unwrap_or_else(|| msg.author.name.clone());

        let lookup_value = discord::get_from_discord_id(id)
            .await
            .unwrap_or(author_name);

        lookup(ctx, msg, &lookup_value).await
    } else if let Some(mentioned_id) = serenity::utils::parse_mention(args.rest()) {
        let tetrio_id = discord::get_from_discord_id(mentioned_id).await;
        match tetrio_id {
            Ok(lookup_value) => lookup(ctx, msg, &lookup_value).await,
            Err(DatabaseError::NotFound) => {
                msg.channel_id
                    .say(
                        &ctx.http,
                        "Discord user isn't linked to a Tetrio user, use `.link <username>`",
                    )
                    .await?;
                None
            }
            Err(_) => {
                msg.channel_id
                    .say(&ctx.http, "Connection to database failed")
                    .await?;
                None
            }
        }
    } else {
        lookup(ctx, msg, args.rest()).await
    };

    if let Some(entry) = data {
        msg.channel_id
            .send_message(&ctx.http, |m| {
                m.embed(|e| {
                    e.title(&entry.username);
                    e.url(format!("https://ch.tetr.io/u/{}", &entry.username));

                    let league = &entry.data.league;

                    e.color(
                        u64::from_str_radix(
                            crate::tetrio::Rank::from_str(&league.rank).to_color(),
                            16,
                        )
                        .unwrap_or(0),
                    );

                    e.thumbnail(format!(
                        "https://tetrio.team2xh.net/images/ranks/{}.png",
                        &entry.data.league.rank
                    ));

                    e.fields(vec![
                        (
                            "Tetra Rating",
                            format!("{:.0} ± {}", &league.rating, &league.rd.unwrap_or_default()),
                            false,
                        ),
                        (
                            "APM",
                            format!("{:.2}", &league.apm.unwrap_or_default()),
                            true,
                        ),
                        (
                            "PPS",
                            format!("{:.2}", &league.pps.unwrap_or_default()),
                            true,
                        ),
                        ("VS", format!("{:.2}", &league.vs.unwrap_or_default()), true),
                    ]);

                    e.timestamp(
                        chrono::DateTime::parse_from_rfc3339(&entry.timestamp)
                            .expect("Bad timestamp")
                            .to_rfc3339_opts(SecondsFormat::Secs, false),
                    );

                    e
                })
            })
            .await?;
    }

    Ok(())
}

async fn lookup(ctx: &Context, msg: &Message, username: &str) -> Option<PlayerEntry> {
    match players::get(username).await {
        Ok(user) => Some(user),
        Err(DatabaseError::NotFound) => {
            msg.channel_id
                .say(&ctx.http, format!("User {} not found", username))
                .await
                .ok()?;
            None
        }
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "Connection to database failed")
                .await
                .ok()?;
            None
        }
    }
}
