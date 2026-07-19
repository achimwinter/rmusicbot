use poise::serenity_prelude::{CreateEmbed, Timestamp};
use poise::CreateReply;
use songbird::tracks::TrackHandle;

use crate::commands::utils::{get_guild_id, send_warning, to_time};
use crate::{CommandResult, Context};

#[poise::command(prefix_command, guild_only)]
pub async fn current(ctx: Context<'_>) -> CommandResult {
    let guild_id = match get_guild_id(ctx) {
        Ok(id) => id,
        Err(_) => {
            send_warning(ctx, "Guild not found.").await?;
            return Ok(());
        }
    };

    let songbird_client = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = songbird_client.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();

        match queue.current() {
            Some(current) => display_track_info(ctx, &current).await?,
            None => send_warning(ctx, "Nothing is playing right now.").await?,
        }
    } else {
        send_warning(ctx, "Currently not in a voice channel.").await?;
    }

    Ok(())
}

async fn display_track_info(ctx: Context<'_>, track: &TrackHandle) -> CommandResult {
    let track_info = track.get_info().await.unwrap();

    let time_formatted = to_time(track_info.position.as_secs());

    let embed = CreateEmbed::default()
        .color(0xffffff)
        .title("Now Playing")
        .field("Position", &time_formatted, true)
        .field("Status", format!("{:?}", track_info.playing), true)
        .timestamp(Timestamp::now());

    ctx.send(CreateReply::default().embed(embed)).await?;

    Ok(())
}
