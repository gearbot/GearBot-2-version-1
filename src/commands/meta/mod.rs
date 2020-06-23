// Remove this when they are used later
#[allow(dead_code, unused_variables)]
pub mod nodes;

#[macro_use]
pub mod macros {
    #[macro_export]
    macro_rules! pin_box {
        ($e: expr) => {
            Box::new(move |ctx, msg, parser| Box::pin($e(ctx, msg, parser)))
        };
    }

    #[macro_export]
    macro_rules! command {
        ($name: literal, $e: expr) => {
            CommandNode::create_command(String::from($name), Box::new(move |ctx, parser| Box::pin($e(ctx, parser))))
        };
    }

    #[macro_export]
    macro_rules! subcommands {
    ( $node_name :expr, $node_handler:expr, $($node: expr),*) => {{
         let mut map = ::std::collections::HashMap::new();
         $( map.insert(String::from($node.get_name().clone()), $node); )*

         CommandNode::create_node(String::from($node_name), $node_handler, map)
    }}
    }
}
