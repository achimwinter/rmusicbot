use serenity::builder::{CreateEmbed, CreateMessage};
use serenity::framework::standard::macros::command;
use serenity::framework::standard::CommandResult;
use serenity::model::prelude::*;
use serenity::model::Timestamp;
use serenity::prelude::*;
use songbird::tracks::TrackHandle;

use crate::commands::utils::{send_warning, to_time};

#[command]
#[only_in(guilds)]
async fn current(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild(&ctx.cache).map(|g| g.id);
    let guild_id = match guild_id {
        Some(id) => id,
        None => {
            send_warning(ctx, msg, "Guild not found.").await?;
            return Ok(());
        }
    };

    let songbird_client = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = songbird_client.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();

        match queue.current() {
            Some(current) => display_track_info(ctx, msg, &current).await?,
            None => {
                send_warning(ctx, msg, "Nothing is playing right now.").await?
            }
        }
    } else {
        send_warning(ctx, msg, "Currently not in a voice channel.").await?;
    }

    Ok(())
}

async fn display_track_info(
    ctx: &Context,
    msg: &Message,
    track: &TrackHandle,
) -> CommandResult {
    let track_info = track.get_info().await.unwrap();

    let time_formatted = to_time(track_info.position.as_secs());

    let embed = CreateEmbed::default()
        .color(0xffffff)
        .title("Now Playing")
        .field("Position", &time_formatted, true)
        .field("Status", format!("{:?}", track_info.playing), true)
        .timestamp(Timestamp::now());

    let builder = CreateMessage::default().add_embed(embed);
    msg.channel_id.send_message(&ctx.http, builder).await?;

    Ok(())
}
