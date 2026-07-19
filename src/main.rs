mod commands;

use std::collections::HashSet;
use std::env;

use poise::serenity_prelude as serenity;
use serenity::ActivityData;
use serenity::GatewayIntents;
use serenity::Http;
use songbird::SerenityInit;
use tracing::{debug, info};

use crate::commands::help::help;

use crate::commands::music::clear::clear;
use crate::commands::music::current::current;
use crate::commands::music::leave::leave;
use crate::commands::music::pause::pause;
use crate::commands::music::play::play;
use crate::commands::music::resume::resume;
use crate::commands::music::skip::skip;
use crate::commands::music::stop::stop;

use reqwest::Client as HttpClient;

pub struct Data {
    pub http: HttpClient,
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;
pub type CommandResult = Result<(), Error>;

#[cfg(feature = "development")]
fn init_env() {
    dotenv::dotenv().ok();
    println!("Running in development mode. Dotenv loaded.");
}

#[cfg(not(feature = "development"))]
fn init_env() {
    println!("Running in production mode. Using system environment variables.");
}

#[tokio::main]
async fn main() {
    init_env();

    let token = env::var("DISCORD_TOKEN").expect("Set your DISCORD_TOKEN environment variable!");
    let prefix = env::var("PREFIX").expect("Set your PREFIX environment variable!");

    let http = Http::new(&token);

    let owners = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            owners.insert(info.owner.unwrap().id);
            owners
        }
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    tracing_subscriber::fmt::init();

    let intents = GatewayIntents::non_privileged()
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::GUILD_VOICE_STATES;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                help(),
                leave(),
                play(),
                pause(),
                resume(),
                clear(),
                skip(),
                stop(),
                current(),
            ],
            owners,
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some(prefix),
                ..Default::default()
            },
            pre_command: |ctx| {
                Box::pin(async move {
                    info!(
                        "Received command --> '{}' || User --> '{}'",
                        ctx.command().name,
                        ctx.author().name
                    );
                })
            },
            ..Default::default()
        })
        .setup(move |ctx, ready, _framework| {
            Box::pin(async move {
                info!(
                    "Connected as --> {} [id: {}]",
                    ready.user.name, ready.user.id
                );
                let status = env::var("DISCORD_STATUS")
                    .expect("Set your DISCORD_STATUS environment variable!");
                ctx.set_activity(Some(ActivityData::playing(status)));
                debug!("Bot ready");
                Ok(Data {
                    http: HttpClient::new(),
                })
            })
        })
        .build();

    let mut client = serenity::ClientBuilder::new(&token, intents)
        .framework(framework)
        .register_songbird()
        .await
        .expect("Err creating client");

    tokio::spawn(async move {
        let _ = client
            .start()
            .await
            .map_err(|why| println!("Client ended {:?}", why));
    });

    let _signal_err = tokio::signal::ctrl_c().await;
    println!("Received Ctrl-C, shutting down");
}
