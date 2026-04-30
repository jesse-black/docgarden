use std::collections::HashSet;
use std::sync::OnceLock;

use rust_stemmers::{Algorithm, Stemmer};

static STOPWORDS: OnceLock<HashSet<&'static str>> = OnceLock::new();
static ENGLISH_STEMMER: OnceLock<Stemmer> = OnceLock::new();
static COMPOUND_DICTIONARY: &[(&str, &[&str])] = &[("execplan", &["exec", "plan"])];

pub(crate) struct AnalyzedSpan<'a> {
    pub(crate) surface: &'a str,
    pub(crate) terms: Vec<String>,
}

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
    vec![stem_token(&normalized)]
}

pub(crate) fn normalize_text(text: &str) -> Vec<String> {
    tokenize(text)
}

pub(crate) fn normalize_path(path: &str) -> Vec<String> {
    let without_ext = path.strip_suffix(".md").unwrap_or(path);
    tokenize(without_ext)
}

pub(crate) fn analyze_surface_spans(token: &str) -> Vec<AnalyzedSpan<'_>> {
    let (base, suffix) = split_possessive_surface(token);
    let mut spans: Vec<AnalyzedSpan<'_>> = split_camel_case(base)
        .into_iter()
        .filter(|surface| !surface.is_empty())
        .flat_map(analyze_surface_part)
        .collect();

    if let Some(surface) = suffix {
        spans.push(AnalyzedSpan {
            surface,
            terms: Vec::new(),
        });
    }

    spans
}

fn analyze_surface_part(surface: &str) -> Vec<AnalyzedSpan<'_>> {
    if let Some(spans) = split_compound_surface(surface) {
        return spans;
    }

    vec![AnalyzedSpan {
        surface,
        terms: analyze_token(surface),
    }]
}

fn tokenize(input: &str) -> Vec<String> {
    input
        .split(is_separator)
        .flat_map(analyze_surface_spans)
        .flat_map(|span| span.terms)
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

fn possessive_suffix_len(lower: &str) -> Option<usize> {
    if lower.ends_with("'s") || lower.ends_with("s'") {
        Some(2)
    } else {
        None
    }
}

fn strip_possessive(token: &str) -> String {
    match possessive_suffix_len(token) {
        Some(len) => token[..token.len() - len].to_string(),
        None => token.to_string(),
    }
}

fn split_possessive_surface(token: &str) -> (&str, Option<&str>) {
    let lower = token.to_lowercase();
    match possessive_suffix_len(&lower) {
        Some(len) => {
            let split = token.len() - len;
            (&token[..split], Some(&token[split..]))
        }
        None => (token, None),
    }
}

fn split_compound_surface(token: &str) -> Option<Vec<AnalyzedSpan<'_>>> {
    let lower = token.to_lowercase();
    let (_, expansion) = COMPOUND_DICTIONARY
        .iter()
        .find(|(compound, _)| *compound == lower)?;

    let mut start = 0;
    let mut spans = Vec::new();
    for part in *expansion {
        let end = start + part.len();
        let surface = token.get(start..end)?;
        spans.push(AnalyzedSpan {
            surface,
            terms: vec![stem_token(part)],
        });
        start = end;
    }

    (start == token.len()).then_some(spans)
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
        let cases = [
            ("BM25F", "bm25f"),
            ("v1", "v1"),
            ("f32", "f32"),
            ("R2D2", "r2d2"),
            ("SD500", "sd500"),
            ("ADR0004", "adr0004"),
        ];
        for (token, expected_stem) in cases {
            let result = normalize_text(token);
            assert_eq!(result.len(), 1, "{token} should stay one analyzed token");
            assert_eq!(result, vec![expected_stem], "{token} stem mismatch");
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
        assert_eq!(analyze_token("O'Reilly"), vec!["o'reilli"]);
        assert_eq!(normalize_text("O'Reilly's"), normalize_text("O'Reilly"));
    }

    #[test]
    fn compound_dictionary_expands_curated_entries() {
        assert_eq!(normalize_text("execplan"), vec!["exec", "plan"]);
        let path_terms = normalize_path("docs/planner-execplan.md");
        assert!(path_terms.contains(&"exec".to_string()));
        assert!(path_terms.contains(&"plan".to_string()));
    }

    #[test]
    fn analyze_surface_spans_splits_camel_case_with_terms() {
        let spans = analyze_surface_spans("ExecPlan");
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].surface, "Exec");
        assert_eq!(spans[0].terms, vec!["exec"]);
        assert_eq!(spans[1].surface, "Plan");
        assert_eq!(spans[1].terms, vec!["plan"]);
    }

    #[test]
    fn analyze_surface_spans_keeps_possessive_suffix_termless() {
        let spans = analyze_surface_spans("Jim's");
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].surface, "Jim");
        assert_eq!(spans[0].terms, vec!["jim"]);
        assert_eq!(spans[1].surface, "'s");
        assert!(spans[1].terms.is_empty());
    }

    #[test]
    fn analyze_surface_spans_splits_compound_surface_boundaries() {
        let spans = analyze_surface_spans("execplan");
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].surface, "exec");
        assert_eq!(spans[0].terms, vec!["exec"]);
        assert_eq!(spans[1].surface, "plan");
        assert_eq!(spans[1].terms, vec!["plan"]);
    }

    #[test]
    fn analyze_surface_spans_emits_no_terms_for_stopword_and_suffix() {
        let spans = analyze_surface_spans("the's");
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].surface, "the");
        assert!(spans[0].terms.is_empty());
        assert_eq!(spans[1].surface, "'s");
        assert!(spans[1].terms.is_empty());

        let bare_plural = analyze_surface_spans("s'");
        assert_eq!(bare_plural.len(), 1);
        assert_eq!(bare_plural[0].surface, "s'");
        assert!(bare_plural[0].terms.is_empty());
        assert!(normalize_text("s'").is_empty());
    }
}
