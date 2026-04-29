---
description: "Decision to apply English stemming to `docgarden match` query and corpus tokens; read when changing tokenization, query parsing, BM25F corpus statistics, or matched-term highlighting."
---

# Use stemming for match query and corpus tokens

## Context and Problem Statement

`docgarden match` ranks repository guidance with BM25F over a small curated corpus (per [ADR 0002](0002-use-bm25f-as-the-scoring-model.md)). BM25F treats morphological variants as unrelated terms, so a query for `plan` does not match a document that only contains `plans`, and `review` does not match `reviews`. In a small corpus, document-level IDF cannot recover this signal on its own.

Should `docgarden match` apply morphological normalization to query and corpus tokens?

## Considered Options

- **Exact tokens only.** Rely on authors to write queries and frontmatter that share surface forms.
- **English stemming.** Apply a conservative English stemmer to query and corpus tokens.
- **Lemmatization.** Use a dictionary-backed lemmatizer to map tokens to canonical lemmas.
- **Character n-gram or fuzzy matching.** Approximate morphological matches by indexing sub-word n-grams or by allowing edit-distance matches at query time.

## Decision Outcome

Chosen option: **English stemming**, because:

- Stemming directly addresses the dominant remaining recall gap in repository routing: singular/plural and other closely related word forms that BM25F treats as unrelated terms.
- A conservative algorithmic stemmer is deterministic, has no external services, and adds little implementation complexity.
- Lemmatization adds dictionary dependencies and language-model assumptions without measurable additional benefit for the short, technical, English-dominant text that `docgarden` routes over.
- Character n-gram and fuzzy matching change the matching model rather than the token analysis, and would erode the precision properties that make BM25F appropriate for a small curated corpus.

