use std::borrow::Cow;
use std::collections::HashMap;
use std::fs;

use fluent_bundle::{concurrent::FluentBundle, FluentArgs, FluentResource, FluentValue};
use serde_json;
use unic_langid::{langid, LanguageIdentifier};

use crate::gearbot_warn;

const TRANSLATION_DIR: &str = "./lang";

/// The default language to fall back to if a string can't be translated in the requested language.
/// This is also the language that new guild configs will default to.
pub const DEFAULT_LANG: LanguageIdentifier = langid!("en_US");

pub struct Translations {
    translations: HashMap<LanguageIdentifier, FluentBundle<FluentResource>>,
}

impl Translations {
    /// Generates the arguments needed for getting a text string that takes arguments. The advised type to pass in here
    /// (for resource efficiency) is a &[(key, &dynamic_value_string_ref)] since the output only borrows the input.
    pub fn generate_args<'a, P: 'a, T>(&self, arg_mappings: T) -> FluentArgs<'a>
    where
        &'a P: Into<FluentValue<'a>>,
        T: IntoIterator<Item = &'a (&'a str, &'a P)>,
    {
        let mappings = arg_mappings.into_iter();

        // Try to be smart with our allocations
        let mut args = FluentArgs::with_capacity(mappings.size_hint().1.unwrap_or_default());

        for (arg_key, arg_inserted_value) in mappings {
            let f_value = (*arg_inserted_value).into();

            args.insert(arg_key, f_value);
        }

        args
    }

    /// Retreives a string key to use when sending a message to chat that *does not* require arguments and can be sent as fetched with no
    /// further modifications.
    pub fn get_text_plain<'a>(
        &'a self,
        lang_key: &LanguageIdentifier,
        string_key: GearBotStrings,
    ) -> Cow<'a, str> {
        // TODO: See how well this will work out in practice with unwrapping
        let lang_bundle = self.translations.get(lang_key).unwrap();

        if let Some(expected_msg) = lang_bundle.get_message(string_key.as_str()) {
            let mut errors = Vec::new();

            let pattern = expected_msg.value.unwrap();

            let value = lang_bundle.format_pattern(pattern, None, &mut errors);

            for _error in errors {
                gearbot_warn!(
                    "A translation error occured: TODO: Library doesn't give error descriptions!"
                );
            }

            value
        } else {
            // If we can't find the key in the expected language, fallback to English
            let fallback_bundle = self.translations.get(&DEFAULT_LANG).unwrap();

            if let Some(fallback_msg) = fallback_bundle.get_message(string_key.as_str()) {
                let mut errors = Vec::new();

                let pattern = fallback_msg.value.unwrap();

                let value = lang_bundle.format_pattern(pattern, None, &mut errors);

                for _error in errors {
                    gearbot_warn!("A translation error occured: TODO: Library doesn't give error descriptions!");
                }

                value
            } else {
                // Something really went wrong, error in chat
                Cow::Borrowed("A translation error occured and no fallback could be found!")
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
        string_key: GearBotStrings,
        args: &'a FluentArgs<'a>,
    ) -> Cow<'a, str> {
        let lang_bundle = self.translations.get(lang_key).unwrap();

        if let Some(expected_msg) = lang_bundle.get_message(string_key.as_str()) {
            let mut errors = Vec::new();

            let pattern = expected_msg.value.unwrap();

            let value = lang_bundle.format_pattern(pattern, Some(args), &mut errors);

            for _error in errors {
                gearbot_warn!(
                    "A translation error occured: TODO: Library doesn't give error descriptions!"
                );
            }

            value
        } else {
            // If we can't find the key in the expected language, fallback to English
            let fallback_bundle = self.translations.get(&DEFAULT_LANG).unwrap();

            if let Some(fallback_msg) = fallback_bundle.get_message(string_key.as_str()) {
                let mut errors = Vec::new();

                let pattern = fallback_msg.value.unwrap();

                let value = lang_bundle.format_pattern(pattern, Some(args), &mut errors);

                for _error in errors {
                    gearbot_warn!("A translation error occured: TODO: Library doesn't give error descriptions!");
                }

                value
            } else {
                // Something went really wrong, error in chat
                Cow::Borrowed("A translation error occured and no fallback could be found! Something may be wrong with the guild configuration!")
            }
        }
    }
}

// This allows us to take full advantage of the type system to make sure that a key always exists in an
// ergonomic way instead of checking a bunch of options.
/// This is where *all* of the different things Gearbot can say should go.
pub enum GearBotStrings {
    PingPong,
}

impl GearBotStrings {
    fn as_str(&self) -> &str {
        match self {
            GearBotStrings::PingPong => "basic__ping_pong",
        }
    }

    // TODO: Have a verification function here that makes sure you passed the right number of arguments in
    // Note that this would require a bit more work with maintaining strings as you would have to keep the mapping
    // data up to date, but it probably isn't that bad for the nice compile time saftey it gives us.
}

pub fn load_translations() -> Translations {
    let translation_files =
        fs::read_dir(TRANSLATION_DIR).expect("The translation directory was not found!");

    let mut translations = HashMap::new();

    for lang_dir in translation_files {
        let lang_dir = lang_dir.unwrap();

        if !lang_dir.file_type().unwrap().is_dir() {
            panic!("Each language must be contained in its own directory!")
        }

        let lang_dir_path = lang_dir.path();

        let lang_dir_name = lang_dir_path.file_stem().unwrap().to_str().unwrap();

        let langid: LanguageIdentifier = lang_dir_name.parse().expect(&format!(
            "{} was not a valid language identifier!",
            lang_dir_name
        ));

        // Make the bundle of the specific language
        let mut bundle = FluentBundle::new(&[langid.clone()]);

        for t_file in fs::read_dir(lang_dir.path()).unwrap() {
            let t_file = {
                let tmp = t_file.unwrap();
                fs::File::open(tmp.path()).expect("Failed to read a translation file in!")
            };

            let translation_data: HashMap<String, String> =
                serde_json::from_reader(&t_file).unwrap();

            // Then we add all the actual translations for said language
            for (translation_key, translation_string) in translation_data {
                let tl_string = format!("{} = {}", translation_key, translation_string);

                println!("Adding a resource string of: {}", tl_string);

                let res = FluentResource::try_new(tl_string).unwrap();

                bundle.add_resource(res).unwrap();
            }
        }

        translations.insert(langid, bundle);
    }

    Translations { translations }
}
