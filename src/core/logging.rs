use std::io;

use flexi_logger::writers::LogWriter;
use flexi_logger::{
    colored_opt_format, Age, Cleanup, Criterion, DeferredNow, Duplicate, Logger, Naming, ReconfigurationHandle,
};
use log::{Level, LevelFilter, Record};
use once_cell::sync::OnceCell;
use twilight::http::Client as HttpClient;
use twilight::model::user::CurrentUser;

use crate::core::BotConfig;
use crate::gearbot_error;
use crate::utils::Emoji;
use crate::Error;
use lazy_static::lazy_static;
use std::sync::RwLock;
use std::time::Duration;

static LOGGER_HANDLE: OnceCell<ReconfigurationHandle> = OnceCell::new();
static BOT_USER: OnceCell<CurrentUser> = OnceCell::new();

const DISCORD_AVATAR_URL: &str = "https://cdn.discordapp.com/avatars/";

lazy_static! {
    pub static ref INFO_QUEUE: RwLock<Vec<String>> = RwLock::new(Vec::new());
}
lazy_static! {
    pub static ref IMPORTANT_QUEUE: RwLock<Vec<String>> = RwLock::new(Vec::new());
}

pub fn initialize() -> Result<(), Error> {
    // TODO: validate webhook by doing a get to it
    // If invalid, `return Err(Error::InvalidLoggingWebhook(url))

    let important = WebhookLogger {
        queue: &IMPORTANT_QUEUE,
    };

    let gearbot_important = Box::new(important);

    let gearbot_info = Box::new(WebhookLogger { queue: &INFO_QUEUE });

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
    BOT_USER.set(user).unwrap();
    run(http.clone(), &IMPORTANT_QUEUE, config.logging.important_logs.to_owned());
    run(http, &INFO_QUEUE, config.logging.info_logs.to_owned());
}

struct WebhookLogger {
    queue: &'static RwLock<Vec<String>>,
}

impl LogWriter for WebhookLogger {
    fn write(&self, now: &mut DeferredNow, record: &Record) -> Result<(), io::Error> {
        let timestamp = now.now().naive_utc().format("%Y-%m-%d %H:%M:%S");
        let log_emote = get_emoji(record.level()).for_chat();
        let log_info = &record.args();

        self.queue
            .write()
            .unwrap()
            .push(format!("``[{}]`` {} {}", timestamp, log_emote, log_info));

        Ok(())
    }

    fn flush(&self) -> Result<(), io::Error> {
        Ok(())
    }

    fn max_log_level(&self) -> LevelFilter {
        LevelFilter::Info
    }
}

pub fn run(http: HttpClient, queue: &'static RwLock<Vec<String>>, url: String) {
    //TODO: when we get too far behind group into a file
    tokio::spawn(async move {
        loop {
            let mut out = {
                let mut todo = queue.write().unwrap();

                let mut out = vec![];
                let count = 0;
                while let Some(s) = todo.first() {
                    if count + s.len() < 2000 {
                        out.push(todo.remove(0));
                    } else {
                        break;
                    }
                }
                out
            };

            if !out.is_empty() {
                let message = out.join("\n");
                out.clear();
                match send_webhook(&http, &url, message.clone()).await {
                    Ok(_) => {}
                    Err(e) => {
                        if e.to_string().contains("Response got 429: Response") {
                            queue.write().unwrap().insert(0, message);
                            tokio::time::delay_for(Duration::new(1, 0)).await;
                        }
                    }
                }
            }
            tokio::time::delay_for(Duration::new(1, 0)).await;
        }
    });
}

async fn send_webhook(http: &HttpClient, url: &str, message: String) -> Result<(), Error> {
    let user = BOT_USER.get().unwrap();
    let executor = http
        .execute_webhook_from_url(url)?
        .content(message)
        .username(&user.name);

    match &user.avatar {
        Some(avatar) => executor.avatar_url(format!("{}{}/{}.png", DISCORD_AVATAR_URL, &user.id, avatar)),
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
