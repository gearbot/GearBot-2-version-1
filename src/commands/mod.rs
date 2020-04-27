use once_cell::sync::OnceCell;

use crate::commands::meta::nodes::CommandNode;
use crate::{command, subcommands};
pub mod basic;
pub mod meta;

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
                    command!("about", basic::about)
                ))
                .ok()
                .unwrap();

            ROOT_NODE.get().unwrap()
        }
    }
}
