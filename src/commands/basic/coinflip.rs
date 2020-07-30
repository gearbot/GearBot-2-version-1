use crate::commands::meta::nodes::CommandResult;
use crate::core::CommandContext;
use crate::translation::{FluArgs, GearBotString};
use crate::utils;

pub async fn coinflip(ctx: CommandContext) -> CommandResult {
    let string_len: usize = ctx.parser.parts.iter().map(String::len).sum::<usize>() + ctx.parser.parts.len();

    let thing_todo = ctx
        .parser
        .parts
        .iter()
        .skip(1)
        .fold(String::with_capacity(string_len), |mut s, part| {
            s += part;
            s.push(' ');
            s
        });

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

    let args = FluArgs::with_capacity(1).insert("input", thing_todo).generate();

    ctx.reply(key, args).await?;

    Ok(())
}
