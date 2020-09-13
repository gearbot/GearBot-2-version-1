use std::collections::HashMap;
use std::str::FromStr;

use once_cell::sync::OnceCell;
use serde::Deserialize;
use twilight_model::id::EmojiId;

use crate::define_emoji;
use crate::utils::errors::Error;
use twilight_http::request::channel::reaction::RequestReactionType;
use twilight_model::channel::ReactionType;

define_emoji!(
    Yes => "✅",
    No => "🚫",
    Info => "ℹ️",
    Warn => "⚠️",
    Robot => "🤖",
    Bug => "🐛",
    Bad => "😶",
    GearDiamond => "⚙️",
    GearGold => "⚙️",
    GearIron => "⚙️",
    GearStone => "⚙️",
    GearWood => "⚙️",
    Left => "⬅️",
    Right => "➡️",

    StaffBadge => "",
    PartnerBadge => "",
    HypesquadEvents => "",
    BraveryBadge => "",
    BrillianceBadge => "",
    BalanceBadge => "",
    BugHunterBadge => "",
    EarlySupporterBadge => "",
    BugHunterLvl2Badge => "",
    VerifiedBotDevBadge => ""
);

#[derive(Debug)]
pub struct EmojiOverride {
    pub for_chat: String,
    pub id: EmojiId,
    pub name: String,
}

pub static EMOJI_OVERRIDES: OnceCell<HashMap<String, EmojiOverride>> = OnceCell::new();

#[macro_use]
mod macros {
    #[macro_export]
    macro_rules! define_emoji {
    ($($name: ident => $fallback: literal), *) => {


        #[derive(Deserialize, Debug, Clone)]
        pub enum Emoji {
            $( $name ,)*
        }

        impl std::fmt::Display for Emoji {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "{:?}", self)
            }
        }

        impl Emoji {

            pub fn get_fallback(&self)-> &'static str {
                match self {
                    $(Emoji::$name => $fallback ,)*
                }
            }

            pub fn for_chat(&self) -> &'static str {
                match EMOJI_OVERRIDES.get() {
                    Some(overrides) => match overrides.get(&self.to_string()) {
                        Some(thing) => &thing.for_chat,
                        None => self.get_fallback()
                    },
                    None => self.get_fallback()
                }
            }

            pub fn matches(&self, emoji: &ReactionType) -> bool {
                let o = match EMOJI_OVERRIDES.get() {
                    Some(overrides) => overrides.get(&self.to_string()),
                    None => None
                };
                match &emoji {
                    ReactionType::Custom { id, .. } => {
                        match o {
                            Some(o) => id.0 == o.id.0,
                            None => false
                        }
                    }
                    ReactionType::Unicode { name } => {
                        match o {
                            Some(_) => false,
                            None => *name == self.get_fallback()
                        }
                    }
                }
            }


            pub fn to_reaction(&self) -> RequestReactionType {
                let o = match EMOJI_OVERRIDES.get() {
                        Some(overrides) => overrides.get(&self.to_string()),
                        None => None
                    };
                if let Some(o) = o {
                    RequestReactionType::Custom{id: o.id, name: Some(o.name.clone())}
                } else {
                    RequestReactionType::Unicode{name: self.get_fallback().to_string()}
                }
            }
        }

        impl FromStr for Emoji {
            type Err = Error;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s.to_uppercase().as_str() {
                    $(stringify!($name) => Ok(Emoji::$name) ,)*
                    _ => Err(Error::UnknownEmoji(s.to_string())),
                }
            }
        }
     };
    }
}
