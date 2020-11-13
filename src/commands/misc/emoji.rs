use crate::core::reactors::gen_emoji_page;
use crate::core::{CommandContext, Reactor};
use crate::error::CommandResult;
use crate::utils::Emoji;

pub async fn emoji_list(ctx: CommandContext) -> CommandResult {
    let guild = ctx.get_guild();
    let guild_config = &ctx.get_config()?;

    let reactor = Reactor::new_emoji_list();
    let pages = guild.emoji.len() as u8 + 1;
    let page = gen_emoji_page(0, pages, guild, guild_config, &ctx.bot_context).await?;

    let message = ctx.reply_embed(page).await?;
    reactor.save(&ctx.bot_context, message.id).await?;

    ctx.bot_context
        .http
        .create_reaction(message.channel_id, message.id, Emoji::Left.to_reaction())
        .await?;

    ctx.bot_context
        .http
        .create_reaction(message.channel_id, message.id, Emoji::Right.to_reaction())
        .await?;

    Ok(())
}

pub async fn emoji_info(_ctx: CommandContext) -> CommandResult {
    Ok(())
}
