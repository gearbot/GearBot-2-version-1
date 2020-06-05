use std::fmt;
use std::sync::atomic::Ordering;

use chrono::Utc;
use twilight::builders::embed::EmbedBuilder;
use twilight::model::channel::Message;

use crate::commands::meta::nodes::CommandResult;
use crate::core::{BotStats, CommandContext};
use crate::parser::Parser;

const ABOUT_EMBED_COLOR: u32 = 0x00_cea2;

struct AboutUptime {
    days: u64,
    hours: u64,
    minutes: u64,
    seconds: u64,
}

impl fmt::Display for AboutUptime {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} days, {} hours, {} minutes, {} seconds",
            self.days, self.hours, self.minutes, self.seconds
        )
    }
}

struct AboutDescription {
    uptime: AboutUptime,
    user_messages: usize,
    bot_messages: usize,
    my_messages: usize,
    errors: usize,
    commands_ran: usize,
    custom_commands_ran: usize,
    guilds: usize,
    users: usize,
    unique_users: usize,
    tacos_eaten: usize,
    version: &'static str,
}

impl AboutDescription {
    async fn create(stats: &BotStats) -> Self {
        let (users, unique_users) = {
            // This is the list of all the users that we can see, which
            // means that it has no duplicates.
            // TODO: Find a way to access this
            let unique_users = 1_000_000; // ctx.cache.0.members.len();
                                          // let mut total_users: usize = 0;
            let total_users = 1_500_000;

            // for guild_id in ctx.cache.0.guilds {
            //     if let Ok(guild_members) = ctx.http.get_guild_members(guild_id.0, None, None).await {
            //         total_users += guild_members.len()
            //     }
            // }

            (total_users as usize, unique_users as usize)
        };

        let uptime = {
            let current_time = Utc::now();
            let diff = current_time - stats.start_time;

            let total_secs = diff.to_std().unwrap().as_secs();

            let (hours, remainder) = (total_secs / 3600, total_secs % 3600);
            let (days, hours) = (hours / 24, hours % 24);
            let (minutes, seconds) = (remainder / 60, remainder % 60);

            AboutUptime {
                days,
                hours,
                minutes,
                seconds,
            }
        };

        let tacos_eaten = {
            let seconds_running = 3;
            // uptime.timestamp() as usize;
            // Below assumes that every user has been with us since the start. Maybe
            // this could be changed
            // If a person can eat a taco every 5 mins, the following formula applies:

            let tacos_per_user = seconds_running / 300; // 300 seconds = 5 minutes

            println!("Each user has eaten {} tacos themselves!", tacos_per_user);

            tacos_per_user * unique_users
        };

        AboutDescription {
            uptime,
            user_messages: stats.user_messages.load(Ordering::Relaxed),
            bot_messages: stats.bot_messages.load(Ordering::Relaxed),
            my_messages: stats.my_messages.load(Ordering::Relaxed),
            errors: stats.error_count.load(Ordering::Relaxed),
            commands_ran: stats.commands_ran.load(Ordering::Relaxed),
            custom_commands_ran: stats.custom_commands_ran.load(Ordering::Relaxed),
            guilds: stats.guilds.load(Ordering::Relaxed),
            users,
            unique_users,
            tacos_eaten,
            version: stats.version,
        }
    }
}

impl fmt::Display for AboutDescription {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "
            The Gears have been spinning for {}
            I have received {} user messages, {} bot messages ({} were mine)
            Number of times people have grinded my commands: {}
            {} commands have been executed, as well as {} custom commands
            Working in {} guilds
            With a total of {} users ({} unique)
            Together we could of eaten {} tacos in this time
            GearBot version {}
        ",
            self.uptime.to_string(),
            self.user_messages,
            self.bot_messages,
            self.my_messages,
            self.errors,
            self.commands_ran,
            self.custom_commands_ran,
            self.guilds,
            self.users,
            self.unique_users,
            self.tacos_eaten,
            self.version
        )
    }
}

pub async fn about(ctx: CommandContext, msg: Message, _: Parser) -> CommandResult {
    let about_stats = AboutDescription::create(ctx.get_bot_stats()).await;

    let embed = EmbedBuilder::new()
        .color(ABOUT_EMBED_COLOR)
        .description(about_stats.to_string())
        .timestamp(Utc::now().to_rfc3339())
        .add_field("Support Server", "[Click Here](https://discord.gg/vddW3D9)")
        .inline()
        .commit()
        .add_field("Website", "[Click Here](https://gearbot.rocks)")
        .inline()
        .commit()
        .add_field("GitHub", "[Click Here](https://github.com/gearbot/GearBot)")
        .inline()
        .commit()
        .build();

    ctx.send_embed(embed, msg.channel_id).await?;

    Ok(())
}
