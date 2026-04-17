use std::collections::{HashMap, HashSet};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

pub(crate) struct Candidate<'a> {
    pub(crate) name: Option<&'a str>,
    pub(crate) description: Option<&'a str>,
    pub(crate) repo_relative_path: &'a str,
}

/// Corpus-local IDF table built once per invocation from all discovered files.
///
/// `idf(t) = clamp(ln((N+1)/(df(t)+1)) + 1, 0.5, 1.8)` per term.
/// Unknown tokens get weight 1.0 (neutral).
pub(crate) struct IdfTable {
    weights: HashMap<String, f32>,
}

impl IdfTable {
    pub(crate) fn build(candidates: &[Candidate<'_>]) -> Self {
        let n = candidates.len() as f32;
        let mut df: HashMap<String, u32> = HashMap::new();

        for candidate in candidates {
            let mut seen: HashSet<&str> = HashSet::new();

            let name_toks = candidate.name.map(normalize_text).unwrap_or_default();
            let desc_toks = candidate
                .description
                .map(normalize_text)
                .unwrap_or_default();
            let path_toks = normalize_path(candidate.repo_relative_path);

            for tok in name_toks
                .iter()
                .chain(desc_toks.iter())
                .chain(path_toks.iter())
            {
                seen.insert(tok.as_str());
            }
            // We borrowed from owned Vecs above; collect unique strings.
            let seen_owned: Vec<String> = seen.iter().map(|s| s.to_string()).collect();
            for tok in seen_owned {
                *df.entry(tok).or_default() += 1;
            }
        }

        let weights = df
            .into_iter()
            .map(|(tok, df_count)| {
                let raw = ((n + 1.0) / (df_count as f32 + 1.0)).ln() + 1.0;
                (tok, raw.clamp(0.5, 1.8))
            })
            .collect();

        Self { weights }
    }

    pub(crate) fn weight(&self, token: &str) -> f32 {
        *self.weights.get(token).unwrap_or(&1.0)
    }
}

/// Which scoring field produced the first (highest-priority) match.
/// Ordering: Name < Path < Description — lower enum value = higher priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Field {
    Name,
    Path,
    Description,
}

pub(crate) struct ScoredHit {
    pub(crate) score: i32,
    pub(crate) matched_terms: u32,
    /// The highest-priority field that contained at least one matched term.
    pub(crate) first_field_hit: Option<Field>,
}

// ---------------------------------------------------------------------------
// Normalization helpers (pub(crate) for reuse by matching.rs and tests)
// ---------------------------------------------------------------------------

/// Lowercase and split text on whitespace and ASCII punctuation.
pub(crate) fn normalize_text(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// Split a repository-relative path into tokens.
///
/// Strips the `.md` extension, lowercases, and splits on `/`, `_`, `-`, `.`,
/// whitespace, and ASCII punctuation.
pub(crate) fn normalize_path(path: &str) -> Vec<String> {
    let lower = path.to_lowercase();
    let without_ext = lower.strip_suffix(".md").unwrap_or(&lower);
    without_ext
        .split(|c: char| {
            matches!(c, '/' | '_' | '-' | '.') || c.is_whitespace() || c.is_ascii_punctuation()
        })
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

// ---------------------------------------------------------------------------
// Scoring
// ---------------------------------------------------------------------------

/// Return the tier score for a single query term against a single field.
///
/// Tiers (descending): exact token (10), prefix (4), substring (1), none (0).
/// For each (query-term, field) pair we take the best tier; tiers are not summed.
fn match_tier(query_term: &str, field_tokens: &[String], field_normalized: &str) -> i32 {
    // Exact token match.
    if field_tokens.iter().any(|t| t == query_term) {
        return 10;
    }
    // Prefix: any field token starts with the query term (min term length 2).
    if query_term.len() >= 2 && field_tokens.iter().any(|t| t.starts_with(query_term)) {
        return 4;
    }
    // Substring anywhere in the joined normalized field text.
    if field_normalized.contains(query_term) {
        return 1;
    }
    0
}

/// Score a single candidate against the tokenized query.
pub(crate) fn score(
    query_terms: &[String],
    candidate: &Candidate<'_>,
    idf: &IdfTable,
) -> ScoredHit {
    if query_terms.is_empty() {
        return ScoredHit {
            score: 0,
            matched_terms: 0,
            first_field_hit: None,
        };
    }

    // Pre-compute normalized fields.
    let name_toks = candidate.name.map(normalize_text).unwrap_or_default();
    let name_norm = name_toks.join(" ");
    let desc_toks = candidate
        .description
        .map(normalize_text)
        .unwrap_or_default();
    let desc_norm = desc_toks.join(" ");
    let path_toks = normalize_path(candidate.repo_relative_path);
    let path_norm = path_toks.join(" ");

    // Path basename normalized string for phrase-bonus check.
    let basename_raw = candidate
        .repo_relative_path
        .rsplit('/')
        .next()
        .unwrap_or(candidate.repo_relative_path);
    let basename_norm = normalize_path(basename_raw).join(" ");

    let mut total: f32 = 0.0;
    let mut matched_terms: u32 = 0;
    let mut first_field_hit: Option<Field> = None;

    for term in query_terms {
        let idf_w = idf.weight(term);
        let mut term_matched = false;

        // Name — field weight 3.
        let name_tier = match_tier(term, &name_toks, &name_norm);
        if name_tier > 0 {
            total += name_tier as f32 * 3.0 * idf_w;
            term_matched = true;
            first_field_hit = best_field_hit(first_field_hit, Field::Name);
        }

        // Path — field weight 2.
        let path_tier = match_tier(term, &path_toks, &path_norm);
        if path_tier > 0 {
            total += path_tier as f32 * 2.0 * idf_w;
            term_matched = true;
            first_field_hit = best_field_hit(first_field_hit, Field::Path);
        }

        // Description — field weight 1.
        let desc_tier = match_tier(term, &desc_toks, &desc_norm);
        if desc_tier > 0 {
            total += desc_tier as f32 * 1.0 * idf_w;
            term_matched = true;
            first_field_hit = best_field_hit(first_field_hit, Field::Description);
        }

        if term_matched {
            matched_terms += 1;
        }
    }

    // Phrase bonus — flat bump for contiguous literal match, no IDF.
    let query_phrase: String = query_terms.join(" ");
    let mut phrase_bonus: i32 = 0;
    if query_terms.len() > 1 && !query_phrase.is_empty() {
        if candidate.name.is_some() && name_norm.contains(query_phrase.as_str()) {
            phrase_bonus += 25;
        }
        if basename_norm.contains(query_phrase.as_str()) {
            phrase_bonus += 25;
        }
        if candidate.description.is_some() && desc_norm.contains(query_phrase.as_str()) {
            phrase_bonus += 10;
        }
    }

    ScoredHit {
        score: total.round() as i32 + phrase_bonus,
        matched_terms,
        first_field_hit,
    }
}

fn best_field_hit(current: Option<Field>, candidate: Field) -> Option<Field> {
    Some(match current {
        Some(existing) => existing.min(candidate),
        None => candidate,
    })
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn flat_idf() -> IdfTable {
        IdfTable {
            weights: HashMap::new(), // all weights default to 1.0
        }
    }

    fn candidate<'a>(
        name: Option<&'a str>,
        description: Option<&'a str>,
        path: &'a str,
    ) -> Candidate<'a> {
        Candidate {
            name,
            description,
            repo_relative_path: path,
        }
    }

    fn terms(q: &str) -> Vec<String> {
        normalize_text(q)
    }

    // --- Tier ordering ---

    #[test]
    fn exact_beats_prefix_beats_substring() {
        let idf = flat_idf();

        // Exact name match
        let exact = score(
            &terms("scoring"),
            &candidate(Some("Scoring System"), None, "docs/other.md"),
            &idf,
        );
        // Prefix name match ("scor" is a prefix of "scoring")
        let prefix = score(
            &terms("scor"),
            &candidate(Some("Scoring System"), None, "docs/other.md"),
            &idf,
        );
        // Substring description match only ("scoring" appears in description)
        let substr = score(
            &terms("scoring"),
            &candidate(None, Some("About scoring things"), "docs/other.md"),
            &idf,
        );

        assert!(
            exact.score > prefix.score,
            "exact ({}) > prefix ({})",
            exact.score,
            prefix.score
        );
        assert!(
            prefix.score > substr.score,
            "prefix ({}) > substr ({})",
            prefix.score,
            substr.score
        );
    }

    // --- Field weight priority ---

    #[test]
    fn name_match_beats_description_match() {
        let idf = flat_idf();

        let name_hit = score(
            &terms("discovery"),
            &candidate(Some("Discovery Guide"), None, "docs/other.md"),
            &idf,
        );
        let desc_hit = score(
            &terms("discovery"),
            &candidate(None, Some("About discovery things"), "docs/other.md"),
            &idf,
        );

        assert!(
            name_hit.score > desc_hit.score,
            "name ({}) > desc ({})",
            name_hit.score,
            desc_hit.score
        );
    }

    #[test]
    fn first_field_hit_reflects_highest_priority_field() {
        let idf = flat_idf();

        let hit = score(
            &terms("guide"),
            &candidate(
                Some("User Guide"),
                Some("A guide for users"),
                "docs/guide.md",
            ),
            &idf,
        );
        // Name matched first (it's the highest priority field)
        assert_eq!(hit.first_field_hit, Some(Field::Name));
    }

    #[test]
    fn first_field_hit_falls_through_to_path_when_no_name() {
        let idf = flat_idf();

        let hit = score(
            &terms("guide"),
            &candidate(None, None, "docs/guide.md"),
            &idf,
        );
        assert_eq!(hit.first_field_hit, Some(Field::Path));
    }

    #[test]
    fn first_field_hit_prefers_name_even_if_later_query_term_matches_it() {
        let idf = flat_idf();

        let hit = score(
            &terms("guide discovery"),
            &candidate(Some("Discovery"), None, "docs/guide.md"),
            &idf,
        );

        assert_eq!(hit.first_field_hit, Some(Field::Name));
    }

    #[test]
    fn first_field_hit_is_stable_across_query_term_order() {
        let idf = flat_idf();
        let candidate = candidate(Some("Discovery"), Some("Reference guide"), "docs/guide.md");

        let path_then_name = score(&terms("guide discovery"), &candidate, &idf);
        let name_then_path = score(&terms("discovery guide"), &candidate, &idf);

        assert_eq!(path_then_name.first_field_hit, Some(Field::Name));
        assert_eq!(name_then_path.first_field_hit, Some(Field::Name));
    }

    // --- Phrase bonus ---

    #[test]
    fn phrase_bonus_fires_for_contiguous_name_match() {
        let idf = flat_idf();

        let with_phrase = score(
            &terms("match subcommand"),
            &candidate(Some("Implement match subcommand"), None, "docs/other.md"),
            &idf,
        );
        let without_phrase = score(
            &terms("match subcommand"),
            &candidate(Some("Subcommand to implement match"), None, "docs/other.md"),
            &idf,
        );

        // Both have both terms, but first doc has contiguous phrase "match subcommand"
        assert!(
            with_phrase.score > without_phrase.score,
            "with phrase ({}) > without ({})",
            with_phrase.score,
            without_phrase.score
        );
    }

    #[test]
    fn phrase_bonus_does_not_fire_for_non_contiguous_query_in_name() {
        let idf = flat_idf();

        // "match subcommand" is not contiguous in "match the new subcommand"
        let hit = score(
            &terms("match subcommand"),
            &candidate(Some("Match the new subcommand"), None, "docs/other.md"),
            &idf,
        );
        // Score should only have per-term contributions, no phrase bonus
        let expected_no_phrase = score(
            &terms("subcommand"),
            &candidate(Some("Match the new subcommand"), None, "docs/other.md"),
            &idf,
        )
        .score
            + score(
                &terms("match"),
                &candidate(Some("Match the new subcommand"), None, "docs/other.md"),
                &idf,
            )
            .score;

        // hit.score should equal the sum of individual term scores (no phrase bonus)
        assert_eq!(hit.score, expected_no_phrase);
    }

    // --- Zero-score behavior ---

    #[test]
    fn zero_score_when_no_terms_match() {
        let idf = flat_idf();

        let hit = score(
            &terms("xyzzy"),
            &candidate(Some("Unrelated Document"), None, "docs/unrelated.md"),
            &idf,
        );
        assert_eq!(hit.score, 0);
        assert_eq!(hit.matched_terms, 0);
        assert_eq!(hit.first_field_hit, None);
    }

    #[test]
    fn zero_score_for_empty_query() {
        let idf = flat_idf();
        let hit = score(&[], &candidate(Some("Anything"), None, "docs/x.md"), &idf);
        assert_eq!(hit.score, 0);
    }

    // --- Matched term count ---

    #[test]
    fn matched_terms_counts_distinct_query_terms_that_hit() {
        let idf = flat_idf();

        let hit = score(
            &terms("discovery guide"),
            &candidate(
                Some("Discovery Guide"),
                Some("A guide for discovery"),
                "docs/x.md",
            ),
            &idf,
        );
        assert_eq!(hit.matched_terms, 2);
    }

    // --- IDF effects ---

    #[test]
    fn idf_boosts_rare_term_over_ubiquitous_term() {
        // Build a corpus where "common" appears in all 5 docs, "rare" in only 1.
        let docs = vec![
            Candidate {
                name: Some("Common Rare Doc"),
                description: None,
                repo_relative_path: "a.md",
            },
            Candidate {
                name: Some("Common Doc Two"),
                description: None,
                repo_relative_path: "b.md",
            },
            Candidate {
                name: Some("Common Doc Three"),
                description: None,
                repo_relative_path: "c.md",
            },
            Candidate {
                name: Some("Common Doc Four"),
                description: None,
                repo_relative_path: "d.md",
            },
            Candidate {
                name: Some("Common Doc Five"),
                description: None,
                repo_relative_path: "e.md",
            },
        ];
        let idf = IdfTable::build(&docs);

        // "rare" is only in doc[0]; "common" is in all 5.
        assert!(
            idf.weight("rare") > idf.weight("common"),
            "rare ({}) should outweigh common ({})",
            idf.weight("rare"),
            idf.weight("common")
        );
    }

    #[test]
    fn idf_clamp_prevents_ubiquitous_term_reaching_zero() {
        // All 5 docs have "common" — df=N, so raw_idf = ln(6/6)+1 = 1.0
        // Clamp lower bound 0.5 means it never goes below 0.5.
        let docs: Vec<Candidate> = (0..5)
            .map(|i| Candidate {
                name: Some("Common name"),
                description: None,
                repo_relative_path: ["a.md", "b.md", "c.md", "d.md", "e.md"][i],
            })
            .collect();
        let idf = IdfTable::build(&docs);
        assert!(idf.weight("common") >= 0.5);
        assert!(idf.weight("common") <= 1.8);
    }

    #[test]
    fn idf_clamp_prevents_unique_term_dominating_tiny_corpus() {
        // Only 2 docs, one unique term — without clamp raw_idf = ln(3/2)+1 ≈ 1.4
        let docs = vec![
            Candidate {
                name: Some("Unique term here"),
                description: None,
                repo_relative_path: "a.md",
            },
            Candidate {
                name: Some("Other doc"),
                description: None,
                repo_relative_path: "b.md",
            },
        ];
        let idf = IdfTable::build(&docs);
        assert!(idf.weight("unique") <= 1.8);
    }

    // --- Normalization ---

    #[test]
    fn normalize_text_lowercases_and_splits_on_punctuation() {
        let toks = normalize_text("Hello, World!");
        assert_eq!(toks, vec!["hello", "world"]);
    }

    #[test]
    fn normalize_path_strips_extension_and_splits_separators() {
        let toks = normalize_path("docs/design-docs/my_guide.md");
        assert!(toks.contains(&"docs".to_string()));
        assert!(toks.contains(&"design".to_string()));
        assert!(toks.contains(&"docs".to_string()));
        assert!(toks.contains(&"my".to_string()));
        assert!(toks.contains(&"guide".to_string()));
        // Extension should not produce a token
        assert!(!toks.contains(&"md".to_string()));
    }

    // --- IdfTable::build isolation helper ---

    #[test]
    fn can_build_idf_from_empty_corpus() {
        let idf = IdfTable::build(&[]);
        // Unknown tokens return 1.0
        assert_eq!(idf.weight("anything"), 1.0);
    }
}
