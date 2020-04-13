use std::borrow::Borrow;
use std::io::{Error, Write};
use std::sync::Arc;

use flexi_logger::{Age, Cleanup, Criterion, DeferredNow, FlexiLoggerError, Logger, LogSpecification, Naming, opt_format, ReconfigurationHandle};
use flexi_logger::writers::LogWriter;
use log::{Level, LevelFilter, Record};
use log::{error, info};
use once_cell::sync::OnceCell;
use tokio;
use twilight::http::Client as HttpClient;
use twilight::model::channel::embed::{Embed, EmbedFooter};

use crate::core::BotConfig;

static LOGGER_HANDLE: OnceCell<ReconfigurationHandle> = OnceCell::new();

pub fn initialize(http: HttpClient, config: &BotConfig) {
    // TODO: validate webhook by doing a get to it

    LOGGER_HANDLE.set(Logger::with_str("info")
        .log_to_file()
        .directory("logs")
        .format(opt_format)
        .o_timestamp(true)
        .rotate(Criterion::Age(Age::Day), Naming::Timestamps, Cleanup::KeepLogFiles(30))
        .add_writer("gearbot_important", Box::new(WebhookLogger { http: http.clone(), url: String::from(&config.logging.important_logs) }))
        .add_writer("gearbot_info", Box::new(WebhookLogger { http, url: String::from(&config.logging.info_logs) }))
        .start_with_specfile("logconfig.toml")
        .unwrap())
    ;
}

struct WebhookLogger {
    http: HttpClient,
    url: String,
}

impl LogWriter for WebhookLogger {
    fn write(&self, _now: &mut DeferredNow, record: &Record<'_>) -> Result<(), Error> {
        let e = Embed {
            author: None,
            color: Some(0x0043FF),
            description: Some(record.args().to_string()),
            fields: vec![],
            footer: Some(EmbedFooter {text: record.level().to_string(), icon_url: Some(get_icon(record.level())), proxy_icon_url: None}),
            image: None,
            kind: String::from("rich"),
            provider: None,
            thumbnail: None,
            timestamp: Some(_now.now().naive_utc().to_string()),
            title: None,
            url: None,
            video: None,
        };

        let url = self.url.to_owned();
        let http = self.http.clone();
        //TODO: get rid of the wraps
        tokio::spawn(async move {
            http
                .execute_webhook_from_url(url)
                .unwrap()
                .embeds(vec![e])
                .await
            ;
        });
        Ok(())
    }

    fn flush(&self) -> Result<(), Error> { Ok(()) }

    fn max_log_level(&self) -> LevelFilter {
        LevelFilter::Info
    }
}

fn get_icon(level: Level) -> String {
    match level {
        Level::Info => String::from("https://cdn.discordapp.com/emojis/459697272326848520.png?v=1"),
        Level::Warn => String::from("https://cdn.discordapp.com/emojis/473506219919802388.png?v=1"),
        Level::Error => String::from("https://cdn.discordapp.com/emojis/528335386238255106.png?v=1"),
        Level::Debug => {String::from("https://cdn.discordapp.com/emojis/528335315593723914.png?v=1")}
        Level::Trace => {String::from("https://cdn.discordapp.com/emojis/528335315593723914.png?v=1")}
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
}