use serenity::builder::{CreateEmbed, CreateMessage};
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::prelude::*;
use serenity::model::Timestamp;
use serenity::prelude::*;

use std::env;

// Custom help menu

/// Pure lookup used to build the help embed's fields for a given menu choice.
/// Kept separate from the command handler so it can be unit tested without
/// needing a live Discord context.
fn help_fields(menu_choice: &str) -> Vec<(&'static str, &'static str, bool)> {
    match menu_choice {
        "general" => {
            vec![("help", "Displays this help menu", true)]
        }

        "music" => {
            vec![
                ("leave", "Leaves a music channel", true),
                ("play", "Play / queue a song from a YouTube URL", true),
                ("stop", "Stops current playlist", true),
                ("skip", "Skips the current song", true),
                ("pause", "Pauses the current song", true),
                ("resume", "Resumes the current song", true),
                ("nowplaying", "Shows info about current song", true),
                ("clear", "Clear the queue", true),
            ]
        }

        _ => {
            vec![
                ("help", "Displays this help menu", false),
                ("help music", "Show music commands", false),
                ("help general", "Show general commands", false),
            ]
        }
    }
}

/// Parses the requested menu choice from the command args, defaulting to
/// "default" when no argument was supplied.
fn parse_menu_choice(args: &mut Args) -> String {
    match args.single::<String>() {
        Ok(menu_choice) => menu_choice,
        Err(_) => "default".to_string(),
    }
}

#[command]
pub async fn help(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let prefix = env::var("PREFIX").expect("Set your PREFIX environment variable!");

    let menu_choice_str = parse_menu_choice(&mut args);
    let menu_choice: &str = &menu_choice_str;

    msg.channel_id
        .send_message(&ctx.http, {
            CreateMessage::default().add_embed(
                CreateEmbed::default()
                    .colour(0xffffff)
                    .title("**-- Help Menu --**")
                    .description(format!("Hi i'm RMusicBot. My prefix is `{}`", prefix))
                    .fields(help_fields(menu_choice))
                    .timestamp(Timestamp::now()),
            )
        })
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_fields_general_lists_help_command() {
        let fields = help_fields("general");
        assert_eq!(fields, vec![("help", "Displays this help menu", true)]);
    }

    #[test]
    fn help_fields_music_lists_all_music_commands() {
        let fields = help_fields("music");
        assert_eq!(fields.len(), 8);
        assert!(fields.contains(&("play", "Play / queue a song from a YouTube URL", true)));
        assert!(fields.contains(&("clear", "Clear the queue", true)));
    }

    #[test]
    fn help_fields_defaults_for_unknown_choice() {
        let fields = help_fields("default");
        assert_eq!(fields, help_fields("anything-else"));
        assert_eq!(fields.len(), 3);
    }

    #[test]
    fn parse_menu_choice_defaults_when_no_args() {
        let mut args = Args::new("", &[]);
        assert_eq!(parse_menu_choice(&mut args), "default");
    }

    #[test]
    fn parse_menu_choice_reads_first_argument() {
        let mut args = Args::new("music", &[]);
        assert_eq!(parse_menu_choice(&mut args), "music");
    }
}
