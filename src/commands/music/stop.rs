use serenity::model::channel::Message;
use serenity::{framework::standard::macros::command, client::Context};
use serenity::framework::standard::CommandResult;

use crate::commands::utils::{send_error_message, send_success_message};

#[command]
#[only_in(guilds)]
async fn stop(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild(&ctx.cache).map(|g| g.id);
    let guild_id = match guild_id {
        Some(id) => id,
        None => {
            send_error_message(ctx, msg, "Guild not found.").await?;
            return Ok(());
        }
    };

    let manager = match songbird::get(ctx).await {
        Some(manager) => manager,
        None => {
            send_error_message(
                ctx,
                msg,
                "Songbird Voice client not initialized.",
            )
            .await?;
            return Ok(());
        }
    };

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        queue.stop();

        send_success_message(ctx, msg, ":stop_button: Playlist stopped!").await?;
    } else {
        send_error_message(ctx, msg, "Not in a voice channel.").await?;
    }

    Ok(())
}
