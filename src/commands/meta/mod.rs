pub mod nodes;

#[macro_use]
pub mod macros {
    #[macro_export]
    macro_rules! pin_box {
        ($e: expr) => {
            Box::new(move |ctx, parser| Box::pin($e(ctx, parser)))
        };
    }

    #[macro_export]
    macro_rules! command {
        ($name: literal, $e: expr, $bot_permissions: expr, $command_permission: expr, $group: expr) => {
            CommandNode {
                name: String::from($name),
                handler: Some(Box::new(move |ctx, parser| Box::pin($e(ctx, parser)))),
                sub_nodes: HashMap::new(),
                bot_permissions: $bot_permissions,
                command_permission: $command_permission,
                group: $group,
            }
        };
    }

    #[macro_export]
    macro_rules! command_with_subcommands_and_handler {
        ($name: literal, $e: expr, $bot_permissions: expr, $command_permission: expr, $group: expr, $($node: expr),*) => {
        {
        let mut map = ::std::collections::HashMap::new();
         $(
         let node = $node;
         map.insert(String::from(node.name.clone()), node);
         )*
        CommandNode {
                name: String::from($name),
                handler: Some(Box::new(move |ctx, parser| Box::pin($e(ctx, parser)))),
                sub_nodes: map,
                bot_permissions: $bot_permissions,
                command_permission: $command_permission,
                group: $group,
            }
        }
        }
    }

    #[macro_export]
    macro_rules! command_with_subcommands {
        ($name: literal, $bot_permissions: expr, $command_permission: expr, $group: expr, $($node: expr),*) => {
        {
        let mut map = ::std::collections::HashMap::new();
         $(
          let node = $node;
          map.insert(String::from(node.name.clone()), node);
          )*
        CommandNode {
                name: String::from($name),
                handler: None,
                sub_nodes: map,
                bot_permissions: $bot_permissions,
                command_permission: $command_permission,
                group: $group,
            }
        }
        }
    }
}
