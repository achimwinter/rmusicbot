use serenity::{
    client::Context,
    framework::standard::{
            macros::command,
            CommandResult,
        },
    model::{channel::Message, Timestamp}, builder::{CreateMessage, CreateEmbed},
};


#[command]
#[only_in(guilds)]
async fn clear(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild(&ctx.cache).unwrap().id;

    let songbird_client = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    match songbird_client.get(guild_id) {
        Some(handler_lock) => {
            let handler = handler_lock.lock().await;
            handler.queue().stop();
            send_clear_message(ctx, msg, 0xffffff, "Queue emptied!").await?;
        }
        None => {
            send_clear_message(ctx, msg, 0xf38ba8, ":warning: Not in voice channel.").await?;
        }
    }
    Ok(())
}

async fn send_clear_message(
    ctx: &Context,
    msg: &Message,
    color: u32,
    title: &str,
) -> CommandResult {
    let embed = CreateEmbed::default()
        .colour(color)
        .title(title)
        .timestamp(Timestamp::now());

    // Create a message and add the embed
    let builder = CreateMessage::default().add_embed(embed);

    // Send the message
    msg.channel_id
        .send_message(&ctx.http, builder)
        .await?;
    Ok(())
}
