use std::io;

use flexi_logger::{Age, Cleanup, Criterion, DeferredNow, Duplicate, Logger, Naming, opt_format, ReconfigurationHandle};
use flexi_logger::writers::LogWriter;
use log::{Level, LevelFilter, Record};
use once_cell::sync::OnceCell;
use tokio;
use twilight::builders::embed::EmbedBuilder;
use twilight::http::Client as HttpClient;
use twilight::model::channel::embed::{Embed, EmbedFooter};

use crate::core::BotConfig;
use crate::Error;
use crate::gearbot_error;

static LOGGER_HANDLE: OnceCell<ReconfigurationHandle> = OnceCell::new();

pub fn initialize(http: HttpClient, config: &BotConfig) -> Result<(), Error> {
    // TODO: validate webhook by doing a get to it
    // If invalid, `return Err(Error::InvalidLoggingWebhook(url))

    let gearbot_important = Box::new(WebhookLogger { http: http.clone(), url: config.logging.important_logs.to_owned() });

    let gearbot_info = Box::new(WebhookLogger { http, url: config.logging.info_logs.to_owned() });

    let log_init_status = LOGGER_HANDLE.set(
        Logger::with_str("info")
            .duplicate_to_stderr(Duplicate::Info)
            .log_to_file()
            .directory("logs")
            .format(opt_format)
            .o_timestamp(true)
            .rotate(Criterion::Age(Age::Day), Naming::Timestamps, Cleanup::KeepLogAndZipFiles(10, 30))
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

struct WebhookLogger {
    http: HttpClient,
    url: String,
}

impl LogWriter for WebhookLogger {
    fn write(&self, now: &mut DeferredNow, record: &Record) -> Result<(), io::Error> {
        let embed_builder = EmbedBuilder::new().color(0x0043FF).description(record.args().to_string()).timestamp(now.now().naive_utc().to_string()).footer(record.level().to_string()).icon_url(get_icon(record.level())).commit();

        let url = self.url.to_owned();
        let http = self.http.clone();
        let embeds = vec![embed_builder.build()];
        tokio::spawn(async move { send_webhook(http, &url, embeds).await });

        Ok(())
    }

    fn flush(&self) -> Result<(), io::Error> {
        Ok(())
    }

    fn max_log_level(&self) -> LevelFilter {
        LevelFilter::Info
    }
}

async fn send_webhook(http: HttpClient, url: &str, embeds: Vec<Embed>) -> Result<(), Error> {
    http.execute_webhook_from_url(url)?.embeds(embeds).await.map_err(Error::TwilightError).map(|_| ())
}

fn get_icon(level: Level) -> String {
    match level {
        Level::Info => String::from("https://cdn.discordapp.com/emojis/459697272326848520.png?v=1"),
        Level::Warn => String::from("https://cdn.discordapp.com/emojis/473506219919802388.png?v=1"),
        Level::Error => String::from("https://cdn.discordapp.com/emojis/528335386238255106.png?v=1"),
        Level::Debug => String::from("https://cdn.discordapp.com/emojis/528335315593723914.png?v=1"),
        Level::Trace => String::from("https://cdn.discordapp.com/emojis/528335315593723914.png?v=1"),
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
}
