pub mod nodes;

#[macro_use]
pub mod macros {
    #[macro_export]
    macro_rules! pin_box {
        ($e: expr) => {
            Box::new(move |ctx| Box::pin($e(ctx)))
        };
    }

    #[macro_export]
    macro_rules! command_with_aliases {
        ($name: literal, $a: expr, $e: expr, $bot_permissions: expr, $command_permission: expr, $group: expr) => {{
            Arc::new(CommandNode {
                name: String::from($name),
                handler: Some(Box::new(move |ctx| Box::pin($e(ctx)))),
                sub_nodes: HashMap::new(),
                node_list: vec![],
                bot_permissions: $bot_permissions,
                command_permission: $command_permission,
                group: $group,
                aliases: $a,
            })
        }};
    }

    #[macro_export]
    macro_rules! command {
        ($name: literal, $e: expr, $bot_permissions: expr, $command_permission: expr, $group: expr) => {
            $crate::command_with_aliases!($name, vec![], $e, $bot_permissions, $command_permission, $group)
        };
    }

    #[macro_export]
    macro_rules! command_with_subcommands_and_handler_and_aliases {
        ($name: literal, $a: expr, $e: expr, $bot_permissions: expr, $command_permission: expr, $group: expr, $($node: expr),*) => {
        {
        let mut map = ::std::collections::HashMap::new();
        let mut list = vec![];
         $(
         let node = $node;
         for a in &node.aliases {
            if map.contains_key(&*a) {
                panic!(format!("Tried to register subcommand alias {} but a subcommand is already registered under this name", a));
            }
            map.insert(a.clone(), node.clone());
         }
         if map.contains_key(&*node.name) {
            panic!(format!("Tried to register subcommand name {} but a subcommand is already registered under this name", &node.name))
         }
         map.insert(String::from(node.name.clone()), node.clone());
         list.push(node);
         )*
        Arc::new(CommandNode {
                name: String::from($name),
                handler: Some(Box::new(move |ctx| Box::pin($e(ctx)))),
                sub_nodes: map,
                node_list: list,
                bot_permissions: $bot_permissions,
                command_permission: $command_permission,
                group: $group,
                aliases: $a
            })
        }
        }
    }

    #[macro_export]
    macro_rules! command_with_subcommands_and_handler {
        ($name: literal, $e: expr, $bot_permissions: expr, $command_permission: expr, $group: expr, $($node: expr),*) => {
        {
        let mut map = ::std::collections::HashMap::new();
        let mut list = vec![];
         $(
         let node = $node;
         for a in &node.aliases {
            if (map.contains_key(a)) {
                panic!(format!("Tried to register subcommand alias {} but a subcommand is already registered under this name", a));
            }
            map.insert(a.clone(), node.clone());
         }
         if (map.contains_key(&node.name)){
            panic!(format!("Tried to register subcommand name {} but a subcommand is already registered under this name", &node.name))
         }
         map.insert(String::from(node.name.clone()), node.clone());
         list.push(node);
         )*
        Arc::new(CommandNode {
                name: String::from($name),
                handler: Some(Box::new(move |ctx| Box::pin($e(ctx)))),
                sub_nodes: map,
                node_list: list,
                bot_permissions: $bot_permissions,
                command_permission: $command_permission,
                group: $group,
                aliases: vec![]
            })
        }
        }
    }

    #[macro_export]
    macro_rules! command_with_subcommands_and_aliases {
        ($name: literal, $a: expr, $bot_permissions: expr, $command_permission: expr, $group: expr, $($node: expr),*) => {
        {
        let mut map = ::std::collections::HashMap::new();
        let mut list = vec![];
         $(
          let node = $node;
          for a in &node.aliases {
            if map.contains_key(&*a) {
                panic!(format!("Tried to register subcommand alias {} but a subcommand is already registered under this name", a));
            }
            map.insert(a.clone(), node.clone());
         }
         if map.contains_key(&*node.name) {
            panic!(format!("Tried to register subcommand name {} but a subcommand is already registered under this name", &node.name))
         }
          map.insert(String::from(node.name.clone()), node.clone());
          list.push(node);
          )*
        Arc::new(CommandNode {
                name: String::from($name),
                handler: None,
                sub_nodes: map,
                bot_permissions: $bot_permissions,
                command_permission: $command_permission,
                group: $group,
                aliases: $a,
                node_list: list
            })
        }
        }
    }

    #[macro_export]
    macro_rules! command_with_subcommands {
        ($name: literal, $command_permission: expr, $group: expr, $($node: expr),*) => {
        {
        let mut map = ::std::collections::HashMap::new();
        let mut list = vec![];
         $(
          let node = $node;
          for a in &node.aliases {
            if map.contains_key(&*a) {
                panic!(format!("Tried to register subcommand alias {} but a subcommand is already registered under this name", a));
            }
            map.insert(a.clone(), node.clone());
         }
         if map.contains_key(&*node.name) {
            panic!(format!("Tried to register subcommand name {} but a subcommand is already registered under this name", &node.name))
         }
          map.insert(String::from(node.name.clone()), node.clone());
          list.push(node);
          )*
        Arc::new(CommandNode {
                name: String::from($name),
                handler: None,
                sub_nodes: map,
                bot_permissions: Permissions::empty(),
                command_permission: $command_permission,
                group: $group,
                aliases: vec![],
                node_list: list
            })
        }
        }
        }
}
