
use lazy_static::lazy_static;
use regex::{Regex, RegexBuilder};

pub fn contains_id(msg: &str) -> bool {
    ID_MATCHER.is_match(msg)
}

pub fn contains_role_id(msg: &str) -> bool {
    ROLE_ID_MATCHER.is_match(msg)
}

pub fn contains_channel_id(msg: &str) -> bool {
    CHANNEL_ID_MATCHER.is_match(msg)
}

pub fn contains_mention(msg: &str) -> bool {
    MENTION_MATCHER.is_match(msg)
}

pub fn contains_url(msg: &str) -> bool {
    // Url::parse(msg).is_ok()
    URL_MATCHER.is_match(msg)
}

pub fn contains_emote(msg: &str) -> bool {
    EMOJI_MATCHER.is_match(msg)
}

pub fn contains_jump_link(msg: &str) -> bool {
    JUMP_LINK_MATCHER.is_match(msg)
}

pub fn contains_modifier(msg: &str) -> bool {
    MODIFIER_MATCHER.is_match(msg)
}

pub fn starts_with_number(msg: &str) -> bool {
    START_WITH_NUMBER_MATCHER.is_match(msg)
}

pub fn contains_invite_link(msg: &str) -> bool {
    INVITE_MATCHER.is_match(msg)
}

lazy_static! {
    static ref ID_MATCHER: Regex = {
        Regex::new(r"<@!?([0-9]+)>").unwrap()
    }; 
}

lazy_static! {
    static ref ROLE_ID_MATCHER: Regex = {
        Regex::new(r"<@&([0-9]+)>").unwrap()
    };
}

lazy_static! {
    static ref CHANNEL_ID_MATCHER: Regex = {
        Regex::new(r"<#([0-9]+)>").unwrap()
    }; 
}

lazy_static! {
    static ref MENTION_MATCHER: Regex = {
        Regex::new(r"<@[!&]?\\d+>").unwrap()
    }; 
}

// TODO: Should all the URL matching be replaced with the url crate?
lazy_static! {
    static ref URL_MATCHER: Regex = {
        RegexBuilder::new(r"((?:https?:)[a-z0-9]+(?:[-._][a-z0-9]+)*\.[a-z]{2,5}(?::[0-9]{1,5})?(?:/[^ \n<>]*)?)")
            .case_insensitive(true)
            .build()
            .unwrap()
    }; 
}

lazy_static! {
    static ref EMOJI_MATCHER: Regex = {
        Regex::new(r"<(a?):([^:\n]+):([0-9]+)>").unwrap()
    }; 
}

lazy_static! {
    static ref JUMP_LINK_MATCHER: Regex = {
        Regex::new(r"https://(?:canary|ptb)?\.?discordapp.com/channels/\d*/(\d*)/(\d*)").unwrap()
    }; 
}

lazy_static! {
    static ref MODIFIER_MATCHER: Regex = {
        Regex::new(r"^\[(.*):(.*)\]$").unwrap()
    };

}

lazy_static! {
    static ref START_WITH_NUMBER_MATCHER: Regex = {
        Regex::new(r"^(\d+)").unwrap()
    }; 
}

lazy_static! {
    static ref INVITE_MATCHER: Regex = {
        // TODO: This needs re-written without look-around/behind, Rust Regex doesn't support it in favor of higher performance.
        RegexBuilder::new(r"(?:https?://)?(?:www\.)?(?:discord(?:\.| |\[?\(?'?'?dot'?'?\)?\]?)?(?:gg|io|me|li)|discordapp\.com/invite)/+((?:(?!https?)[\w\d-])+)")
            .case_insensitive(true)
            .build()
            .unwrap()
    }; 
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_id_works() {
        let msg = "<@282830930237292>";
        let msg_2 = "<@!383738338398392>";
        let control = "Hello there";

        assert_eq!(contains_id(msg), true);
        assert_eq!(contains_id(msg_2), true);
        assert_eq!(contains_id(control), false);
    }
    
    #[test]
    fn role_id_works() {
        let msg = "<@&3892320392392>";
        let control = "<@#439332392320>";

        assert_eq!(contains_role_id(msg), true);
        assert_eq!(contains_role_id(control), false);
    }

    #[test]
    fn channel_id_works() {
        let msg = "<#7012116760323232>";
        let control = "<@!39238293809232>";

        assert_eq!(contains_channel_id(msg), true);
        assert_eq!(contains_channel_id(control), false);
    }
        
    #[test]
    fn mention_matcher_works() {
        // TODO: This doesn't work. The Rust regex engine may be intrepreting
        // the regex differently.
        let msg = "<@!32923232327837278932>";
        let msg_2 = "<@&32923232327837278932>";
        let control = "Just a normal message, how are you today?";

        assert_eq!(contains_mention(msg), true);
        assert_eq!(contains_mention(msg_2), true);
        assert_eq!(contains_mention(control), false);
    }
    
    #[test]
    fn url_matcher_works() {
        // TODO: This doesn't work.
        let msg = "Hey, check out this not shady website: https://google.com";
        let msg2 = "Go to example.com for free money!";
        let msg3 = "https://google.com";
        let control = "I would never give you a sketchy URL";

        assert_eq!(contains_url(msg), true);
        assert_eq!(contains_url(msg2), true);
        assert_eq!(contains_url(msg3), true);
        assert_eq!(contains_url(control), false);
    }

    #[test]
    fn emote_matcher_works() {
        // This doesn't work.
        let msg = ":computer:";
        let msg2 = "<:someCustomEmote:3747384343434>";
        let control = "Hello there";

        assert_eq!(contains_emote(msg), true);
        assert_eq!(contains_emote(msg2), true);
        assert_eq!(contains_emote(control), false)
    }

    #[test]
    fn jump_link_matcher_works() {
        let msg = " Check this out: https://canary.discordapp.com/channels/365498559174410241/365498559723732993/606145193766551552";
        let msg2 = "https://discordapp.com/channels/365498559174410241/365498559723732993/606145193766551552";
        let msg3 = "https://ptb.discordapp.com/channels/365498559174410241/365498559723732993/606145193766551552";
        let control = "No link here";

        assert_eq!(contains_jump_link(msg), true);
        assert_eq!(contains_jump_link(msg2), true);
        assert_eq!(contains_jump_link(msg3), true);
        assert_eq!(contains_jump_link(control), false);
    }

    #[test]
    fn modifier_matcher_works() {
        // This doesnt work
        let msg = "TODO";
        let msg2 = "TODO2";
        let control = "Test";

        assert_eq!(contains_modifier(msg), true);
        assert_eq!(contains_modifier(msg2), true);
        assert_eq!(contains_modifier(control), false)
    }

    #[test]
    fn starts_with_number_works() {
        let msg = "1 birthday a year, only!";
        let msg2 = "25 bugs on the wall...";
        let control = "Numbers are evil, so is math";

        assert_eq!(starts_with_number(msg), true);
        assert_eq!(starts_with_number(msg2), true);
        assert_eq!(starts_with_number(control), false);
    }

    #[test]
    fn invite_matcher_works() {
        let msg = "https://discord.gg/vddW3D9";
        let control = "I don't have my own server :(";

        assert_eq!(contains_invite_link(msg), true);
        assert_eq!(contains_invite_link(control), false);
    }
}