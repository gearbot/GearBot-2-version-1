use std::collections::VecDeque;
use std::io;
use std::sync::Arc;

use flexi_logger::writers::LogWriter;
use flexi_logger::{colored_opt_format, Age, Cleanup, Criterion, DeferredNow, Duplicate, Logger, LoggerHandle, Naming};
use log::{Level, LevelFilter, Record};
use once_cell::sync::OnceCell;
use twilight_http::Client as HttpClient;
use twilight_model::user::CurrentUser;

use super::bot_config::{BotConfig, WebhookComponents};
use crate::error::StartupError;
use crate::gearbot_error;
use crate::utils::Emoji;
use std::sync::RwLock;
use std::time::Duration;

static LOGGER_HANDLE: OnceCell<LoggerHandle> = OnceCell::new();
type LogQueue = Arc<RwLock<VecDeque<String>>>;

struct WebhookLogger {
    queue: LogQueue,
}

impl LogWriter for WebhookLogger {
    fn write(&self, now: &mut DeferredNow, record: &Record) -> Result<(), io::Error> {
        let timestamp = now.now().naive_utc().format("%Y-%m-%d %H:%M:%S");
        let log_emote = get_emoji(record.level()).for_chat();
        let log_info = record.args();

        self.queue
            .write()
            .unwrap()
            .push_back(format!("``[{}]`` {} {}", timestamp, log_emote, log_info));

        Ok(())
    }

    fn flush(&self) -> Result<(), io::Error> {
        Ok(())
    }

    fn max_log_level(&self) -> LevelFilter {
        LevelFilter::Info
    }
}

const DISCORD_AVATAR_URL: &str = "https://cdn.discordapp.com/avatars/";

pub fn initialize(http: HttpClient, config: &BotConfig, user: CurrentUser) -> Result<(), StartupError> {
    // TODO: validate webhook by doing a get to it
    // If invalid, `return Err(Error::InvalidLoggingWebhook(url))

    let important_queue = LogQueue::default();
    let info_queue = LogQueue::default();
    let user = Arc::new(user);

    let gearbot_important = Box::new(WebhookLogger {
        queue: important_queue.clone(),
    });

    let gearbot_info = Box::new(WebhookLogger {
        queue: info_queue.clone(),
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
                Cleanup::KeepLogAndCompressedFiles(10, 30),
            )
            .add_writer("gearbot_important", gearbot_important)
            .add_writer("gearbot_info", gearbot_info)
            .start_with_specfile("logconfig.toml")
            .map_err(|_| StartupError::NoLoggingSpec)?,
    );

    if log_init_status.is_err() {
        gearbot_error!("The logging system was attempted to be initalized a second time!");
    }

    run_logging_queue(
        http.clone(),
        important_queue,
        config.logging.important_logs.to_owned(),
        user.clone(),
    );

    run_logging_queue(http, info_queue, config.logging.info_logs.to_owned(), user);

    Ok(())
}

pub fn run_logging_queue(http: HttpClient, queue: LogQueue, url: WebhookComponents, user: Arc<CurrentUser>) {
    //TODO: when we get too far behind group into a file
    tokio::spawn(async move {
        loop {
            let message = {
                let mut todo = queue.write().unwrap();

                let mut out: Vec<String> = Vec::with_capacity(todo.len());
                let mut total_msg_len = 0;

                for s in todo.drain(..) {
                    total_msg_len += s.len();

                    if total_msg_len < 2000 {
                        out.push(s);
                    } else {
                        break;
                    }
                }

                out.join("\n")
            };

            if !message.is_empty() {
                if let Err(e) = send_webhook(&http, &url, &user, &message).await {
                    if e.to_string().contains("Response got 429: Response") {
                        queue.write().unwrap().push_front(message);
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
}

async fn send_webhook(
    http: &HttpClient,
    webhook: &WebhookComponents,
    user: &CurrentUser,
    message: &str,
) -> Result<(), twilight_http::Error> {
    let executor = {
        let raw = http
            .execute_webhook(webhook.id, &webhook.token)
            .content(message)
            .username(&user.name);

        match &user.avatar {
            Some(avatar) => raw.avatar_url(format!("{}{}/{}.png", DISCORD_AVATAR_URL, &user.id, avatar)),
            None => raw,
        }
    };

    if let Err(e) = executor.await {
        log::error!("Log failure: {}", e);
    }

    Ok(())
}

fn get_emoji(level: Level) -> Emoji {
    match level {
        Level::Error => Emoji::No,
        Level::Warn => Emoji::Warn,
        Level::Info => Emoji::Info,
        _ => Emoji::Info, // Never sent to discord so doesn't matter
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
