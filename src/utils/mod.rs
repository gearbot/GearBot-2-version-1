use chrono::{DateTime, NaiveDateTime, Utc};
pub use emoji::*;
pub use errors::*;

// TODO: Remove this when they are all used.
#[allow(dead_code)]
pub mod matchers;

pub mod emoji;
mod errors;

pub use emoji::*;
pub use errors::*;

mod log_types;
pub use log_types::*;

const MARKDOWN_REPALCEMENTS: &[&str; 7] = &["\\", "*", "_", "~", "|", "{", ">"];
const DISCORD_EPOCH: i64 = 1_420_070_400_000;

fn replace_markdown(msg: &mut String) {
    for c in MARKDOWN_REPALCEMENTS.iter() {
        if let Some(pos) = msg.find(c) {
            msg.insert_str(pos, "\\")
        }
    }
}

fn replace_urls(before: String, msg: &mut String) {
    let urls = matchers::get_urls(&before);
    for url in urls.iter().rev() {
        msg.insert_str(url.start(), "<");
        msg.insert_str(url.end() + 1, ">");
    }
}

fn replace_emotes(before: String, msg: &mut String) {
    for em in matchers::get_emotes(&before).iter().rev() {
        println!("{:?}", em);
        msg.insert_str(em.start(), "\\");
        msg.insert_str(em.end() + 1, "\\");
    }
}

pub fn replace_lookalikes(msg: &mut String) -> String {
    msg.replace('`', "ˋ")
}

pub fn clean(msg: &str, markdown: bool, links: bool, emotes: bool, lookalikes: bool) -> String {
    let mut msg = msg.to_owned();

    if lookalikes {
        msg = replace_lookalikes(&mut msg);
    }

    if markdown {
        msg = replace_lookalikes(&mut msg);
        replace_markdown(&mut msg);
    }

    if links {
        replace_markdown(&mut msg);
        replace_urls(msg.clone(), &mut msg);
    }

    if emotes {
        replace_emotes(msg.clone(), &mut msg);
    }

    println!("{:?}", msg);

    msg
}

pub fn snowflake_timestamp(snowflake: u64) -> DateTime<Utc> {
    DateTime::from_utc(
        NaiveDateTime::from_timestamp(((snowflake as i64 >> 22) + DISCORD_EPOCH) / 1000, 0),
        Utc,
    )
}

pub fn age(old: DateTime<Utc>, new: DateTime<Utc>, max_parts: i8) -> String {
    let mut seconds = new.signed_duration_since(old).num_seconds();
    let mut parts = 0;
    let mut output = "".to_string();

    let years = (seconds as f64 / (60.0 * 60.0 * 24.0 * 365.25)) as i64;
    if years > 0 {
        seconds -= (years as f64 * 60.0 * 60.0 * 24.0 * 365.25) as i64;
        output += &format!("{} years ", years);
        parts += 1;

        if parts == max_parts {
            return output;
        }
    }

    let months = seconds / (60 * 60 * 24 * 30);
    if months > 0 {
        seconds -= months * 60 * 60 * 24 * 30;
        output += &format!("{} months ", months);
        parts += 1;

        if parts == max_parts {
            return output;
        }
    }

    let weeks = seconds / (60 * 60 * 24 * 7);
    if weeks > 0 {
        seconds -= weeks * 60 * 60 * 24 * 7;
        output += &format!("{} weeks ", weeks);
        parts += 1;
        if parts == max_parts {
            return output;
        }
    }

    let days = seconds / (60 * 60 * 24);
    if days > 0 {
        seconds -= days * 60 * 60 * 24;
        output += &format!("{} days ", days);
        parts += 1;
        if parts == max_parts {
            return output;
        }
    }

    let hours = seconds / (60 * 60);
    if hours > 0 {
        seconds -= hours * 60 * 60;
        output += &format!("{} hours ", hours);
        parts += 1;
        if parts == max_parts {
            return output;
        }
    }

    let minutes = seconds / 60;
    if minutes > 0 {
        seconds -= minutes * 60;
        output += &format!("{} minutes ", minutes);
        parts += 1;
        if parts == max_parts {
            return output;
        }
    }

    output += &format!("{} seconds", seconds);
    output
}
