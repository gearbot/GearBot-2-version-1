use std::io;

use flexi_logger::writers::LogWriter;
use flexi_logger::{
    colored_opt_format, Age, Cleanup, Criterion, DeferredNow, Duplicate, Logger, Naming,
    ReconfigurationHandle,
};
use log::{Level, LevelFilter, Record};
use once_cell::sync::OnceCell;
use tokio;
use twilight::builders::embed::EmbedBuilder;
use twilight::http::Client as HttpClient;
use twilight::model::channel::embed::Embed;
use twilight::model::user::CurrentUser;

use crate::core::BotConfig;
use crate::gearbot_error;
use crate::utils::Emoji;
use crate::Error;

static LOGGER_HANDLE: OnceCell<ReconfigurationHandle> = OnceCell::new();
static BOT_USER: OnceCell<CurrentUser> = OnceCell::new();
static HTTP_CLIENT: OnceCell<HttpClient> = OnceCell::new();
static IMPORTANT_WEBHOOK: OnceCell<String> = OnceCell::new();
static INFO_WEBHOOK: OnceCell<String> = OnceCell::new();

const DISCORD_AVATAR_URL: &str = "https://cdn.discordapp.com/avatars/";
const EMBED_LOG_BLUE: u32 = 0x00_43FF;

const LOGGING_ERROR_EMOTE: &str = "https://cdn.discordapp.com/emojis/528335386238255106.png?v=1";
const LOGGING_WARN_EMOTE: &str = "https://cdn.discordapp.com/emojis/473506219919802388.png?v=1";
const LOGGING_INFO_EMOTE: &str = "https://cdn.discordapp.com/emojis/459697272326848520.png?v=1";
const LOGGING_DEBUG_EMOTE: &str = "https://cdn.discordapp.com/emojis/528335315593723914.png?v=1";

pub fn initialize() -> Result<(), Error> {
    // TODO: validate webhook by doing a get to it
    // If invalid, `return Err(Error::InvalidLoggingWebhook(url))

    let gearbot_important = Box::new(WebhookLogger {
        cell: &IMPORTANT_WEBHOOK,
    });

    let gearbot_info = Box::new(WebhookLogger {
        cell: &INFO_WEBHOOK,
    });

    let log_init_status = LOGGER_HANDLE.set(
        Logger::with_env_or_str("info")
            .duplicate_to_stderr(Duplicate::Debug)
            .log_to_file()
            .directory("logs")
            .format(colored_opt_format)
            .o_timestamp(true)
            .rotate(
                Criterion::Age(Age::Day),
                Naming::Timestamps,
                Cleanup::KeepLogAndZipFiles(10, 30),
            )
            .add_writer("gearbot_important", gearbot_important)
            .add_writer("gearbot_info", gearbot_info)
            .start_with_specfile("logconfig.toml")
            .map_err(|_| Error::NoLoggingSpec)?,
    );

    if log_init_status.is_err() {
        gearbot_error!("The logging system was attempted to be initalized a second time!");
    }

    Ok(())
}

pub fn initialize_discord_webhooks(http: HttpClient, config: &BotConfig, user: CurrentUser) {
    HTTP_CLIENT.set(http).unwrap();
    IMPORTANT_WEBHOOK
        .set(config.logging.important_logs.to_owned())
        .unwrap();
    INFO_WEBHOOK
        .set(config.logging.info_logs.to_owned())
        .unwrap();
    BOT_USER.set(user).unwrap();
}

struct WebhookLogger<'a> {
    cell: &'a OnceCell<String>,
}

impl LogWriter for WebhookLogger<'_> {
    fn write(&self, now: &mut DeferredNow, record: &Record) -> Result<(), io::Error> {
        let mut message = String::from("``[");
        message += &now
            .now()
            .naive_utc()
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        message += "]`` ";
        message += get_emoji(record.level()).for_chat();
        message += " ";
        message += &record.args().to_string();

        let url = self.cell.get().unwrap().to_owned();
        let http = HTTP_CLIENT.get().unwrap().clone();
        tokio::spawn(async move { send_webhook(http, &url, message).await });

        Ok(())
    }

    fn flush(&self) -> Result<(), io::Error> {
        Ok(())
    }

    fn max_log_level(&self) -> LevelFilter {
        LevelFilter::Info
    }
}

async fn send_webhook(http: HttpClient, url: &str, message: String) -> Result<(), Error> {
    let user = BOT_USER.get().unwrap();
    let mut executor = http
        .execute_webhook_from_url(url)?
        .content(message)
        .username(&user.name);

    match &user.avatar {
        Some(avatar) => {
            executor.avatar_url(format!("{}{}/{}.png", DISCORD_AVATAR_URL, &user.id, avatar))
        }
        None => executor,
    }
    .await
    .map_err(Error::TwilightHttp)
    .map(|_| ())
}

fn get_emoji(level: Level) -> Emoji {
    match level {
        Level::Error => Emoji::No,
        Level::Warn => Emoji::Warn,
        Level::Info => Emoji::Info,
        _ => Emoji::Info, // never send to discord so doesn't matter
    }
}

#[macro_use]
pub mod macros {
    #[macro_export]
    macro_rules! gearbot_info {
        ($($arg:tt)*) => (
            log::info!(target: "{gearbot_info,_Default}", $($arg)*);
        )
    }

    #[macro_export]
    macro_rules! gearbot_important {
        ($($arg:tt)*) => (
            log::info!(target: "{gearbot_important,gearbot_info,_Default}", $($arg)*);
        )
    }

    #[macro_export]
    macro_rules! gearbot_error {
        ($($arg:tt)*) => (
            log::error!(target: "{gearbot_important,gearbot_info,_Default}", $($arg)*);
        )
    }

    #[macro_export]
    macro_rules! gearbot_warn {
        ($($arg:tt)*) => (
            log::warn!(target: "{gearbot_important,gearbot_info,_Default}", $($arg)*);
        )
    }
}
