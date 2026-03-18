use serenity::framework::standard::macros::command;
use serenity::framework::standard::CommandResult;
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::commands::utils::{get_guild_id_from_message, send_error_message, send_success_message};

#[command]
#[only_in(guilds)]
async fn pause(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = get_guild_id_from_message(msg, ctx)?;

    let manager = match songbird::get(ctx).await {
        Some(m) => m,
        None => {
            send_error_message(&ctx, msg, "Songbird client missing.").await?;
            return Ok(());
        }
    };

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();

        if let Err(e) = queue.pause() {
            println!("Error pausing track: {}", e);
            send_error_message(&ctx, msg, "Error pausing track.").await?;
        } else {
            send_success_message(&ctx, msg, ":pause_button: Paused!").await?;
        }
    } else {
        send_error_message(
            &ctx,
            msg,
            "Currently not in a voice channel.",
        )
        .await?;
    }

    Ok(())
}
