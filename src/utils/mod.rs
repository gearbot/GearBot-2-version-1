// Remove this when they are all used.
#[allow(dead_code)]
pub mod matchers;

pub mod emoji;
mod errors;

pub use emoji::*;
pub use errors::*;

const MARKDOWN_REPALCEMENTS: &[&str; 7] = &["\\", "*", "_", "~", "|", "{", ">"];

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

fn replace_lookalikes(msg: &mut String) -> String {
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
