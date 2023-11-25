use serenity::framework::standard::macros::command;
use serenity::framework::standard::CommandResult;
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::commands::utils::{
    get_guild_id_from_message, get_songbird_manager, send_error_message, send_success_message,
};

#[command]
#[only_in(guilds)]
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = get_guild_id_from_message(msg, ctx).await?;

    let manager = get_songbird_manager(ctx).await?;

    if let Some(_) = manager.get(guild_id) {
        if let Err(e) = manager.remove(guild_id).await {
            send_error_message(
                &ctx.http,
                msg.channel_id,
                &format!("Error leaving voice channel: {}", e),
            )
            .await?;
            return Ok(());
        }
        send_success_message(&ctx.http, msg.channel_id, "Left voice channel!").await?;
    } else {
        send_error_message(
            &ctx.http,
            msg.channel_id,
            ":warning: Not in a voice channel.",
        )
        .await?;
    }

    Ok(())
}
