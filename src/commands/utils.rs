use std::sync::Arc;

use serenity::framework::standard::CommandResult;
use serenity::http::Http;
use serenity::model::channel::Message;
use serenity::model::prelude::*;
use serenity::prelude::*;
use songbird::Songbird;

pub fn to_time(secs: u64) -> String {
    let sec = (secs % 60) as u8;
    let min = ((secs / 60) % 60) as u8;
    let hrs = secs / 60 / 60;

    if hrs == 0 {
        return format!("{:0>2}:{:0>2}", min, sec);
    }
    format!("{}:{:0>2}:{:0>2}", hrs, min, sec)
}

pub async fn get_guild_id_from_message(
    msg: &Message,
    ctx: &Context,
) -> Result<GuildId, &'static str> {
    msg.guild(&ctx.cache)
        .map(|guild| Ok(guild.id))
        .unwrap_or_else(|| Err("Guild not found"))
}

pub async fn get_songbird_manager(ctx: &Context) -> Result<Arc<Songbird>, &'static str> {
    songbird::get(ctx).await.ok_or("Songbird client missing")
}

pub async fn send_success_message(
    http: &Http,
    channel_id: ChannelId,
    message: &str,
) -> CommandResult {
    channel_id
        .send_message(http, |m| {
            m.embed(|e| {
                e.colour(0xffffff)
                    .title(message)
                    .timestamp(Timestamp::now())
            })
        })
        .await?;
    Ok(())
}

pub async fn send_warning(http: &Http, channel_id: ChannelId, message: &str) -> CommandResult {
    channel_id
        .send_message(http, |m| {
            m.embed(|e| {
                e.colour(0xf38ba8)
                    .title(format!(":warning: {}", message))
                    .timestamp(Timestamp::now())
            })
        })
        .await?;
    Ok(())
}

pub async fn send_error_message(
    http: &Http,
    channel_id: ChannelId,
    message: &str,
) -> CommandResult {
    channel_id
        .send_message(http, |m| {
            m.embed(|e| {
                e.colour(0xf38ba8)
                    .title(message)
                    .timestamp(Timestamp::now())
            })
        })
        .await?;
    Ok(())
}
