use crate::commands::meta::nodes::{CommandGroup, CommandNode, GearBotPermissions, RootNode};
use crate::{
    command, command_with_aliases, command_with_subcommands, command_with_subcommands_and_aliases,
    command_with_subcommands_and_handler_and_aliases,
};
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::Arc;
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
                GearBotPermissions::ABOUT_COMMAND,
                CommandGroup::Basic
            ),
            command!(
                "coinflip",
                basic::coinflip,
                Permissions::empty(),
                GearBotPermissions::COINFLIP_COMMAND,
                CommandGroup::Basic
            ),
            command!(
                "ping",
                basic::ping,
                Permissions::empty(),
                GearBotPermissions::PING_COMMAND,
                CommandGroup::Basic
            ),
            command!(
                "quote",
                basic::quote,
                Permissions::EMBED_LINKS,
                GearBotPermissions::QUOTE_COMMAND,
                CommandGroup::Basic
            ),
            command!(
                "uid",
                basic::uid,
                Permissions::empty(),
                GearBotPermissions::UID_COMMAND,
                CommandGroup::Basic
            ),
            command_with_subcommands_and_aliases!(
                "config",
                vec![String::from("c")],
                Permissions::empty(),
                GearBotPermissions::READ_CONFIG, //lock behind only read, technically possible to have write without read but no my problem if you set stupid settings
                CommandGroup::GuildAdmin,
                command_with_subcommands_and_handler_and_aliases!(
                    "get",
                    vec![String::from("g")],
                    debug::get_config,
                    Permissions::empty(),
                    GearBotPermissions::READ_CONFIG,
                    CommandGroup::GuildAdmin,
                    command_with_aliases!(
                        "pretty",
                        vec![String::from("p")],
                        debug::get_config_pretty,
                        Permissions::empty(),
                        GearBotPermissions::READ_CONFIG,
                        CommandGroup::GuildAdmin
                    )
                ),
                command!(
                    "set",
                    debug::set_config,
                    Permissions::empty(),
                    GearBotPermissions::WRITE_CONFIG,
                    CommandGroup::GuildAdmin
                )
            ),
            command!(
                "userinfo",
                moderation::userinfo,
                Permissions::EMBED_LINKS,
                GearBotPermissions::USERINFO_COMMAND,
                CommandGroup::Moderation
            ),
            command_with_subcommands!(
                "check",
                Permissions::empty(),
                GearBotPermissions::BOT_ADMIN,
                CommandGroup::BotAdmin,
                command!(
                    "cache",
                    admin::check_cache,
                    Permissions::EMBED_LINKS,
                    GearBotPermissions::BOT_ADMIN,
                    CommandGroup::BotAdmin
                )
            ),
            command!(
                "restart",
                admin::restart,
                Permissions::empty(),
                GearBotPermissions::BOT_ADMIN,
                CommandGroup::BotAdmin
            ),
        ];

        let mut all_commands = HashMap::new();
        let mut command_list = vec![];
        let mut by_group = HashMap::new();

        for command in commandlist {
            command_list.push(command.clone());
            if all_commands.contains_key(&*command.name) {
                panic!(format!(
                    "Tried to register command name {} but another command was already registered with that name!",
                    command.name
                ))
            }
            all_commands.insert(command.name.clone(), command.clone());

            for a in &command.aliases {
                if all_commands.contains_key(a) {
                    panic!(format!(
                        "Tried to register command alias {} but another command was already registered with that name!",
                        a
                    ))
                }
                all_commands.insert(a.clone(), command.clone());
            }

            let mut list = match by_group.remove(&command.group) {
                Some(list) => list,
                None => vec![],
            };
            list.push(command.clone());
            by_group.insert(command.group.clone(), list);
        }

        RootNode {
            all_commands,
            command_list,
            by_group,
        }
    };
}
