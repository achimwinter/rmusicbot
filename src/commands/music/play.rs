
use regex::Regex;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::prelude::*;
use serenity::{prelude::*, async_trait};

use serenity::Result as SerenityResult;
use songbird::input::{YoutubeDl};
use songbird::{Call, TrackEvent, EventContext};
use tokio::process::Command as TokioCommand;
use songbird::events::{Event, EventHandler as VoiceEventHandler};

use crate::HttpKey;
use crate::commands::utils::{send_error_message, send_success_message};


#[command]
#[aliases(p)]
#[only_in(guilds)]
async fn play(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let url = match get_url_from_args(&args) {
        Some(url) => url,
        None => {
            send_error_message(
                &ctx,
                msg,
                "Use the command like this: play <url> or <song name>",
            )
            .await?;
            return Ok(());
        }
    };

    let guild_id = match get_guild_id(msg, &ctx).await {
        Ok(id) => id,
        Err(_) => {
            send_error_message(&ctx, msg, "Guild not found").await?;
            return Ok(());
        }
    };

    let manager = match songbird::get(&ctx).await {
        Some(manager) => manager,
        None => {
            send_error_message(&ctx, msg, "Songbird client missing").await?;
            return Ok(());
        }
    };

    if manager.get(guild_id).is_none() {
        if let Err(_) = join_channel_if_needed(&ctx, msg).await {
            send_error_message(&ctx, msg, "Error joining the voice channel")
                .await?;
            return Ok(());
        }
    }

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;

        if !url.starts_with("http") {
            search_and_play_single_track(&ctx, msg, &mut handler, &url).await?;
        } else if url.contains("index") {
            play_playlist(&ctx, msg, &mut handler, &url).await?;
        } else if url.contains("live") {
            play_live_stream(&ctx, msg, &mut handler, &url).await?;
        } else {
            play_direct_link(&ctx, msg, &mut handler, &url).await?;
        }
    }

    Ok(())
}

fn get_url_from_args(args: &Args) -> Option<String> {
    let input = args.rest().trim();
    if input.is_empty() {
        None
    } else {
        Some(input.to_string())
    }
}

async fn get_guild_id(msg: &Message, ctx: &Context) -> Result<GuildId, &'static str> {
    msg.guild(&ctx.cache)
        .map(|guild| guild.id)
        .ok_or("Guild not found")
}


async fn join_channel_if_needed(ctx: &Context, msg: &Message) -> CommandResult {
    let (guild_id, channel_id) = {
        let guild = msg.guild(&ctx.cache).unwrap();
        let channel_id = guild
            .voice_states
            .get(&msg.author.id)
            .and_then(|voice_state| voice_state.channel_id);

        (guild.id, channel_id)
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            check_msg(msg.reply(ctx, "Not in a voice channel").await);
            return Ok(());
        }
    };

    if let Ok(handler_lock) = manager.join(guild_id, connect_to).await {
        let mut handler = handler_lock.lock().await;
        handler.add_global_event(TrackEvent::Error.into(), TrackErrorNotifier);
    }

    Ok(())
}

struct TrackErrorNotifier;

#[async_trait]
impl VoiceEventHandler for TrackErrorNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(track_list) = ctx {
            for (state, handle) in *track_list {
                println!(
                    "Track {:?} encountered an error: {:?}",
                    handle.uuid(),
                    state.playing
                );
            }
        }

        None
    }
}

async fn search_and_play_single_track(
    ctx: &Context,
    msg: &Message,
    handler: &mut Call,
    query: &str,
) -> CommandResult {
    let http_client = {
        let data = ctx.data.read().await;
        data.get::<HttpKey>()
        .cloned()
        .expect("Should exist in typemap")
    };

    let source = YoutubeDl::new_search(http_client, query.to_string());
    let _ = handler.enqueue(source.into()).await;

    let _ = send_success_message(ctx, msg, &format!(":mag: Searching and queuing: **{}**", query)).await;

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
                send_error_message(&ctx, msg, "Failed to parse playlist data")
                    .await?;
                return Ok(());
            }
        },
        Err(_) => {
            send_error_message(&ctx, msg, "Failed to retrieve playlist").await?;
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
        send_error_message(&ctx, msg, "No tracks found in the playlist").await?;
        return Ok(());
    }

    let http_client = {
        let data = ctx.data.read().await;
        data.get::<HttpKey>()
        .cloned()
        .expect("Should exist in typemap")
    };

    let track_errors = 0;

    for track_url in track_urls.iter().cloned() {
        let track = YoutubeDl::new(http_client.clone(), track_url);
        handler.enqueue(track.into()).await;
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

    let _ = send_success_message(ctx, msg, &queued_message).await;

    Ok(())
}

async fn play_live_stream(
    ctx: &Context,
    msg: &Message,
    handler: &mut Call,
    stream_url: &str,
) -> CommandResult {
    let url = stream_url.to_string();

    let http_client = {
        let data = ctx.data.read().await;
        data.get::<HttpKey>()
        .cloned()
        .expect("Should exist in typemap")
    };

    let source = YoutubeDl::new(http_client, url);
    let _song = handler.enqueue(source.into()).await;

    let _ = send_success_message(ctx, msg, ":notes: Live stream added to queue!").await;

    Ok(())
}

async fn play_direct_link(
    ctx: &Context,
    msg: &Message,
    handler: &mut Call,
    stream_url: &str,
) -> CommandResult {
    let url = stream_url.to_string();

    let http_client = {
        let data = ctx.data.read().await;
        data.get::<HttpKey>()
        .cloned()
        .expect("Should exist in typemap")
    };

    let source = YoutubeDl::new(http_client, url);
    let _song = handler.enqueue(source.into()).await;

    let _ = send_success_message(ctx, msg, ":notes: Track added to queue!").await;

    Ok(())
}

fn check_msg(result: SerenityResult<Message>) {
    if let Err(why) = result {
        println!("Error sending message {:?}", why)
    }
}
