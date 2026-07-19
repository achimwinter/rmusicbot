use serenity::framework::standard::macros::command;
use serenity::framework::standard::CommandResult;
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::commands::utils::{get_guild_id_from_message, send_error_message, send_success_message};

#[command]
#[only_in(guilds)]
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = get_guild_id_from_message(msg, ctx)?;

    let songbird_client = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if songbird_client.get(guild_id).is_some() {
        if let Err(e) = songbird_client.remove(guild_id).await {
            send_error_message(ctx, msg, &format!("Error leaving voice channel: {}", e)).await?;
            return Ok(());
        }
        send_success_message(ctx, msg, "Left voice channel!").await?;
    } else {
        send_error_message(ctx, msg, ":warning: Not in a voice channel.").await?;
    }

    Ok(())
}
