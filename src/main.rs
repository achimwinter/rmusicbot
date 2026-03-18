mod commands;

use std::collections::HashSet;
use std::env;
use std::sync::Arc;

use serenity::async_trait;
use serenity::framework::standard::macros::{group, hook};
use serenity::framework::standard::{Configuration, StandardFramework};
use serenity::gateway::{ActivityData, ShardManager};
use serenity::http::Http;
use serenity::model::channel::Message;
use serenity::model::event::ResumedEvent;
use serenity::model::gateway::Ready;

use serenity::prelude::*;
use songbird::SerenityInit;
use tracing::{debug, info, instrument};

use crate::commands::help::*;

use crate::commands::music::clear::*;
use crate::commands::music::leave::*;
use crate::commands::music::current::*;
use crate::commands::music::pause::*;
use crate::commands::music::play::*;
use crate::commands::music::resume::*;
use crate::commands::music::skip::*;
use crate::commands::music::stop::*;

use reqwest::Client as HttpClient;

pub struct HttpKey;

impl TypeMapKey for HttpKey {
    type Value = HttpClient;
}

pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(
            "Connected as --> {} [id: {}]",
            ready.user.name, ready.user.id
        );
        let status =
            env::var("DISCORD_STATUS").expect("Set your DISCORD_STATUS environment variable!");
        ctx.set_activity(Some(ActivityData::playing(status)));
    }

    #[instrument(skip(self, _ctx))]
    async fn resume(&self, _ctx: Context, resume: ResumedEvent) {
        debug!("Resumed; trace: {:?}", resume)
    }
}

#[hook]
#[instrument]
async fn before(_: &Context, msg: &Message, command_name: &str) -> bool {
    info!(
        "Received command --> '{}' || User --> '{}'",
        command_name, msg.author.name
    );
    true
}

#[group]
#[commands(help, leave, play, pause, resume, clear, skip, stop, current)]
struct General;

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

    let (owners, _bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            owners.insert(info.owner.unwrap().id);

            (owners, info.id)
        }
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    tracing_subscriber::fmt::init();

    let framework = StandardFramework::new()
        .group(&GENERAL_GROUP);
    framework.configure(Configuration::new().prefix(prefix).owners(owners));

    let intents = GatewayIntents::non_privileged()
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::GUILD_VOICE_STATES;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .framework(framework)
        .register_songbird()
        .type_map_insert::<HttpKey>(HttpClient::new())
        .await
        .expect("Err creating client");


    tokio::spawn(async move {
        let _ = client.start().await.map_err(|why| println!("Client ended {:?}", why));
    });

    let _signal_err = tokio::signal::ctrl_c().await;
    println!("Received Ctrl-C, shutting down");

}
