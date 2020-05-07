use std::collections::HashMap;
use std::str::FromStr;

use once_cell::sync::OnceCell;
use serde::Deserialize;

use crate::define_emoji;
use crate::utils::errors::Error;

define_emoji!(
    Yes => "✅",
    No => "🚫",
    Info => "ℹ️",
    Warn => "⚠️",
    Robot => "🤖",
    Bug => "🐛",

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

pub static EMOJI_OVERRIDES: OnceCell<HashMap<String, String>> = OnceCell::new();

#[macro_use]
mod macros {
    #[macro_export]
    macro_rules! define_emoji {
    ($($name: ident => $fallback: literal), *) => {


        #[derive(Deserialize, Debug)]
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
                        Some(thing) => thing,
                        None => self.get_fallback()
                    },
                    None => self.get_fallback()
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
