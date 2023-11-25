use serenity::framework::standard::macros::command;
use serenity::framework::standard::CommandResult;
use serenity::http::Http;
use serenity::model::prelude::*;
use serenity::prelude::*;
use songbird::tracks::TrackHandle;

use crate::commands::utils::{
    get_guild_id_from_message, get_songbird_manager, send_warning, to_time,
};

#[command]
#[only_in(guilds)]
async fn nowplaying(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = get_guild_id_from_message(msg, &ctx).await?;

    let manager = get_songbird_manager(ctx).await?;

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();

        match queue.current() {
            Some(current) => display_track_info(&ctx.http, msg.channel_id, &current).await?,
            None => {
                send_warning(&ctx.http, msg.channel_id, "Nothing is playing right now.").await?
            }
        }
    } else {
        send_warning(
            &ctx.http,
            msg.channel_id,
            "Currently not in a voice channel.",
        )
        .await?;
    }

    Ok(())
}

async fn display_track_info(
    http: &Http,
    channel_id: ChannelId,
    track: &TrackHandle,
) -> CommandResult {
    let metadata = track.metadata();
    let track_info = track.get_info().await.unwrap();

    let date_formatted = metadata
        .date
        .as_ref()
        .map(|date| format!("{}/{}/{}", &date[6..8], &date[4..6], &date[0..4]))
        .unwrap_or_else(|| "Unknown date".into());

    let time_formatted = format!(
        "{} - {}",
        to_time(track_info.position.as_secs()),
        to_time(metadata.duration.unwrap().as_secs())
    );

    channel_id
        .send_message(http, |m| {
            m.embed(|e| {
                e.colour(0xffffff)
                    .title(
                        metadata
                            .title
                            .clone()
                            .unwrap_or_else(|| "Unknown Title".into()),
                    )
                    .thumbnail(metadata.thumbnail.clone().unwrap())
                    .url(metadata.source_url.clone().unwrap())
                    .fields(vec![
                        (
                            "Artist",
                            metadata
                                .artist
                                .clone()
                                .unwrap_or_else(|| "Unknown Artist".into()),
                            false,
                        ),
                        ("Released", date_formatted, true),
                        ("Position", time_formatted, true),
                        ("Status", format!("{:?}", track_info.playing), true),
                    ])
                    .timestamp(Timestamp::now())
            })
        })
        .await?;

    Ok(())
}
