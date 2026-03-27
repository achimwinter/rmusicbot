
use regex::Regex;
use std::sync::Arc;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::prelude::*;
use serenity::{prelude::*, async_trait};

use songbird::input::YoutubeDl;
use songbird::{Call, EventContext, Songbird, TrackEvent};
use tokio::process::Command as TokioCommand;
use songbird::events::{Event, EventHandler as VoiceEventHandler};
use tokio::time::{timeout, Duration};
use tracing::{info, warn, debug};

use crate::HttpKey;
use crate::commands::utils::{send_error_message, send_success_message};


#[command]
#[aliases(p)]
#[only_in(guilds)]
async fn play(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    debug!("play: Command invoked by {} with args: {:?}", msg.author.name, args.rest());
    
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

    debug!("play: URL/Query: {}", url);

    let guild_id = match get_guild_id(msg, &ctx).await {
        Ok(id) => {
            debug!("play: Guild ID obtained: {:?}", id);
            id
        }
        Err(_) => {
            send_error_message(&ctx, msg, "Guild not found").await?;
            return Ok(());
        }
    };

    let manager = match songbird::get(&ctx).await {
        Some(manager) => {
            debug!("play: Songbird manager obtained");
            manager
        }
        None => {
            send_error_message(&ctx, msg, "Songbird client missing").await?;
            return Ok(());
        }
    };

    if manager.get(guild_id).is_none() {
        info!("play: Not connected to voice channel yet, attempting to join...");
        if let Err(err_msg) = join_channel_if_needed(&ctx, msg).await {
            warn!("play: Failed to join voice channel: {}", err_msg);
            send_error_message(&ctx, msg, &err_msg).await?;
            return Ok(());
        }
        info!("play: Successfully joined voice channel");
    } else {
        debug!("play: Already connected to voice channel");
    }

    let handler_lock = match manager.get(guild_id) {
        Some(handler_lock) => {
            debug!("play: Handler lock obtained for guild {:?}", guild_id);
            handler_lock
        }
        None => {
            warn!("play: Handler not found after join attempt for guild {:?}", guild_id);
            send_error_message(
                &ctx,
                msg,
                "Failed to connect to the voice channel. Check voice permissions and try again.",
            )
            .await?;
            return Ok(());
        }
    };

    let mut handler = handler_lock.lock().await;
    debug!("play: Handler locked successfully");

    if !url.starts_with("http") {
        info!("play: Searching for track: {}", url);
        search_and_play_single_track(&ctx, msg, &mut handler, &url).await?;
    } else if url.contains("index") {
        info!("play: Playing playlist: {}", url);
        play_playlist(&ctx, msg, &mut handler, &url).await?;
    } else if url.contains("live") {
        info!("play: Playing live stream: {}", url);
        play_live_stream(&ctx, msg, &mut handler, &url).await?;
    } else {
        info!("play: Playing direct link: {}", url);
        play_direct_link(&ctx, msg, &mut handler, &url).await?;
    }

    Ok(())
}

fn get_url_from_args(args: &Args) -> Option<String> {
    let input = args.rest().trim();
    if input.is_empty() {
        debug!("get_url_from_args: No arguments provided");
        None
    } else {
        debug!("get_url_from_args: Input received: {}", input);
        Some(input.to_string())
    }
}

async fn get_guild_id(msg: &Message, ctx: &Context) -> Result<GuildId, &'static str> {
    msg.guild(&ctx.cache)
        .map(|guild| {
            debug!("get_guild_id: Found guild: {}", guild.name);
            guild.id
        })
        .ok_or("Guild not found")
}


async fn join_channel_if_needed(ctx: &Context, msg: &Message) -> Result<(), String> {
    debug!("join_channel_if_needed: Started for user {}", msg.author.name);
    
    let (guild_id, channel_id) = {
        let guild = msg.guild(&ctx.cache).unwrap();
        let channel_id = guild
            .voice_states
            .get(&msg.author.id)
            .and_then(|voice_state| voice_state.channel_id);

        (guild.id, channel_id)
    };

    debug!("join_channel_if_needed: Guild ID: {:?}, Channel ID: {:?}", guild_id, channel_id);

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let connect_to = match channel_id {
        Some(channel) => {
            debug!("join_channel_if_needed: User is in channel {:?}", channel);
            channel
        }
        None => {
            warn!("join_channel_if_needed: User {} is not in any voice channel", msg.author.name);
            return Err("You must join a voice channel first.".to_string());
        }
    };

    // Retry logic with exponential backoff
    // Discord voice gateway can have transient issues that resolve quickly
    for attempt in 1..=3 {
        debug!("join_channel_if_needed: Attempt {} of 3 to join guild {:?} channel {:?}", 
               attempt, guild_id, connect_to);
        
        match timeout(Duration::from_secs(20), manager.join(guild_id, connect_to)).await {
            Ok(Ok(handler_lock)) => {
                info!("join_channel_if_needed: Successfully joined voice channel on attempt {}", attempt);
                let mut handler = handler_lock.lock().await;
                handler.add_global_event(TrackEvent::Error.into(), TrackErrorNotifier);
                handler.add_global_event(
                    TrackEvent::End.into(),
                    QueueEndNotifier {
                        manager: manager.clone(),
                        guild_id,
                    },
                );
                debug!("join_channel_if_needed: Added error notifier");
                return Ok(());
            }
            Ok(Err(err)) => {
                warn!("join_channel_if_needed: Attempt {} failed with error: {}", attempt, err);
                if attempt == 3 {
                    let err_msg = format!("Failed to join voice channel after 3 attempts. Discord may be experiencing issues. Error: {}", err);
                    warn!("join_channel_if_needed: {}", err_msg);
                    return Err(err_msg);
                }
                let wait_ms = 2000 * attempt as u64;
                debug!("join_channel_if_needed: Waiting {}ms before retry", wait_ms);
                tokio::time::sleep(Duration::from_millis(wait_ms)).await;
            }
            Err(_) => {
                warn!("join_channel_if_needed: Attempt {} timed out after 20 seconds", attempt);
                if attempt == 3 {
                    let err_msg = "Joining voice channel timed out after 3 attempts (Discord gateway unresponsive). Please try again. If this persists, check Discord's status page.";
                    warn!("join_channel_if_needed: {}", err_msg);
                    return Err(err_msg.to_string());
                }
                let wait_ms = 2000 * attempt as u64;
                debug!("join_channel_if_needed: Waiting {}ms before retry after timeout", wait_ms);
                tokio::time::sleep(Duration::from_millis(wait_ms)).await;
            }
        }
    }

    let err_msg = "Failed to join voice channel after 3 attempts".to_string();
    warn!("join_channel_if_needed: {}", err_msg);
    Err(err_msg)
}

struct TrackErrorNotifier;

struct QueueEndNotifier {
    manager: Arc<Songbird>,
    guild_id: GuildId,
}

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

#[async_trait]
impl VoiceEventHandler for QueueEndNotifier {
    async fn act(&self, _: &EventContext<'_>) -> Option<Event> {
        if let Some(handler_lock) = self.manager.get(self.guild_id) {
            let should_leave = {
                let handler = handler_lock.lock().await;
                handler.queue().current().is_none()
            };

            if should_leave {
                match self.manager.remove(self.guild_id).await {
                    Ok(_) => info!("Queue empty in guild {:?}, left voice channel", self.guild_id),
                    Err(err) => warn!(
                        "Failed to leave voice channel in guild {:?} after queue finished: {}",
                        self.guild_id,
                        err
                    ),
                }
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
    debug!("search_and_play_single_track: Searching for '{}'", query);
    
    let http_client = {
        let data = ctx.data.read().await;
        data.get::<HttpKey>()
        .cloned()
        .expect("Should exist in typemap")
    };

    let source = YoutubeDl::new_search(http_client, query.to_string());
    match handler.enqueue(source.into()).await {
        _ => {
            info!("search_and_play_single_track: Enqueued search result for '{}'", query);
        }
    }

    let _ = send_success_message(ctx, msg, &format!(":mag: Searching and queuing: **{}**", query)).await;

    Ok(())
}

async fn play_playlist(
    ctx: &Context,
    msg: &Message,
    handler: &mut Call,
    playlist_url: &str,
) -> CommandResult {
    info!("play_playlist: Processing playlist: {}", playlist_url);
    debug!("play_playlist: Running yt-dlp command");
    
    let raw_playlist_output = TokioCommand::new("yt-dlp")
        .args(["-j", "--flat-playlist", playlist_url])
        .output()
        .await;

    let raw_playlist = match raw_playlist_output {
        Ok(output) => match String::from_utf8(output.stdout) {
            Ok(s) => {
                debug!("play_playlist: yt-dlp output received, length: {} bytes", s.len());
                s
            }
            Err(_) => {
                warn!("play_playlist: Failed to parse yt-dlp output as UTF-8");
                send_error_message(&ctx, msg, "Failed to parse playlist data")
                    .await?;
                return Ok(());
            }
        },
        Err(e) => {
            warn!("play_playlist: yt-dlp command failed: {}", e);
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
        warn!("play_playlist: No tracks found in playlist: {}", playlist_url);
        send_error_message(&ctx, msg, "No tracks found in the playlist").await?;
        return Ok(());
    }

    info!("play_playlist: Found {} tracks in playlist", track_urls.len());

    let http_client = {
        let data = ctx.data.read().await;
        data.get::<HttpKey>()
        .cloned()
        .expect("Should exist in typemap")
    };

    let mut track_errors = 0;

    for (idx, track_url) in track_urls.iter().cloned().enumerate() {
        let track = YoutubeDl::new(http_client.clone(), track_url.clone());
        match handler.enqueue(track.into()).await {
            _ => {
                debug!("play_playlist: Enqueued track {}/{}", idx + 1, track_urls.len());
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

    info!("play_playlist: Playlist queued - {} tracks, {} errors", track_urls.len(), track_errors);
    let _ = send_success_message(ctx, msg, &queued_message).await;

    Ok(())
}

async fn play_live_stream(
    ctx: &Context,
    msg: &Message,
    handler: &mut Call,
    stream_url: &str,
) -> CommandResult {
    debug!("play_live_stream: Processing stream: {}", stream_url);
    
    let url = stream_url.to_string();

    let http_client = {
        let data = ctx.data.read().await;
        data.get::<HttpKey>()
        .cloned()
        .expect("Should exist in typemap")
    };

    let source = YoutubeDl::new(http_client, url);
    match handler.enqueue(source.into()).await {
        _ => {
            info!("play_live_stream: Enqueued live stream");
        }
    }

    let _ = send_success_message(ctx, msg, ":notes: Live stream added to queue!").await;

    Ok(())
}

async fn play_direct_link(
    ctx: &Context,
    msg: &Message,
    handler: &mut Call,
    stream_url: &str,
) -> CommandResult {
    debug!("play_direct_link: Processing direct link: {}", stream_url);
    
    let url = stream_url.to_string();

    let http_client = {
        let data = ctx.data.read().await;
        data.get::<HttpKey>()
        .cloned()
        .expect("Should exist in typemap")
    };

    let source = YoutubeDl::new(http_client, url);
    match handler.enqueue(source.into()).await {
        _ => {
            info!("play_direct_link: Enqueued track from direct link");
        }
    }

    let _ = send_success_message(ctx, msg, ":notes: Track added to queue!").await;

    Ok(())
}

