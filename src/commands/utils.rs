use poise::serenity_prelude::{CreateEmbed, GuildId, Timestamp};
use poise::CreateReply;

use crate::{CommandResult, Context};

pub fn to_time(secs: u64) -> String {
    let sec = (secs % 60) as u8;
    let min = ((secs / 60) % 60) as u8;
    let hrs = secs / 60 / 60;

    if hrs == 0 {
        return format!("{:0>2}:{:0>2}", min, sec);
    }
    format!("{}:{:0>2}:{:0>2}", hrs, min, sec)
}

pub fn get_guild_id(ctx: Context<'_>) -> Result<GuildId, &'static str> {
    ctx.guild_id().ok_or("Guild not found")
}

pub async fn send_success_message(ctx: Context<'_>, title: &str) -> CommandResult {
    let embed = CreateEmbed::default()
        .color(0xffffff)
        .title(title)
        .timestamp(Timestamp::now());

    ctx.send(CreateReply::default().embed(embed)).await?;

    Ok(())
}

pub async fn send_warning(ctx: Context<'_>, title: &str) -> CommandResult {
    let embed = CreateEmbed::default()
        .color(0xf38ba8)
        .title(format!(":warning: {}", title))
        .timestamp(Timestamp::now());

    ctx.send(CreateReply::default().embed(embed)).await?;

    Ok(())
}

pub async fn send_error_message(ctx: Context<'_>, title: &str) -> CommandResult {
    let embed = CreateEmbed::default()
        .color(0xf38ba8)
        .title(format!(":error: {}", title))
        .timestamp(Timestamp::now());

    ctx.send(CreateReply::default().embed(embed)).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_time_formats_seconds_only() {
        assert_eq!(to_time(0), "00:00");
        assert_eq!(to_time(9), "00:09");
        assert_eq!(to_time(59), "00:59");
    }

    #[test]
    fn to_time_formats_minutes_and_seconds() {
        assert_eq!(to_time(60), "01:00");
        assert_eq!(to_time(125), "02:05");
        assert_eq!(to_time(3599), "59:59");
    }

    #[test]
    fn to_time_formats_hours_minutes_seconds() {
        assert_eq!(to_time(3600), "1:00:00");
        assert_eq!(to_time(3661), "1:01:01");
        assert_eq!(to_time(90061), "25:01:01");
    }
}
