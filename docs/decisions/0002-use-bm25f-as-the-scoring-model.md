---
description: "Decision to use BM25F as the `docgarden match` scoring model; read when changing field weighting, corpus statistics, IDF, or Lucene-derived scoring details."
---

# Use BM25F as the scoring model

## Context and Problem Statement

`docgarden match` ranks repository guidance with weighted scoring fields such as name, path context, and frontmatter description. The scorer needs field-aware lexical ranking without turning field matches into independent scores that can over-reward repeated evidence for the same term.

The scoring model also needs one collection-level rarity signal per query term. For repository routing, that signal should answer: how many candidate documents contain this term in any scoring field, out of the candidate collection being ranked?

Which scoring model should own field weighting, term-frequency saturation, and document-level rarity for `docgarden match`?

## Considered Options

- **Use plain BM25 over concatenated fields.** Merge candidate fields into one text stream before scoring. This gives document-level IDF but discards field weighting.
- **Use independent per-field BM25 scores.** Score each field separately and combine the field scores. This preserves field separation but breaks BM25's single non-linear term-frequency saturation across the document.
- **Use Lucene-shaped combined-field scoring.** Combine fields before BM25 scoring while using Lucene-style per-field-max corpus statistics.
- **Use BM25F as the scoring model.** Combine weighted field term frequencies before BM25 saturation, and compute IDF from document-level collection statistics: `df(term)` is the number of candidates containing `term` in any scoring field, and `N` is the total candidate documents in the scoring collection.

## Decision Outcome

Chosen option: **Use BM25F as the scoring model**, because:

- BM25F is the model that matches the product intent: field-aware lexical routing with explainable rare-term emphasis and bounded term-frequency saturation.
- Robertson, Zaragoza, and Taylor's "Simple BM25 Extension to Multiple Weighted Fields" (CIKM 2004) and Robertson and Zaragoza's "The Probabilistic Relevance Framework: BM25 and Beyond" (2009) describe BM25F as combining weighted field term frequencies and computing IDF at the document level.
- In small curated corpora, document-level IDF answers the relevant routing question: "How many candidate documents contain this term in any scoring field?"
- Per-field term frequency, length normalization, field weighting, and BM25 parameters still provide the multi-field behavior. Document-level IDF does not collapse the scorer into plain BM25.
- Functionally empty routed documents should be prevented by linting and authoring quality checks, not by changing the BM25F collection-size definition.
