use serenity::builder::{CreateEmbed, CreateMessage};
use serenity::client::Context;
use serenity::framework::standard::CommandResult;
use serenity::model::channel::Message;
use serenity::model::id::GuildId;
use serenity::model::Timestamp;

pub fn to_time(secs: u64) -> String {
    let sec = (secs % 60) as u8;
    let min = ((secs / 60) % 60) as u8;
    let hrs = secs / 60 / 60;

    if hrs == 0 {
        return format!("{:0>2}:{:0>2}", min, sec);
    }
    format!("{}:{:0>2}:{:0>2}", hrs, min, sec)
}

pub fn get_guild_id_from_message(
    msg: &Message,
    ctx: &Context,
) -> Result<GuildId, &'static str> {
    msg.guild(&ctx.cache)
        .map(|guild| guild.id)
        .ok_or("Guild not found")
}

pub async fn send_success_message(
    ctx: &Context, msg: &Message, title: &str
) -> CommandResult {
    let embed = CreateEmbed::default()
        .color(0xffffff)
        .title(title)
        .timestamp(Timestamp::now());

    let builder = CreateMessage::default().add_embed(embed);

    msg.channel_id.send_message(&ctx.http, builder).await?;

    Ok(())
}

pub async fn send_warning(ctx: &Context, msg: &Message, title: &str) -> CommandResult {
    let embed = CreateEmbed::default()
        .color(0xf38ba8)
        .title(format!(":warning: {}", title))
        .timestamp(Timestamp::now());

    let builder = CreateMessage::default().add_embed(embed);

    msg.channel_id.send_message(&ctx.http, builder).await?;

    Ok(())
}

pub async fn send_error_message(ctx: &Context, msg: &Message, title: &str) -> CommandResult {
    let embed = CreateEmbed::default()
        .color(0xf38ba8)
        .title(format!(":error: {}", title))
        .timestamp(Timestamp::now());

    let builder = CreateMessage::default().add_embed(embed);

    msg.channel_id.send_message(&ctx.http, builder).await?;

    Ok(())
}
