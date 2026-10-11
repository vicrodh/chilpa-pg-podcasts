//! The few words the plugin says itself: its columns, its labels, and the
//! languages and categories it offers.
//!
//! A plugin has no access to Chilpa's catalog, so it carries its own, and
//! Chilpa tells it which language the window speaks.

pub struct Words {
    pub search: &'static str,
    pub language: &'static str,
    pub category: &'static str,
    pub categories: &'static str,
    pub episodes: &'static str,
    pub website: &'static str,
    pub directory: &'static str,
    pub season: &'static str,
    /// The window's own language, as a code, which is what the language
    /// column starts on.
    pub own: &'static str,
    /// The languages offered, by the code the index uses.
    pub languages: &'static [(&'static str, &'static str)],
    /// The categories offered, by the name the index uses.
    pub topics: &'static [(&'static str, &'static str)],
}

const ENGLISH: Words = Words {
    search: "Search shows",
    language: "Language",
    category: "Category",
    categories: "Categories",
    episodes: "Episodes",
    website: "Website",
    directory: "Listed in",
    season: "Season",
    own: "en",
    languages: &[
        ("de", "German"),
        ("ca", "Catalan"),
        ("zh", "Chinese"),
        ("nl", "Dutch"),
        ("en", "English"),
        ("fr", "French"),
        ("it", "Italian"),
        ("ja", "Japanese"),
        ("ko", "Korean"),
        ("pt", "Portuguese"),
        ("ru", "Russian"),
        ("es", "Spanish"),
        ("sv", "Swedish"),
    ],
    topics: &[
        ("Arts", "Arts"),
        ("Business", "Business"),
        ("Comedy", "Comedy"),
        ("Culture", "Culture"),
        ("Documentary", "Documentary"),
        ("Education", "Education"),
        ("Fiction", "Fiction"),
        ("Film", "Film"),
        ("Food", "Food"),
        ("Games", "Games"),
        ("Health", "Health"),
        ("History", "History"),
        ("Kids", "Kids"),
        ("Language", "Learning languages"),
        ("Music", "Music"),
        ("News", "News"),
        ("Politics", "Politics"),
        ("Religion", "Religion"),
        ("Science", "Science"),
        ("Society", "Society"),
        ("Sports", "Sports"),
        ("Technology", "Technology"),
        ("Travel", "Travel"),
        ("True Crime", "True crime"),
        ("TV", "TV"),
    ],
};

const SPANISH: Words = Words {
    search: "Buscar programas",
    language: "Idioma",
    category: "Categoría",
    categories: "Categorías",
    episodes: "Episodios",
    website: "Sitio",
    directory: "Listado en",
    season: "Temporada",
    own: "es",
    languages: &[
        ("de", "Alemán"),
        ("ca", "Catalán"),
        ("zh", "Chino"),
        ("ko", "Coreano"),
        ("es", "Español"),
        ("fr", "Francés"),
        ("en", "Inglés"),
        ("it", "Italiano"),
        ("ja", "Japonés"),
        ("nl", "Neerlandés"),
        ("pt", "Portugués"),
        ("ru", "Ruso"),
        ("sv", "Sueco"),
    ],
    topics: &[
        ("Arts", "Arte"),
        ("Film", "Cine"),
        ("Science", "Ciencia"),
        ("Comedy", "Comedia"),
        ("Culture", "Cultura"),
        ("Sports", "Deportes"),
        ("Documentary", "Documental"),
        ("Education", "Educación"),
        ("Fiction", "Ficción"),
        ("Food", "Gastronomía"),
        ("History", "Historia"),
        ("Language", "Idiomas"),
        ("Games", "Juegos"),
        ("Music", "Música"),
        ("Kids", "Niños"),
        ("Business", "Negocios"),
        ("News", "Noticias"),
        ("Politics", "Política"),
        ("Religion", "Religión"),
        ("Health", "Salud"),
        ("Society", "Sociedad"),
        ("Technology", "Tecnología"),
        ("True Crime", "True crime"),
        ("TV", "TV"),
        ("Travel", "Viajes"),
    ],
};

impl Words {
    /// The words for a language code. Anything unknown gets English.
    pub fn of(language: &str) -> &'static Words {
        match language.get(..2).map(str::to_ascii_lowercase).as_deref() {
            Some("es") => &SPANISH,
            _ => &ENGLISH,
        }
    }

    /// True if that is one of the languages offered.
    pub fn offers_language(&self, code: &str) -> bool {
        self.languages.iter().any(|(offered, _)| *offered == code)
    }

    /// True if that is one of the categories offered.
    pub fn offers_topic(&self, name: &str) -> bool {
        self.topics.iter().any(|(offered, _)| *offered == name)
    }

    /// The name of a language in the window's language, or its code.
    pub fn language_name(&self, code: &str) -> String {
        let short = code.get(..2).unwrap_or(code).to_ascii_lowercase();
        self.languages.iter().find(|(c, _)| *c == short).map(|(_, n)| n.to_string()).unwrap_or_else(|| code.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_languages_offer_the_same_codes_and_categories() {
        let sorted = |items: &[(&'static str, &'static str)]| {
            let mut keys: Vec<&str> = items.iter().map(|i| i.0).collect();
            keys.sort();
            keys
        };
        assert_eq!(sorted(ENGLISH.languages), sorted(SPANISH.languages));
        assert_eq!(sorted(ENGLISH.topics), sorted(SPANISH.topics));
        assert!(Words::of("es-MX").offers_language(Words::of("es-MX").own));
        assert!(Words::of("en").offers_language(Words::of("en").own));
        assert_eq!(Words::of("es").language_name("pt-BR"), "Portugués");
        assert_eq!(Words::of("en").language_name("xx"), "xx");
    }
}
