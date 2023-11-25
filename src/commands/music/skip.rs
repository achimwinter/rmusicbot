use serenity::framework::standard::macros::command;
use serenity::framework::standard::CommandResult;
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::commands::utils::{send_error_message, send_success_message};

#[command]
#[only_in(guilds)]
async fn skip(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = match msg.guild(&ctx.cache) {
        Some(guild) => guild.id,
        None => {
            send_error_message(&ctx.http, msg.channel_id, "Guild not found.").await?;
            return Ok(());
        }
    };

    let manager = match songbird::get(ctx).await {
        Some(manager) => manager,
        None => {
            send_error_message(&ctx.http, msg.channel_id, "Songbird client missing.").await?;
            return Ok(());
        }
    };

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        let _ = queue.skip();

        send_success_message(&ctx.http, msg.channel_id, ":track_next: Skipped!").await?;
    } else {
        send_error_message(&ctx.http, msg.channel_id, "Not in a voice channel.").await?;
    }

    Ok(())
}
