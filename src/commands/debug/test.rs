use twilight::model::channel::{Reaction, ReactionType};
use twilight::model::id::GuildId;

use crate::core::{CommandContext, Reactor};
use crate::utils::pattern::Pattern;
use crate::CommandResult;

pub async fn test(mut ctx: CommandContext) -> CommandResult {
    // let reactor = Reactors::new_emoji_list();
    // let reaction = Reaction {
    //     channel_id: Default::default(),
    //     emoji: ReactionType::Unicode { name: "".to_string() },
    //     guild_id: None,
    //     member: None,
    //     message_id: Default::default(),
    //     user_id: Default::default(),
    // };
    // if reactor.processes(&reaction) {
    //     reactor
    //         .do_your_thing(ctx.bot_context, &reaction, ctx.message.get_author_as_member()?)
    //         .await;
    // }

    let out = Pattern::new(ctx.parser.parts.len() - 1)
        .arrange(
            ctx.parser
                .parts
                .iter()
                .skip(1)
                .map(String::from)
                .collect::<Vec<String>>(),
        )
        .iter()
        .map(|inner| inner.join("").to_string())
        .collect::<Vec<String>>()
        .join("\n");

    ctx.reply_raw(out).await?;
    Ok(())
}
