use once_cell::sync::OnceCell;

use crate::commands::meta::nodes::{CommandGroup, CommandNode, GearBotPermission, RootNode};
use crate::{command, command_with_subcommands, command_with_subcommands_and_handler};
use lazy_static::lazy_static;
use std::collections::HashMap;
use twilight::model::guild::Permissions;

mod admin;
mod basic;
mod debug;
pub mod meta;
mod moderation;

lazy_static! {
    pub static ref ROOT_NODE: RootNode = {
        let commandlist = vec![
            command!(
                "about",
                basic::about,
                Permissions::EMBED_LINKS,
                GearBotPermission::AboutCommand,
                CommandGroup::Basic
            ),
            command!(
                "coinflip",
                basic::coinflip,
                Permissions::empty(),
                GearBotPermission::CoinflipCommand,
                CommandGroup::Basic
            ),
            command!(
                "ping",
                basic::ping,
                Permissions::empty(),
                GearBotPermission::PingCommand,
                CommandGroup::Basic
            ),
            command_with_subcommands!(
                "config",
                Permissions::empty(),
                GearBotPermission::ConfigCommand,
                CommandGroup::Admin,
                command_with_subcommands_and_handler!(
                    "get",
                    debug::get_config,
                    Permissions::empty(),
                    GearBotPermission::GetConfigCommand,
                    CommandGroup::Admin,
                    command!(
                        "pretty",
                        debug::get_config_pretty,
                        Permissions::empty(),
                        GearBotPermission::GetConfigCommand,
                        CommandGroup::Admin
                    )
                ),
                command!(
                    "set",
                    debug::set_config,
                    Permissions::empty(),
                    GearBotPermission::SetConfigCommand,
                    CommandGroup::Admin
                )
            ),
            command!(
                "userinfo",
                moderation::userinfo,
                Permissions::EMBED_LINKS,
                GearBotPermission::UserInfoCommand,
                CommandGroup::Moderation
            ),
            command_with_subcommands!(
                "check",
                Permissions::empty(),
                GearBotPermission::AdminGroup,
                CommandGroup::Admin,
                command!(
                    "cache",
                    admin::check_cache,
                    Permissions::EMBED_LINKS,
                    GearBotPermission::AdminGroup,
                    CommandGroup::Admin
                )
            ),
        ];

        let mut commands = HashMap::new();

        for command in commandlist {
            commands.insert(command.name.clone(), command);
        }

        RootNode { all_commands: commands }
    };
}
