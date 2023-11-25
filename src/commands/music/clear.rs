use serenity::{
    framework::standard::{macros::command, CommandResult},
    model::prelude::*,
    prelude::*,
};

use crate::commands::utils::get_songbird_manager;

#[command]
#[only_in(guilds)]
async fn clear(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();

    let songbird_client = get_songbird_manager(ctx).await?;

    match songbird_client.get(guild_id) {
        Some(handler_lock) => {
            let handler = handler_lock.lock().await;
            handler.queue().stop();
            send_clear_message(ctx, msg, 0xffffff, "Queue emptied!").await?;
        }
        None => {
            send_clear_message(ctx, msg, 0xf38ba8, ":warning: Not in voice channel.").await?;
        }
    }
    Ok(())
}

async fn send_clear_message(
    ctx: &Context,
    msg: &Message,
    color: u32,
    title: &str,
) -> CommandResult {
    msg.channel_id
        .send_message(&ctx.http, |m| {
            m.embed(|e| e.colour(color).title(title).timestamp(Timestamp::now()))
        })
        .await?;
    Ok(())
}
