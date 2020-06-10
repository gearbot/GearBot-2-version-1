use rand;

use crate::commands::meta::nodes::CommandResult;
use crate::core::CommandContext;
use crate::parser::Parser;
use crate::translation::{FluArgs, GearBotString};
use crate::utils;

pub async fn coinflip(ctx: CommandContext, parser: Parser) -> CommandResult {
    let thing_todo: String = parser
        .parts
        .into_iter()
        .skip(1)
        .collect::<Vec<String>>()
        .join(" ");

    let thing_todo = if !thing_todo.is_empty() {
        //todo: couple links to invoking user having embed perms
        utils::clean(&thing_todo, false, true, false, false)
    } else {
        ctx.translate(GearBotString::CoinflipDefault)
    };

    let key = if rand::random() {
        GearBotString::CoinflipYes
    } else {
        GearBotString::CoinflipNo
    };

    let args = FluArgs::with_capacity(1)
        .insert("input", thing_todo)
        .generate();

    ctx.reply(key, args).await?;

    Ok(())
}
