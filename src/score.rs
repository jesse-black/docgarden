use std::collections::{HashMap, HashSet};

use crate::analyzer::{normalize_path, normalize_text};

pub(crate) struct Candidate<'a> {
    pub(crate) name: Option<&'a str>,
    pub(crate) path_prefix: &'a str,
    pub(crate) description: Option<&'a str>,
}

pub(crate) struct CombinedFieldStats {
    doc_count: f32,
    avgdl: f32,
    df: HashMap<String, u32>,
}

impl CombinedFieldStats {
    pub(crate) fn build(candidates: &[Candidate<'_>]) -> Self {
        let mut name_sum_total_term_freq = 0;
        let mut path_prefix_sum_total_term_freq = 0;
        let mut description_sum_total_term_freq = 0;
        let mut df = HashMap::new();

        for candidate in candidates {
            let name_terms = candidate.name.map(normalize_text).unwrap_or_default();
            let path_prefix_terms = normalize_path(candidate.path_prefix);
            let description_terms = candidate
                .description
                .map(normalize_text)
                .unwrap_or_default();

            name_sum_total_term_freq += name_terms.len() as u32;
            path_prefix_sum_total_term_freq += path_prefix_terms.len() as u32;
            description_sum_total_term_freq += description_terms.len() as u32;

            let mut seen = HashSet::new();
            for term in name_terms
                .iter()
                .chain(path_prefix_terms.iter())
                .chain(description_terms.iter())
            {
                if seen.insert(term.as_str()) {
                    *df.entry(term.clone()).or_default() += 1;
                }
            }
        }

        let doc_count = candidates.len() as f32;
        let combined_sum_total_term_freq = NAME_BOOST * name_sum_total_term_freq as f32
            + PATH_PREFIX_BOOST * path_prefix_sum_total_term_freq as f32
            + DESCRIPTION_BOOST * description_sum_total_term_freq as f32;
        let avgdl = if doc_count > 0.0 && combined_sum_total_term_freq > 0.0 {
            combined_sum_total_term_freq / doc_count
        } else {
            1.0
        };

        Self {
            doc_count,
            avgdl,
            df,
        }
    }

    fn idf(&self, term: &str) -> f32 {
        let doc_count = self.doc_count.max(1.0);
        let df = self.df.get(term).copied().unwrap_or(0) as f32;
        (1.0 + (doc_count - df + 0.5) / (df + 0.5)).ln()
    }

    fn avgdl(&self) -> f32 {
        self.avgdl
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Field {
    Name,
    Description,
    Path,
}

pub(crate) struct ScoredHit {
    pub(crate) score: f32,
    pub(crate) matched_terms: u32,
    pub(crate) first_field_hit: Option<Field>,
}

const NAME_BOOST: f32 = 3.0;
const PATH_PREFIX_BOOST: f32 = 1.0;
const DESCRIPTION_BOOST: f32 = 1.0;
// Lucene BM25Similarity defaults.
const BM25_K1: f32 = 1.2;
const BM25_B: f32 = 0.75;

pub(crate) fn score(
    query_terms: &[String],
    candidate: &Candidate<'_>,
    stats: &CombinedFieldStats,
) -> ScoredHit {
    if query_terms.is_empty() {
        return ScoredHit {
            score: 0.0,
            matched_terms: 0,
            first_field_hit: None,
        };
    }

    let name_terms = candidate.name.map(normalize_text).unwrap_or_default();
    let path_prefix_terms = normalize_path(candidate.path_prefix);
    let description_terms = candidate
        .description
        .map(normalize_text)
        .unwrap_or_default();

    let combined_length = NAME_BOOST * name_terms.len() as f32
        + PATH_PREFIX_BOOST * path_prefix_terms.len() as f32
        + DESCRIPTION_BOOST * description_terms.len() as f32;
    let avgdl = stats.avgdl();

    let mut total = 0.0;
    let mut matched_terms = 0;
    let mut first_field_hit = None;

    for term in query_terms {
        let name_tf = term_frequency(&name_terms, term);
        let path_tf = term_frequency(&path_prefix_terms, term);
        let description_tf = term_frequency(&description_terms, term);

        let combined_freq = NAME_BOOST * name_tf as f32
            + PATH_PREFIX_BOOST * path_tf as f32
            + DESCRIPTION_BOOST * description_tf as f32;

        if combined_freq <= 0.0 {
            continue;
        }

        matched_terms += 1;
        if name_tf > 0 {
            first_field_hit = best_field_hit(first_field_hit, Field::Name);
        }
        if path_tf > 0 {
            first_field_hit = best_field_hit(first_field_hit, Field::Path);
        }
        if description_tf > 0 {
            first_field_hit = best_field_hit(first_field_hit, Field::Description);
        }

        let norm =
            combined_freq + BM25_K1 * (1.0 - BM25_B + BM25_B * (combined_length / avgdl.max(1e-6)));
        total += stats.idf(term) * (((BM25_K1 + 1.0) * combined_freq) / norm);
    }

    ScoredHit {
        score: total,
        matched_terms,
        first_field_hit,
    }
}

fn term_frequency(tokens: &[String], term: &str) -> u32 {
    tokens.iter().filter(|token| token.as_str() == term).count() as u32
}

fn best_field_hit(current: Option<Field>, candidate: Field) -> Option<Field> {
    Some(match current {
        Some(existing) => existing.min(candidate),
        None => candidate,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::{normalize_path, normalize_text};

    fn candidate<'a>(
        name: Option<&'a str>,
        path_prefix: &'a str,
        description: Option<&'a str>,
    ) -> Candidate<'a> {
        Candidate {
            name,
            path_prefix,
            description,
        }
    }

    fn terms(q: &str) -> Vec<String> {
        normalize_text(q)
    }

    #[test]
    fn rare_term_outranks_common_term() {
        let docs = vec![
            candidate(Some("rare guide"), "", None),
            candidate(Some("common guide"), "", None),
            candidate(Some("common plan"), "", None),
            candidate(Some("common review"), "", None),
        ];
        let stats = CombinedFieldStats::build(&docs);

        let rare = score(&terms("rare"), &docs[0], &stats);
        let common = score(&terms("common"), &docs[1], &stats);

        assert!(rare.score > common.score);
    }

    #[test]
    fn document_frequency_counts_term_once_across_all_fields_per_document() {
        let docs = vec![
            candidate(Some("routing"), "", None),
            candidate(None, "", Some("routing")),
            candidate(Some("other"), "", None),
        ];
        let stats = CombinedFieldStats::build(&docs);

        assert_eq!(stats.df.get("rout"), Some(&2));
    }

    #[test]
    fn empty_candidate_contributes_to_document_count() {
        let docs = vec![
            candidate(Some("routing"), "", None),
            candidate(Some("the"), "", Some("and")),
        ];
        let stats = CombinedFieldStats::build(&docs);

        assert_eq!(stats.doc_count, 2.0);
    }

    #[test]
    fn term_in_multiple_fields_of_one_document_has_document_frequency_one() {
        let docs = vec![candidate(Some("routing"), "routing", Some("routing"))];
        let stats = CombinedFieldStats::build(&docs);

        assert_eq!(stats.df.get("rout"), Some(&1));
    }

    #[test]
    fn boosted_field_outranks_weaker_field_at_equal_tf_df() {
        let docs = vec![
            candidate(Some("routing"), "", None),
            candidate(None, "", Some("routing")),
        ];
        let stats = CombinedFieldStats::build(&docs);

        let name_hit = score(&terms("routing"), &docs[0], &stats);
        let description_hit = score(&terms("routing"), &docs[1], &stats);

        assert!(name_hit.score > description_hit.score);
    }

    #[test]
    fn longer_combined_length_is_penalized_at_fixed_combined_freq() {
        let short = candidate(Some("review"), "", None);
        let long = candidate(
            Some("review"),
            "",
            Some("extra context words for a much longer description"),
        );
        let docs = vec![short, long];
        let stats = CombinedFieldStats::build(&docs);

        let short_score = score(&terms("review"), &docs[0], &stats);
        let long_score = score(&terms("review"), &docs[1], &stats);

        assert!(short_score.score > long_score.score);
    }

    #[test]
    fn stopword_filter_is_symmetric_for_index_and_query() {
        let docs = vec![
            candidate(
                Some("the active plan"),
                "docs/the-active-plan",
                Some("implement from the active plan"),
            ),
            candidate(Some("review"), "docs/review", None),
        ];
        let stats = CombinedFieldStats::build(&docs);

        assert_eq!(normalize_text("the active plan"), vec!["activ", "plan"]);
        assert_eq!(
            normalize_path("docs/the-active-plan.md"),
            vec!["doc", "activ", "plan"]
        );
        assert!(score(&terms("the active plan"), &docs[0], &stats).score > 0.0);
    }

    #[test]
    fn empty_path_prefix_does_not_panic() {
        let docs = vec![
            candidate(Some("root doc"), "", Some("review plan")),
            candidate(Some("nested doc"), "docs/active", Some("review")),
        ];
        let stats = CombinedFieldStats::build(&docs);
        let hit = score(&terms("review"), &docs[0], &stats);

        assert!(hit.score.is_finite());
        assert!(hit.score > 0.0);
    }

    #[test]
    fn strongest_field_hit_prefers_description_over_path() {
        let docs = vec![
            candidate(Some("review"), "", Some("active plan")),
            candidate(None, "docs/review", Some("active plan")),
        ];
        let stats = CombinedFieldStats::build(&docs);

        let first = score(&terms("review active plan"), &docs[0], &stats);
        let second = score(&terms("review active plan"), &docs[1], &stats);

        assert_eq!(first.matched_terms, second.matched_terms);
        assert_eq!(first.first_field_hit, Some(Field::Name));
        assert_eq!(second.first_field_hit, Some(Field::Description));
    }

    #[test]
    fn can_build_stats_from_empty_corpus() {
        let stats = CombinedFieldStats::build(&[]);
        assert_eq!(stats.avgdl(), 1.0);

        let empty = candidate(Some("anything"), "", None);
        let hit = score(&terms("anything"), &empty, &stats);
        assert!(hit.score.is_finite());
        assert!(hit.score > 0.0);
    }

    #[test]
    fn empty_query_scores_zero() {
        let docs = vec![candidate(Some("anything"), "", None)];
        let stats = CombinedFieldStats::build(&docs);
        let hit = score(&[], &docs[0], &stats);

        assert_eq!(hit.score, 0.0);
        assert_eq!(hit.matched_terms, 0);
        assert_eq!(hit.first_field_hit, None);
    }
}
