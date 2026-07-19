use poise::serenity_prelude::{CreateEmbed, Timestamp};
use poise::CreateReply;

use crate::commands::utils::get_guild_id;
use crate::{CommandResult, Context};

#[poise::command(prefix_command, guild_only)]
pub async fn clear(ctx: Context<'_>) -> CommandResult {
    let guild_id = get_guild_id(ctx)?;

    let songbird_client = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    match songbird_client.get(guild_id) {
        Some(handler_lock) => {
            let handler = handler_lock.lock().await;
            handler.queue().stop();
            send_clear_message(ctx, 0xffffff, "Queue emptied!").await?;
        }
        None => {
            send_clear_message(ctx, 0xf38ba8, ":warning: Not in voice channel.").await?;
        }
    }
    Ok(())
}

async fn send_clear_message(ctx: Context<'_>, color: u32, title: &str) -> CommandResult {
    let embed = CreateEmbed::default()
        .colour(color)
        .title(title)
        .timestamp(Timestamp::now());

    ctx.send(CreateReply::default().embed(embed)).await?;
    Ok(())
}
