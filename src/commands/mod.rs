use once_cell::sync::OnceCell;

use crate::commands::meta::nodes::CommandNode;
use crate::{command, subcommands};
mod admin;
mod basic;
mod debug;
pub mod meta;
mod moderation;

static ROOT_NODE: OnceCell<CommandNode> = OnceCell::new();

pub fn get_root() -> &'static CommandNode {
    match ROOT_NODE.get() {
        Some(node) => node,
        None => {
            ROOT_NODE
                .set(subcommands!(
                    "ROOT",
                    None,
                    command!("coinflip", basic::coinflip),
                    command!("ping", basic::ping),
                    command!("echo", basic::echo),
                    command!("about", basic::about),
                    command!("userinfo", moderation::userinfo),
                    command!("get_config", debug::get_config),
                    command!("set_config", debug::set_config),
                    command!("quote", basic::quote),
                    command!("uid", basic::uid),
                    command!("redis_test", admin::restart)
                ))
                .ok()
                .unwrap();

            ROOT_NODE.get().unwrap()
        }
    }
}
