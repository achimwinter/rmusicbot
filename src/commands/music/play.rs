use regex::Regex;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;
use songbird::input::Restartable;
use songbird::Call;
use tokio::process::Command as TokioCommand;
use tracing::error;

use crate::commands::utils::send_error_message;

#[command]
#[aliases(p)]
#[only_in(guilds)]
async fn play(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let url = match get_url_from_args(&args) {
        Some(url) => url,
        None => {
            send_error_message(
                &ctx.http,
                msg.channel_id,
                "Use the command like this: play <url> or <song name>",
            )
            .await?;
            return Ok(());
        }
    };

    let guild_id = match get_guild_id(msg, ctx).await {
        Ok(id) => id,
        Err(_) => {
            send_error_message(&ctx.http, msg.channel_id, "Guild not found").await?;
            return Ok(());
        }
    };

    let manager = match songbird::get(ctx).await {
        Some(manager) => manager,
        None => {
            send_error_message(&ctx.http, msg.channel_id, "Songbird client missing").await?;
            return Ok(());
        }
    };

    if manager.get(guild_id).is_none() {
        if let Err(_) = join_channel_if_needed(&ctx, msg, guild_id).await {
            send_error_message(&ctx.http, msg.channel_id, "Error joining the voice channel")
                .await?;
            return Ok(());
        }
    }

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;

        if !url.starts_with("http") {
            search_and_play_single_track(ctx, msg, &mut handler, &url).await?;
        } else if url.contains("index") {
            play_playlist(ctx, msg, &mut handler, &url).await?;
        } else if url.contains("live") {
            play_live_stream(ctx, msg, &mut handler, &url).await?;
        } else {
            play_direct_link(ctx, msg, &mut handler, &url).await?;
        }
    }

    Ok(())
}

fn get_url_from_args(args: &Args) -> Option<String> {
    args.clone().single::<String>().ok()
}

async fn get_guild_id(msg: &Message, ctx: &Context) -> Result<GuildId, &'static str> {
    msg.guild(&ctx.cache)
        .map(|guild| guild.id)
        .ok_or("Guild not found")
}

async fn join_channel_if_needed(ctx: &Context, msg: &Message, guild_id: GuildId) -> CommandResult {
    let guild = match msg.guild(&ctx.cache) {
        Some(guild) => guild,
        None => {
            return Err("Guild not found".into());
        }
    };

    let channel_id = match guild
        .voice_states
        .get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id)
    {
        Some(channel) => channel,
        None => {
            send_error_message(&ctx.http, msg.channel_id, "Join a voice channel first!").await?;
            return Ok(());
        }
    };

    let manager = match songbird::get(ctx).await {
        Some(manager) => manager,
        None => {
            send_error_message(&ctx.http, msg.channel_id, "Songbird client missing").await?;
            return Ok(());
        }
    };

    let (_, success) = manager.join(guild_id, channel_id).await;

    if success.is_ok() {
        Ok(())
    } else {
        send_error_message(&ctx.http, msg.channel_id, "Error joining the voice channel").await?;
        Err("Error joining the voice channel".into())
    }
}

async fn search_and_play_single_track(
    ctx: &Context,
    msg: &Message,
    handler: &mut Call,
    query: &str,
) -> CommandResult {
    let source = match songbird::input::ytdl_search(query).await {
        Ok(source) => source,
        Err(why) => {
            error!("Error starting source: {:?}", why);
            send_error_message(&ctx.http, msg.channel_id, "Error playing song").await?;
            return Ok(());
        }
    };

    let song = handler.enqueue_source(source);
    let metadata = song.metadata();

    let title = metadata
        .title
        .clone()
        .unwrap_or_else(|| "Unknown title".to_string());
    let artist = metadata
        .artist
        .clone()
        .unwrap_or_else(|| "Unknown artist".to_string());
    let thumbnail = metadata.thumbnail.clone().unwrap();

    let queued_message = format!(
        ":notes: **{}** by **{}** added to the queue!",
        title, artist
    );

    msg.channel_id
        .send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.colour(0xffffff)
                    .title(queued_message)
                    .thumbnail(thumbnail)
                    .timestamp(Timestamp::now())
            })
        })
        .await?;

    Ok(())
}

async fn play_playlist(
    ctx: &Context,
    msg: &Message,
    handler: &mut Call,
    playlist_url: &str,
) -> CommandResult {
    let raw_playlist_output = TokioCommand::new("yt-dlp")
        .args(["-j", "--flat-playlist", playlist_url])
        .output()
        .await;

    let raw_playlist = match raw_playlist_output {
        Ok(output) => match String::from_utf8(output.stdout) {
            Ok(s) => s,
            Err(_) => {
                send_error_message(&ctx.http, msg.channel_id, "Failed to parse playlist data")
                    .await?;
                return Ok(());
            }
        },
        Err(_) => {
            send_error_message(&ctx.http, msg.channel_id, "Failed to retrieve playlist").await?;
            return Ok(());
        }
    };

    let playlist_regex =
        Regex::new(r#""url": "(https://www.youtube.com/watch\?v=[A-Za-z0-9]{11})""#).unwrap();
    let track_urls: Vec<String> = playlist_regex
        .captures_iter(&raw_playlist)
        .map(|cap| cap[1].to_string())
        .collect();

    if track_urls.is_empty() {
        send_error_message(&ctx.http, msg.channel_id, "No tracks found in the playlist").await?;
        return Ok(());
    }

    let mut track_errors = 0;

    for track_url in track_urls.iter().cloned() {
        match Restartable::ytdl(track_url, true).await {
            Ok(source) => {
                handler.enqueue_source(source.into());
            }
            Err(why) => {
                error!("Error starting source: {:?}", why);
                track_errors += 1;
            }
        }
    }

    let queued_message = if track_errors == 0 {
        format!(
            ":notes: Playlist queued successfully! {} tracks added.",
            track_urls.len()
        )
    } else {
        format!(
            ":warning: Playlist queued with {} errors. {} tracks added.",
            track_errors,
            track_urls.len() - track_errors
        )
    };

    msg.channel_id
        .send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.colour(0xffffff)
                    .title(queued_message)
                    .timestamp(Timestamp::now())
            })
        })
        .await?;

    Ok(())
}

async fn play_live_stream(
    ctx: &Context,
    msg: &Message,
    handler: &mut Call,
    stream_url: &str,
) -> CommandResult {
    let url = stream_url.to_string();

    match Restartable::ytdl(url, true).await {
        Ok(source) => {
            let song = handler.enqueue_source(source.into());
            let metadata = song.metadata();

            let title = metadata
                .title
                .clone()
                .unwrap_or_else(|| "Unknown title".to_string());
            let artist = metadata
                .artist
                .clone()
                .unwrap_or_else(|| "Unknown artist".to_string());
            let thumbnail = metadata.thumbnail.clone().unwrap();

            let queued_message = format!(
                ":notes: **{}** by **{}** added to the queue!",
                title, artist
            );

            msg.channel_id
                .send_message(&ctx.http, |m| {
                    m.embed(|e| {
                        e.colour(0xffffff)
                            .title(queued_message)
                            .thumbnail(thumbnail)
                            .timestamp(Timestamp::now())
                    })
                })
                .await?;
        }
        Err(why) => {
            error!("Error starting live stream source: {:?}", why);
            send_error_message(
                &ctx.http,
                msg.channel_id,
                "Error adding live stream to Queue",
            )
            .await?;
        }
    }

    Ok(())
}

async fn play_direct_link(
    ctx: &Context,
    msg: &Message,
    handler: &mut Call,
    stream_url: &str,
) -> CommandResult {
    let url = stream_url.to_string();

    match Restartable::ytdl(url, true).await {
        Ok(source) => {
            let song = handler.enqueue_source(source.into());
            let metadata = song.metadata();

            let title = metadata
                .title
                .clone()
                .unwrap_or_else(|| "Unknown".to_string());
            let artist = metadata
                .artist
                .clone()
                .unwrap_or_else(|| "Unknown".to_string());
            let thumbnail = metadata.thumbnail.clone().unwrap();

            let queued_message = format!(
                ":notes: **{}** by **{}** added to the queue!",
                title, artist
            );

            msg.channel_id
                .send_message(&ctx.http, |m| {
                    m.embed(|e| {
                        e.colour(0xffffff)
                            .title(queued_message)
                            .thumbnail(thumbnail)
                            .timestamp(Timestamp::now())
                    })
                })
                .await?;
        }
        Err(why) => {
            error!("Error starting source: {:?}", why);
            send_error_message(&ctx.http, msg.channel_id, "Error adding song to Queue").await?;
        }
    }

    Ok(())
}
