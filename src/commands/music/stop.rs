use crate::commands::utils::{get_guild_id, send_error_message, send_success_message};
use crate::{CommandResult, Context};

#[poise::command(prefix_command, guild_only)]
pub async fn stop(ctx: Context<'_>) -> CommandResult {
    let guild_id = match get_guild_id(ctx) {
        Ok(id) => id,
        Err(_) => {
            send_error_message(ctx, "Guild not found.").await?;
            return Ok(());
        }
    };

    let manager = match songbird::get(ctx.serenity_context()).await {
        Some(manager) => manager,
        None => {
            send_error_message(ctx, "Songbird Voice client not initialized.").await?;
            return Ok(());
        }
    };

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        queue.stop();

        send_success_message(ctx, ":stop_button: Playlist stopped!").await?;
    } else {
        send_error_message(ctx, "Not in a voice channel.").await?;
    }

    Ok(())
}
