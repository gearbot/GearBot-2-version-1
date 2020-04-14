use std::fmt;
use std::sync::{Arc, atomic::Ordering};

use chrono::{DateTime, Utc, NaiveTime};
use chrono::Duration;
use twilight::model::channel::Message;
use twilight::builders::embed::EmbedBuilder;

use crate::CommandResult;
use crate::core::Context;

const EMBED_UPTIME_FORMAT: &str = "TODO days, %H hours, %M minutes, %S seconds";

struct AboutDescription {
    uptime: NaiveTime,
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
    async fn from(ctx: &Context<'_>) -> Self {
        let stats = &ctx.stats;
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

        // TODO: Fix this
        let uptime = {
            let current_time = Utc::now();
            let old_dur = Duration::seconds(stats.start_time.timestamp());
            let diff = current_time - old_dur;
            diff.time()
        };

        println!("We have been up for: {}", uptime);

        let tacos_eaten = {
            let seconds_running = 3; // uptime.timestamp() as usize;    
            // Below assumes that every user has been with us since the start. Maybe 
            // this could be changed
            // If a person can eat a taco every 5 mins, the following formula applies:
            
            let tacos_per_user = seconds_running / 300; // 300 seconds = 5 minutes

            println!("Each user has eaten {} tacos themselves!", tacos_per_user);
    
            tacos_per_user * unique_users
        };


        AboutDescription {
            uptime: uptime,
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
            version: stats.version
        }
    }
}

impl fmt::Display for AboutDescription {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> fmt::Result { 
        write!(f, "
            The Gears have been spinning for {}
            I have received {} user messages, {} bot messages ({} were mine)
            Number of times people have grinded my gears: {}
            {} commands have been executed, as well as {} custom commands
            Working in {} guilds
            With a total of {} users ({} unique)
            Together we could of eaten {} tacos in this time
            GearBot version {}
        ",  self.uptime.format(EMBED_UPTIME_FORMAT), self.user_messages, self.bot_messages, self.my_messages, 
            self.errors, self.commands_ran, self.custom_commands_ran, self.guilds,
            self.users,  self.unique_users, self.tacos_eaten, self.version
        )
    }
}


pub async fn about(ctx: &Arc<Context<'_>>, msg: &Message) -> CommandResult {
    let about_stats = AboutDescription::from(ctx).await;

    let about_embed = {
        let mut embed = EmbedBuilder::new()
            .color(0x00cea2)
            .description(about_stats.to_string())
            .timestamp(Utc::now().to_rfc3339());

        embed.add_field("Support Server", "[Click Here](https://discord.gg/vddW3D9)").inline().commit();
        embed.add_field("Website", "[Click Here](https://gearbot.rocks)").inline().commit();
        embed.add_field("GitHub", "[Click Here](https://github.com/gearbot/GearBot)").inline().commit();
        embed.build()
    }; 
    
    ctx.http.create_message(msg.channel_id)
        .embed(about_embed)
        .await?;
    
    Ok(())
}