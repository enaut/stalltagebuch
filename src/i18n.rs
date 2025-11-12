use dioxus_i18n::prelude::*;

/// Initialize i18n configuration with German as default language
pub fn init_i18n() -> I18nConfig {
    I18nConfig::new(unic_langid::langid!("de-DE")).with_locale(Locale::new_static(
        unic_langid::langid!("de-DE"),
        include_str!("../locales/de-DE.ftl"),
    ))
}
