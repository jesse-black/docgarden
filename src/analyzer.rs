use std::collections::HashSet;
use std::sync::OnceLock;

use rust_stemmers::{Algorithm, Stemmer};

static STOPWORDS: OnceLock<HashSet<&'static str>> = OnceLock::new();
static ENGLISH_STEMMER: OnceLock<Stemmer> = OnceLock::new();
static COMPOUND_DICTIONARY: &[(&str, &[&str])] = &[("execplan", &["exec", "plan"])];

pub(crate) fn is_stopword(term: &str) -> bool {
    STOPWORDS
        .get_or_init(|| include_str!("data/stopwords_en.txt").lines().collect())
        .contains(term)
}

fn english_stemmer() -> &'static Stemmer {
    ENGLISH_STEMMER.get_or_init(|| Stemmer::create(Algorithm::English))
}

pub(crate) fn analyze_token(token: &str) -> Vec<String> {
    let normalized = strip_possessive(&token.to_lowercase());
    if normalized.is_empty() || is_stopword(&normalized) {
        return Vec::new();
    }

    if let Some((_, expansion)) = COMPOUND_DICTIONARY
        .iter()
        .find(|(compound, _)| *compound == normalized)
    {
        return expansion.iter().map(|token| stem_token(token)).collect();
    }

    vec![stem_token(&normalized)]
}

pub(crate) fn normalize_text(text: &str) -> Vec<String> {
    tokenize(text)
}

pub(crate) fn normalize_path(path: &str) -> Vec<String> {
    let without_ext = path.strip_suffix(".md").unwrap_or(path);
    tokenize(without_ext)
}

fn tokenize(input: &str) -> Vec<String> {
    input
        .split(is_separator)
        .flat_map(split_camel_case)
        .flat_map(analyze_token)
        .collect()
}

pub(crate) fn is_separator(ch: char) -> bool {
    ch.is_whitespace() || (ch.is_ascii_punctuation() && ch != '\'')
}

pub(crate) fn split_camel_case(input: &str) -> Vec<&str> {
    if input.is_empty() {
        return Vec::new();
    }

    let mut start = 0;
    let mut parts = Vec::new();
    let chars: Vec<(usize, char)> = input.char_indices().collect();

    for i in 1..chars.len() {
        if is_camel_boundary(&chars, i) {
            let end = chars[i].0;
            parts.push(&input[start..end]);
            start = end;
        }
    }

    parts.push(&input[start..]);
    parts
}

fn is_camel_boundary(chars: &[(usize, char)], index: usize) -> bool {
    let previous = chars[index - 1].1;
    let current = chars[index].1;
    let next = chars.get(index + 1).map(|(_, ch)| *ch);

    current.is_ascii_uppercase()
        && (previous.is_ascii_lowercase()
            || (previous.is_ascii_uppercase() && next.is_some_and(|ch| ch.is_ascii_lowercase())))
}

fn strip_possessive(token: &str) -> String {
    if let Some(stripped) = token.strip_suffix("'s") {
        stripped.to_string()
    } else if let Some(stripped) = token.strip_suffix('\'') {
        if stripped.ends_with('s') {
            if stripped == "s" {
                return String::new();
            }
            return stripped.to_string();
        }
        token.to_string()
    } else {
        token.to_string()
    }
}

fn stem_token(token: &str) -> String {
    english_stemmer().stem(token).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_text_stems_morphological_variants() {
        assert_eq!(
            normalize_text("plans reviews analyzed"),
            vec!["plan", "review", "analyz"]
        );
    }

    #[test]
    fn normalize_path_stems_path_segments() {
        assert_eq!(
            normalize_path("docs/the-active-plans/scoring-guide.md"),
            vec!["doc", "activ", "plan", "score", "guid"]
        );
    }

    #[test]
    fn analyze_token_filters_empty_and_stopword_input() {
        assert_eq!(analyze_token(""), Vec::<String>::new());
        assert_eq!(analyze_token("the"), Vec::<String>::new());
        assert_eq!(analyze_token("is"), Vec::<String>::new());
    }

    #[test]
    fn analyze_token_filters_stopwords_before_stemming() {
        assert_eq!(analyze_token("was"), Vec::<String>::new());
    }

    #[test]
    fn analyze_token_lowercases_and_stems_mixed_case_input() {
        assert_eq!(analyze_token("Reviews"), vec!["review"]);
    }

    #[test]
    fn normalize_text_preserves_token_ordering_and_empty_input() {
        assert_eq!(normalize_text("Reviews plans"), vec!["review", "plan"]);
        assert!(normalize_text("").is_empty());
    }

    #[test]
    fn normalize_text_lowercases_splits_and_filters_stopwords() {
        let toks = normalize_text("Hello, The World!");
        assert_eq!(toks, vec!["hello", "world"]);
    }

    #[test]
    fn normalize_path_strips_extension_and_splits_separators() {
        let toks = normalize_path("docs/the-active-plan/my_guide.md");
        assert_eq!(toks, vec!["doc", "activ", "plan", "my", "guid"]);
    }

    #[test]
    fn apostrophes_are_not_separators_for_text_or_paths() {
        assert_eq!(normalize_text("you're").len(), 1);
        assert_eq!(normalize_path("you're.md").len(), 1);
        assert_eq!(normalize_path("docs/you're.md"), vec!["doc", "you'r"]);
    }

    #[test]
    fn normalize_text_splits_camel_case_boundaries() {
        assert_eq!(normalize_text("ExecPlan"), vec!["exec", "plan"]);
        assert_eq!(normalize_text("XMLParser"), vec!["xml", "parser"]);
        assert_eq!(normalize_text("PowerShot"), vec!["power", "shot"]);
    }

    #[test]
    fn normalize_text_preserves_digit_identifier_shapes() {
        for token in ["BM25F", "v1", "f32", "R2D2", "SD500", "ADR0004"] {
            assert_eq!(
                normalize_text(token),
                vec![stem_token(&token.to_lowercase())],
                "{token} should stay one analyzed token"
            );
        }
    }

    #[test]
    fn analyze_token_strips_trailing_possessives() {
        assert_eq!(analyze_token("Jim's"), vec!["jim"]);
        assert_eq!(normalize_text("Jim's notebook"), vec!["jim", "notebook"]);
        assert_eq!(analyze_token("O'Reilly's"), analyze_token("O'Reilly"));
        assert_eq!(analyze_token("dogs'"), analyze_token("dog"));
        assert_eq!(
            normalize_text("dogs' bowls")[0],
            normalize_text("dog bowls")[0]
        );
        assert_eq!(analyze_token("'s"), Vec::<String>::new());
        assert_eq!(analyze_token("s'"), Vec::<String>::new());
    }

    #[test]
    fn analyze_token_preserves_internal_apostrophes() {
        assert_eq!(analyze_token("you're"), vec!["you'r"]);
        assert_eq!(analyze_token("O'Reilly").len(), 1);
        assert_eq!(normalize_text("O'Reilly's"), normalize_text("O'Reilly"));
    }

    #[test]
    fn compound_dictionary_expands_curated_entries() {
        assert_eq!(analyze_token("execplan"), vec!["exec", "plan"]);
        assert_eq!(normalize_text("execplan"), vec!["exec", "plan"]);
        let path_terms = normalize_path("docs/planner-execplan.md");
        assert!(path_terms.contains(&"exec".to_string()));
        assert!(path_terms.contains(&"plan".to_string()));
    }
}
