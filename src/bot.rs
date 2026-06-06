use serenity::{
    all::{
        ActivityData, Command, Context, CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage, EventHandler, GatewayIntents, GuildId, Interaction, OnlineStatus, Ready, ScheduledEvent
    },
    async_trait,
};
use tracing::{error, info};

pub enum BotRequest {
    Calendar(
        u64,
        tokio::sync::mpsc::Sender<Result<(String, Vec<ScheduledEvent>), serenity::Error>>,
    ),
}

pub async fn start(mut rx: tokio::sync::mpsc::Receiver<BotRequest>) {
    tracing_subscriber::fmt::init();

    let mut client = serenity::Client::builder(
        std::env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN env var must be set"),
        GatewayIntents::GUILD_SCHEDULED_EVENTS,
    )
    .event_handler(Handler)
    .await
    .expect("should create client");
    let http = client.http.clone();
    tokio::spawn(async move {
        if let Err(why) = client.start().await {
            error!("start error: {:?}", why);
        }
    });
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    loop {
        match rx.recv().await {
            Some(BotRequest::Calendar(guild_id, cmd_tx)) => {
                let guild_id = GuildId::new(guild_id);
                let Ok(guild) = http.get_guild(guild_id).await else {
                    cmd_tx
                        .send(Err(serenity::Error::Other("Guild not found")))
                        .await
                        .expect("should send calendar response");
                    continue;
                };
                let guild_name = guild.name.clone();
                let guild_events = guild_id.scheduled_events(&http, false).await;
                cmd_tx
                    .send(guild_events.map(|events| (guild_name, events)))
                    .await
                    .expect("should send calendar response");
            }
            None => break,
        }
    }
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction
            && command.data.name == "link"
        {
            let content = format!(
                r#"
Here's the link to the public calendar: https://discal.mayson.xyz/calendar/{}

In Google Calendar:
Other Calendars -> + -> From URL

In Outlook:
Add calendar -> Subscribe from web -> Paste the link

In Apple Calendar:
File -> New Calendar Subscription"#,
                command.guild_id.unwrap_or_default()
            );
            let data = CreateInteractionResponseMessage::new()
                .content(content)
                .ephemeral(true);
            let builder = CreateInteractionResponse::Message(data);
            if let Err(why) = command.create_response(&ctx.http, builder).await {
                println!("Cannot respond to slash command: {why}");
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
        ctx.set_presence(Some(ActivityData::custom("use /link")), OnlineStatus::Online);
        if let Err(e) = Command::create_global_command(
            &ctx.http,
            CreateCommand::new("link").description("Get the link to the public calendar"),
        )
        .await
        {
            error!("Failed to create global command: {:?}", e);
        }
    }
}
