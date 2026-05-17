use core::fmt;
use std::{borrow::Cow, collections::HashMap, sync::OnceLock};

pub static I18N: OnceLock<I18n> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Ru, En, Es, De, Fr, Zh, Ja, Ko, It, Tr, Ar,
}

impl Language {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Language::Ar => "ar",
            Language::De => "de",
            Language::En => "en",
            Language::Es => "es",
            Language::Fr => "fr",
            Language::It => "it",
            Language::Ja => "ja",
            Language::Ko => "ko",
            Language::Ru => "ru",
            Language::Tr => "tr",
            Language::Zh => "zh",
        }
    }
}

impl AsRef<str> for Language {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct I18n {
    /// Parameter -> Language -> Text
    items: HashMap<&'static str, HashMap<Language, &'static str>>,
}

impl I18n {
    pub fn new() -> Self {
        Self {
            items: HashMap::new()
        }
    }

    pub fn add_item(&mut self, key: &'static str, lang: Language, text: &'static str) {
        self.items
            .entry(key)
            .or_default()
            .insert(lang, text);
    }

    pub fn translate(&self, key: &str, lang: Language) -> Cow<'static, str> {
        self.items.get(key)
            .and_then(|translations| {
                translations.get(&lang).or_else(|| translations.get(&Language::En))
            })
            .map(|&text| Cow::Borrowed(text))
            .unwrap_or_else(|| Cow::Owned(key.to_string()))
    }
}