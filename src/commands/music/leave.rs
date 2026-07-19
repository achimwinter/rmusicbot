use crate::commands::utils::{get_guild_id, send_error_message, send_success_message};
use crate::{CommandResult, Context};

#[poise::command(prefix_command, guild_only)]
pub async fn leave(ctx: Context<'_>) -> CommandResult {
    let guild_id = get_guild_id(ctx)?;

    let songbird_client = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if songbird_client.get(guild_id).is_some() {
        if let Err(e) = songbird_client.remove(guild_id).await {
            send_error_message(ctx, &format!("Error leaving voice channel: {}", e)).await?;
            return Ok(());
        }
        send_success_message(ctx, "Left voice channel!").await?;
    } else {
        send_error_message(ctx, ":warning: Not in a voice channel.").await?;
    }

    Ok(())
}
