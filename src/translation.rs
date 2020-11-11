use std::borrow::Cow;
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;

use fluent_bundle::{concurrent::FluentBundle, FluentArgs, FluentError, FluentResource, FluentValue};
use unic_langid::{langid, LanguageIdentifier};

use crate::gearbot_warn;

const TRANSLATION_DIR: &str = "./lang";
const FAILED_TRANSLATE_FALLBACK_MSG: &str =
    "A translation error occured and no fallback could be found! Something may be wrong with the guild configuration!";

/// The default language to fall back to if a string can't be translated in the requested language.
/// This is also the language that new guild configs will default to.
pub const DEFAULT_LANG: LanguageIdentifier = langid!("en_US");

/// The transations for all languages that the bot can handle.
pub struct Translations(HashMap<LanguageIdentifier, Arc<FluentBundle<FluentResource>>>);

pub struct FluArgs<'a>(FluentArgs<'a>);

impl<'a> FluArgs<'a> {
    pub fn with_capacity(cap: usize) -> Self {
        Self(FluentArgs::with_capacity(cap))
    }

    pub fn add<P>(mut self, key: &'a str, value: P) -> Self
    where
        P: Into<FluentValue<'a>>,
    {
        self.0.add(key, value.into());
        self
    }

    pub fn generate(self) -> FluentArgs<'a> {
        self.0
    }
}

impl Translations {
    /// Retreives a string key to use when sending a message to chat that *does not* require arguments and can be sent as fetched with no
    /// further modifications.
    pub fn get_text_plain(&self, lang_key: &LanguageIdentifier, string_key: GearBotString) -> Cow<str> {
        // TODO: See how well this will work out in practice with unwrapping
        let lang_bundle = self.0.get(lang_key).unwrap();

        if let Some(expected_msg) = lang_bundle.get_message(string_key.as_str()) {
            let mut errors = Vec::new();

            let pattern = expected_msg.value.unwrap();

            let value = lang_bundle.format_pattern(pattern, None, &mut errors);

            handle_translation_error(&errors, string_key, false);

            value
        } else {
            // If we can't find the key in the expected language, fallback to English
            let fallback_bundle = self.0.get(&DEFAULT_LANG).unwrap();

            if let Some(fallback_msg) = fallback_bundle.get_message(string_key.as_str()) {
                let mut errors = Vec::new();

                let pattern = fallback_msg.value.unwrap();

                let value = lang_bundle.format_pattern(pattern, None, &mut errors);

                handle_translation_error(&errors, string_key, true);

                value
            } else {
                // Something really went wrong, error in chat and the logs
                gearbot_warn!("{}", FAILED_TRANSLATE_FALLBACK_MSG);
                Cow::Borrowed(FAILED_TRANSLATE_FALLBACK_MSG)
            }
        }
    }

    /// Retreives a string key to use when sending a message to chat that *does* require arguments and must have some fields
    /// passed in to have included before it can be sent.
    ///
    /// For example, the ping command.
    pub fn get_text_with_args<'a>(
        &'a self,
        lang_key: &LanguageIdentifier,
        string_key: GearBotString,
        args: &'a FluentArgs<'a>,
    ) -> Cow<'a, str> {
        let lang_bundle = self.0.get(lang_key).unwrap();

        if let Some(expected_msg) = lang_bundle.get_message(string_key.as_str()) {
            let mut errors = Vec::new();

            let pattern = expected_msg.value.unwrap();

            let value = lang_bundle.format_pattern(pattern, Some(args), &mut errors);

            handle_translation_error(&errors, string_key, false);

            value
        } else {
            // If we can't find the key in the expected language, fallback to English
            let fallback_bundle = self.0.get(&DEFAULT_LANG).unwrap();

            if let Some(fallback_msg) = fallback_bundle.get_message(string_key.as_str()) {
                let mut errors = Vec::new();

                let pattern = fallback_msg.value.unwrap();

                let value = lang_bundle.format_pattern(pattern, Some(args), &mut errors);

                handle_translation_error(&errors, string_key, true);

                value
            } else {
                // Something really went wrong, error in chat and the logs
                gearbot_warn!("{}", FAILED_TRANSLATE_FALLBACK_MSG);
                Cow::Borrowed(FAILED_TRANSLATE_FALLBACK_MSG)
            }
        }
    }

    pub fn get_translator(&self, lang: &LanguageIdentifier) -> Arc<FluentBundle<FluentResource>> {
        Arc::clone(self.0.get(lang).unwrap())
    }
}

fn handle_translation_error(errors: &[FluentError], key: GearBotString, is_fallback: bool) {
    for error in errors {
        if is_fallback {
            gearbot_warn!(
                "A translation error occured and had to fallback to '{}' while trying to translate the **``{}``** key: ``{:?}``",
                key.as_str(),
                DEFAULT_LANG,
                error
            );
        } else {
            gearbot_warn!(
                "A translation error occured while trying to translate the **``{}``** key: ``{:?}``",
                key.as_str(),
                error
            );
        }
    }
}

// This allows us to take full advantage of the type system to make sure that a key always exists in an
// ergonomic way instead of checking a bunch of options.
/// This is where *all* of the different things Gearbot can say should go.
pub enum GearBotString {
    // Basic commands
    PingPong,
    CoinflipDefault,
    CoinflipYes,
    CoinflipNo,
    UserinfoHeader,
    UserinfoNoRoles,
    AboutDescription,
    QuoteNotFound,

    EmojiPageHeader,
    EmojiOverviewHeader,
    EmojiInfo,

    //General logs (Text)
    CommandUsedText,

    //General logs (embed)
    CommandUsedEmbed,
    CommandUsedFooter,

    //Errors
    MissingPermissions,

    //DM error strings
    UnableToReply,
    UnableToReplyForManager,
}

impl GearBotString {
    fn as_str(&self) -> &'static str {
        match self {
            GearBotString::PingPong => "basic__ping_pong",
            GearBotString::CoinflipDefault => "bacic__coinflip_default_input",
            GearBotString::CoinflipYes => "basic__coinflip_yes",
            GearBotString::CoinflipNo => "basic__coinflip_no",
            GearBotString::UserinfoHeader => "basic__userinfo_header",
            GearBotString::UnableToReply => "errors_unable_to_reply",
            GearBotString::UnableToReplyForManager => "errors_unable_to_reply_manager",
            GearBotString::AboutDescription => "basic__about",
            GearBotString::QuoteNotFound => "basic__quote_notfound",
            GearBotString::MissingPermissions => "errors_missing_permissions",
            GearBotString::UserinfoNoRoles => "basic__userinfo_no_roles",
            GearBotString::EmojiPageHeader => "basic__emoji_page_header",
            GearBotString::EmojiOverviewHeader => "basic__emoji_overview_header",
            GearBotString::EmojiInfo => "basic__emoji_info",
            GearBotString::CommandUsedText => "command_used_text",
            GearBotString::CommandUsedEmbed => "command_used_embed",
            GearBotString::CommandUsedFooter => "command_used_footer",
        }
    }

    // TODO: Have a verification function here that makes sure you passed the right number of arguments in
    // Note that this would require a bit more work with maintaining strings as you would have to keep the mapping
    // data up to date, but it probably isn't that bad for the nice compile time saftey it gives us.
}

pub fn load_translations() -> Translations {
    let translation_files = fs::read_dir(TRANSLATION_DIR).expect("The translation directory was not found!");

    let mut translations = HashMap::new();

    for lang_dir in translation_files {
        let lang_dir = lang_dir.unwrap();

        if !lang_dir.file_type().unwrap().is_dir() {
            panic!("Each language must be contained in its own directory!")
        }

        let lang_dir_path = lang_dir.path();

        let lang_dir_name = lang_dir_path.file_stem().unwrap().to_str().unwrap();

        let langid: LanguageIdentifier = lang_dir_name
            .parse()
            .unwrap_or_else(|_| panic!("{} was not a valid language identifier!", lang_dir_name));

        // Make the bundle of the specific language
        let mut bundle = FluentBundle::new(&[langid.clone()]);
        bundle.set_use_isolating(false);
        for t_file in fs::read_dir(lang_dir.path()).unwrap() {
            let t_file = {
                let tmp = t_file.unwrap();
                fs::File::open(tmp.path()).expect("Failed to read a translation file in!")
            };

            let translation_data: HashMap<String, String> = serde_json::from_reader(&t_file).unwrap();

            // Then we add all the actual translations for said language
            for (translation_key, translation_string) in translation_data {
                let tl_string = format!("{} = {}", translation_key, translation_string);
                let res = FluentResource::try_new(tl_string).unwrap();

                bundle.add_resource(res).unwrap();
            }
        }

        translations.insert(langid, Arc::new(bundle));
    }

    Translations(translations)
}

#[cfg(test)]
mod tests {
    use super::{GearBotString, TRANSLATION_DIR};
    use lazy_static::lazy_static;
    use serde_json;
    use std::collections::HashMap;
    use std::fs;

    lazy_static! {
        static ref ALL_TRANSLATION_STR_KEYS: [&'static str; 17] = [
            GearBotString::PingPong.as_str(),
            GearBotString::CoinflipDefault.as_str(),
            GearBotString::CoinflipYes.as_str(),
            GearBotString::CoinflipNo.as_str(),
            GearBotString::UserinfoHeader.as_str(),
            GearBotString::UnableToReply.as_str(),
            GearBotString::UnableToReplyForManager.as_str(),
            GearBotString::AboutDescription.as_str(),
            GearBotString::QuoteNotFound.as_str(),
            GearBotString::MissingPermissions.as_str(),
            GearBotString::UserinfoNoRoles.as_str(),
            GearBotString::EmojiPageHeader.as_str(),
            GearBotString::EmojiOverviewHeader.as_str(),
            GearBotString::EmojiInfo.as_str(),
            GearBotString::CommandUsedText.as_str(),
            GearBotString::CommandUsedEmbed.as_str(),
            GearBotString::CommandUsedFooter.as_str(),
        ];
    }

    fn load_translations(lang: &str) -> HashMap<String, String> {
        let mut t_data = HashMap::new();
        let path = format!("{}/{}", TRANSLATION_DIR, lang);
        for t_file in fs::read_dir(path).unwrap() {
            let t_file = {
                let tmp = t_file.unwrap();
                fs::File::open(tmp.path()).expect("Failed to read a translation file in!")
            };

            let t_part: HashMap<String, String> = serde_json::from_reader(&t_file).unwrap();
            // We should always have the same number of keys

            t_data.extend(t_part)
        }

        assert_eq!(t_data.len(), ALL_TRANSLATION_STR_KEYS.len());
        t_data
    }

    #[test]
    fn enum_variants_translation_coverage() {
        let translation_data = load_translations("en_US");

        for t_var in ALL_TRANSLATION_STR_KEYS.iter() {
            assert!(translation_data.get(*t_var).is_some());
        }
    }

    #[test]
    fn translation_strings_are_used() {
        let translation_data = load_translations("en_US");

        let mut covered = 0;

        for (t_key, _) in translation_data {
            ALL_TRANSLATION_STR_KEYS.contains(&t_key.as_str());
            covered += 1;
        }

        // Make sure we exhausted everything
        assert_eq!(covered, ALL_TRANSLATION_STR_KEYS.len())
    }
}
