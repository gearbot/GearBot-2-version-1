use std::pin::Pin;
use std::sync::Arc;

use once_cell::sync::OnceCell;
use serde::export::Formatter;
use twilight::model::channel::Message;
use twilight::model::gateway::payload::MessageCreate;

use lazy_static::lazy_static;

use crate::{command, pin_box, subcommands};
use crate::commands::meta::nodes::CommandNode;
use crate::core::Context;
use crate::parser::parser::Parser;
use crate::utils::errors::Error;

pub mod basic;
pub mod meta;

static ROOT_NODE: OnceCell<CommandNode> = OnceCell::new();

pub fn get_root() -> &'static CommandNode {
    match ROOT_NODE.get()
    {
        Some(node) => node,
        None => {
            ROOT_NODE.set(subcommands!("ROOT", None,
                command!("ping", basic::ping),
                command!("echo", basic::echo),
                subcommands!("sub", None,
                        command!("ping", basic::ping))
 ));
            ROOT_NODE.get().unwrap()
        }
    }
}