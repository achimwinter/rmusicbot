use crate::commands::utils::{get_guild_id, send_error_message, send_success_message};
use crate::{CommandResult, Context};

#[poise::command(prefix_command, guild_only)]
pub async fn pause(ctx: Context<'_>) -> CommandResult {
    let guild_id = get_guild_id(ctx)?;

    let manager = match songbird::get(ctx.serenity_context()).await {
        Some(m) => m,
        None => {
            send_error_message(ctx, "Songbird client missing.").await?;
            return Ok(());
        }
    };

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();

        if let Err(e) = queue.pause() {
            println!("Error pausing track: {}", e);
            send_error_message(ctx, "Error pausing track.").await?;
        } else {
            send_success_message(ctx, ":pause_button: Paused!").await?;
        }
    } else {
        send_error_message(ctx, "Currently not in a voice channel.").await?;
    }

    Ok(())
}
