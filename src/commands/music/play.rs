use poise::serenity_prelude::{self as serenity, GuildId};
use regex::Regex;
use std::sync::Arc;

use songbird::events::{Event, EventHandler as VoiceEventHandler};
use songbird::input::YoutubeDl;
use songbird::{Call, EventContext, Songbird, TrackEvent};
use tokio::process::Command as TokioCommand;
use tokio::time::{timeout, Duration};
use tracing::{debug, info, warn};

use crate::commands::utils::{get_guild_id, send_error_message, send_success_message};
use crate::{CommandResult, Context};

#[poise::command(prefix_command, aliases("p"), guild_only)]
pub async fn play(ctx: Context<'_>, #[rest] query: Option<String>) -> CommandResult {
    debug!(
        "play: Command invoked by {} with args: {:?}",
        ctx.author().name,
        query
    );

    let url = match normalize_query(query) {
        Some(url) => url,
        None => {
            send_error_message(ctx, "Use the command like this: play <url> or <song name>").await?;
            return Ok(());
        }
    };

    debug!("play: URL/Query: {}", url);

    let guild_id = match get_guild_id(ctx) {
        Ok(id) => {
            debug!("play: Guild ID obtained: {:?}", id);
            id
        }
        Err(_) => {
            send_error_message(ctx, "Guild not found").await?;
            return Ok(());
        }
    };

    let manager = match songbird::get(ctx.serenity_context()).await {
        Some(manager) => {
            debug!("play: Songbird manager obtained");
            manager
        }
        None => {
            send_error_message(ctx, "Songbird client missing").await?;
            return Ok(());
        }
    };

    if manager.get(guild_id).is_none() {
        info!("play: Not connected to voice channel yet, attempting to join...");
        if let Err(err_msg) = join_channel_if_needed(ctx).await {
            warn!("play: Failed to join voice channel: {}", err_msg);
            send_error_message(ctx, &err_msg).await?;
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
            warn!(
                "play: Handler not found after join attempt for guild {:?}",
                guild_id
            );
            send_error_message(
                ctx,
                "Failed to connect to the voice channel. Check voice permissions and try again.",
            )
            .await?;
            return Ok(());
        }
    };

    let mut handler = handler_lock.lock().await;
    debug!("play: Handler locked successfully");

    match classify_url(&url) {
        UrlKind::Search => {
            info!("play: Searching for track: {}", url);
            search_and_play_single_track(ctx, &mut handler, &url).await?;
        }
        UrlKind::Playlist => {
            info!("play: Playing playlist: {}", url);
            play_playlist(ctx, &mut handler, &url).await?;
        }
        UrlKind::Live => {
            info!("play: Playing live stream: {}", url);
            play_live_stream(ctx, &mut handler, &url).await?;
        }
        UrlKind::Direct => {
            info!("play: Playing direct link: {}", url);
            play_direct_link(ctx, &mut handler, &url).await?;
        }
    }

    Ok(())
}

/// Trims the supplied query and returns `None` when it is absent or empty.
fn normalize_query(query: Option<String>) -> Option<String> {
    let input = query?;
    let trimmed = input.trim();
    if trimmed.is_empty() {
        debug!("normalize_query: No arguments provided");
        None
    } else {
        debug!("normalize_query: Input received: {}", trimmed);
        Some(trimmed.to_string())
    }
}

#[derive(Debug, PartialEq, Eq)]
enum UrlKind {
    Search,
    Playlist,
    Live,
    Direct,
}

fn classify_url(url: &str) -> UrlKind {
    if !url.starts_with("http") {
        UrlKind::Search
    } else if url.contains("index") {
        UrlKind::Playlist
    } else if url.contains("live") {
        UrlKind::Live
    } else {
        UrlKind::Direct
    }
}

async fn join_channel_if_needed(ctx: Context<'_>) -> Result<(), String> {
    let author_name = ctx.author().name.clone();
    debug!("join_channel_if_needed: Started for user {}", author_name);

    let guild_id = ctx.guild_id().ok_or("Guild not found".to_string())?;
    let author_id = ctx.author().id;

    let channel_id = ctx
        .serenity_context()
        .cache
        .guild(guild_id)
        .and_then(|guild| {
            guild
                .voice_states
                .get(&author_id)
                .and_then(|vs| vs.channel_id)
        });

    debug!(
        "join_channel_if_needed: Guild ID: {:?}, Channel ID: {:?}",
        guild_id, channel_id
    );

    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let connect_to = match channel_id {
        Some(channel) => {
            debug!("join_channel_if_needed: User is in channel {:?}", channel);
            channel
        }
        None => {
            warn!(
                "join_channel_if_needed: User {} is not in any voice channel",
                author_name
            );
            return Err("You must join a voice channel first.".to_string());
        }
    };

    // Retry logic with exponential backoff
    // Discord voice gateway can have transient issues that resolve quickly
    for attempt in 1..=3 {
        debug!(
            "join_channel_if_needed: Attempt {} of 3 to join guild {:?} channel {:?}",
            attempt, guild_id, connect_to
        );

        match timeout(Duration::from_secs(20), manager.join(guild_id, connect_to)).await {
            Ok(Ok(handler_lock)) => {
                info!(
                    "join_channel_if_needed: Successfully joined voice channel on attempt {}",
                    attempt
                );
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
                warn!(
                    "join_channel_if_needed: Attempt {} failed with error: {}",
                    attempt, err
                );
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
                warn!(
                    "join_channel_if_needed: Attempt {} timed out after 20 seconds",
                    attempt
                );
                if attempt == 3 {
                    let err_msg = "Joining voice channel timed out after 3 attempts (Discord gateway unresponsive). Please try again. If this persists, check Discord's status page.";
                    warn!("join_channel_if_needed: {}", err_msg);
                    return Err(err_msg.to_string());
                }
                let wait_ms = 2000 * attempt as u64;
                debug!(
                    "join_channel_if_needed: Waiting {}ms before retry after timeout",
                    wait_ms
                );
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

#[serenity::async_trait]
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

#[serenity::async_trait]
impl VoiceEventHandler for QueueEndNotifier {
    async fn act(&self, _: &EventContext<'_>) -> Option<Event> {
        if let Some(handler_lock) = self.manager.get(self.guild_id) {
            let should_leave = {
                let handler = handler_lock.lock().await;
                handler.queue().current().is_none()
            };

            if should_leave {
                match self.manager.remove(self.guild_id).await {
                    Ok(_) => info!(
                        "Queue empty in guild {:?}, left voice channel",
                        self.guild_id
                    ),
                    Err(err) => warn!(
                        "Failed to leave voice channel in guild {:?} after queue finished: {}",
                        self.guild_id, err
                    ),
                }
            }
        }

        None
    }
}

async fn search_and_play_single_track(
    ctx: Context<'_>,
    handler: &mut Call,
    query: &str,
) -> CommandResult {
    debug!("search_and_play_single_track: Searching for '{}'", query);

    let http_client = ctx.data().http.clone();

    let source = YoutubeDl::new_search(http_client, query.to_string());
    handler.enqueue(source.into()).await;
    info!(
        "search_and_play_single_track: Enqueued search result for '{}'",
        query
    );

    let _ = send_success_message(ctx, &format!(":mag: Searching and queuing: **{}**", query)).await;

    Ok(())
}

async fn play_playlist(ctx: Context<'_>, handler: &mut Call, playlist_url: &str) -> CommandResult {
    info!("play_playlist: Processing playlist: {}", playlist_url);
    debug!("play_playlist: Running yt-dlp command");

    let raw_playlist_output = TokioCommand::new("yt-dlp")
        .args(["-j", "--flat-playlist", playlist_url])
        .output()
        .await;

    let raw_playlist = match raw_playlist_output {
        Ok(output) => match String::from_utf8(output.stdout) {
            Ok(s) => {
                debug!(
                    "play_playlist: yt-dlp output received, length: {} bytes",
                    s.len()
                );
                s
            }
            Err(_) => {
                warn!("play_playlist: Failed to parse yt-dlp output as UTF-8");
                send_error_message(ctx, "Failed to parse playlist data").await?;
                return Ok(());
            }
        },
        Err(e) => {
            warn!("play_playlist: yt-dlp command failed: {}", e);
            send_error_message(ctx, "Failed to retrieve playlist").await?;
            return Ok(());
        }
    };

    let track_urls = parse_playlist_track_urls(&raw_playlist);

    if track_urls.is_empty() {
        warn!(
            "play_playlist: No tracks found in playlist: {}",
            playlist_url
        );
        send_error_message(ctx, "No tracks found in the playlist").await?;
        return Ok(());
    }

    info!(
        "play_playlist: Found {} tracks in playlist",
        track_urls.len()
    );

    let http_client = ctx.data().http.clone();

    let track_errors = 0;

    for (idx, track_url) in track_urls.iter().cloned().enumerate() {
        let track = YoutubeDl::new(http_client.clone(), track_url.clone());
        handler.enqueue(track.into()).await;
        debug!(
            "play_playlist: Enqueued track {}/{}",
            idx + 1,
            track_urls.len()
        );
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

    info!(
        "play_playlist: Playlist queued - {} tracks, {} errors",
        track_urls.len(),
        track_errors
    );
    let _ = send_success_message(ctx, &queued_message).await;

    Ok(())
}

async fn play_live_stream(ctx: Context<'_>, handler: &mut Call, stream_url: &str) -> CommandResult {
    debug!("play_live_stream: Processing stream: {}", stream_url);

    let url = stream_url.to_string();

    let http_client = ctx.data().http.clone();

    let source = YoutubeDl::new(http_client, url);
    handler.enqueue(source.into()).await;
    info!("play_live_stream: Enqueued live stream");

    let _ = send_success_message(ctx, ":notes: Live stream added to queue!").await;

    Ok(())
}

async fn play_direct_link(ctx: Context<'_>, handler: &mut Call, stream_url: &str) -> CommandResult {
    debug!("play_direct_link: Processing direct link: {}", stream_url);

    let url = stream_url.to_string();

    let http_client = ctx.data().http.clone();

    let source = YoutubeDl::new(http_client, url);
    handler.enqueue(source.into()).await;
    info!("play_direct_link: Enqueued track from direct link");

    let _ = send_success_message(ctx, ":notes: Track added to queue!").await;

    Ok(())
}

fn parse_playlist_track_urls(raw_playlist: &str) -> Vec<String> {
    let playlist_regex =
        Regex::new(r#""url": "(https://www.youtube.com/watch\?v=[A-Za-z0-9]{11})""#).unwrap();
    playlist_regex
        .captures_iter(raw_playlist)
        .map(|cap| cap[1].to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_url_detects_search_query() {
        assert_eq!(classify_url("never gonna give you up"), UrlKind::Search);
    }

    #[test]
    fn classify_url_detects_playlist() {
        assert_eq!(
            classify_url("https://www.youtube.com/playlist?list=abc&index=1"),
            UrlKind::Playlist
        );
    }

    #[test]
    fn classify_url_detects_live_stream() {
        assert_eq!(
            classify_url("https://www.youtube.com/watch?v=live12345678"),
            UrlKind::Live
        );
    }

    #[test]
    fn classify_url_detects_direct_link() {
        assert_eq!(
            classify_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ"),
            UrlKind::Direct
        );
    }

    #[test]
    fn parse_playlist_track_urls_extracts_all_video_urls() {
        let raw = r#"{"url": "https://www.youtube.com/watch?v=aaaaaaaaaaa"}
{"url": "https://www.youtube.com/watch?v=bbbbbbbbbbb"}"#;

        let urls = parse_playlist_track_urls(raw);

        assert_eq!(
            urls,
            vec![
                "https://www.youtube.com/watch?v=aaaaaaaaaaa".to_string(),
                "https://www.youtube.com/watch?v=bbbbbbbbbbb".to_string(),
            ]
        );
    }

    #[test]
    fn parse_playlist_track_urls_returns_empty_for_no_matches() {
        assert!(parse_playlist_track_urls("no urls here").is_empty());
    }

    #[test]
    fn normalize_query_returns_none_for_missing_input() {
        assert_eq!(normalize_query(None), None);
    }

    #[test]
    fn normalize_query_returns_none_for_blank_input() {
        assert_eq!(normalize_query(Some("   ".to_string())), None);
    }

    #[test]
    fn normalize_query_returns_trimmed_input() {
        assert_eq!(
            normalize_query(Some("  some song name  ".to_string())),
            Some("some song name".to_string())
        );
    }
}
