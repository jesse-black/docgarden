---
description: "Decision to apply the Snowball English (Porter2) stemming algorithm to `docgarden match` tokens via the `rust-stemmers` crate; read when changing the stemming algorithm, evaluating an alternative crate, or considering vendoring a Snowball-compiled implementation."
---

# Use Snowball English (Porter2) via `rust-stemmers`

## Context and Problem Statement

[ADR 0003](0003-use-stemming-for-match-tokens.md) commits `docgarden match` to applying English stemming to query and corpus tokens but leaves both the specific algorithm and the implementation as open implementation concerns.

The English stemmer catalog includes Lovins, Porter, Porter2 (also called Snowball English), Lancaster, and Krovetz. Of these, only Snowball English has a maintained Rust implementation in the public ecosystem; choosing any other algorithm would require writing or vendoring a Rust implementation of it.

The implementation has to slot into the existing analyzer as a small library primitive: a function that takes one already-lowercased token and returns its stemmed form, alongside the existing tokenization and stopword filtering. There is no tokenizer pipeline, full-text index, or other framework infrastructure for the stemmer to integrate with.

Which English stemming algorithm should `docgarden` apply, and which implementation should it depend on?

## Considered Options

- **Snowball English (Porter2) via `rust-stemmers`** — incumbent Rust Snowball binding. Exposes `Stemmer::create(Algorithm::English).stem(token)`. Code is auto-generated from the Snowball compiler.
- **Snowball English (Porter2) via `tantivy-stemmers`** — a Snowball stemmer collection shaped as Tantivy tokenizer extensions, importing from `tantivy-tokenizer-api` and exposing `StemmerTokenizer` / `StemmerFilter` types. Each algorithm is gated behind a Cargo feature.
- **Snowball English (Porter2) via `porter_stemmers_rs`** — a newer Rust Snowball implementation.
- **Snowball English (Porter2) via vendored compiler output** — run the Snowball compiler against the English Porter2 source and check the generated Rust into this repository.
- **A different English algorithm, hand-implemented** — write Porter (1980), Lancaster, or Krovetz directly in Rust, since none of these have published Rust implementations.

## Decision Outcome

Chosen option: **Snowball English (Porter2) via `rust-stemmers`**, because:

- Snowball English is the modern English Porter-family standard and is conservative enough for repository routing on a small curated corpus. The more aggressive alternatives (Lancaster) would over-stem and erode precision; the older Porter (1980) and Lovins algorithms have been functionally superseded by Porter2 and offer no advantage for this use case.
- Hand-implementing a non-Snowball algorithm such as Krovetz adds substantial code and maintenance cost — Krovetz in particular requires shipping an English dictionary inside this repository — without producing better routing behavior than Porter2 already provides.
- `rust-stemmers` is the smallest implementation surface that matches the way the analyzer needs to call the stemmer: one function from token to stem, with no required surrounding pipeline. The shared analyzer already owns lowercasing and stopword filtering and only needs the morphological step plugged in alongside them.
- The crate has broad ecosystem adoption as the de facto Snowball binding for Rust, which gives high confidence that defects in the Porter2 output would have already surfaced and been corrected.
- The Snowball English specification is stable, and the crate's source is generated from the Snowball compiler. A low recent release cadence is therefore consistent with "stable primitive against a stable specification" rather than with abandonment.
- `tantivy-stemmers` is shaped around Tantivy tokenizer pipeline integration that `docgarden` does not have, and pulling it in would require either adopting that pipeline shape or wrapping it back down into a primitive.
- Vendoring Snowball-compiled output adds ongoing maintenance burden inside this repository without producing different runtime stemming behavior than depending on a published crate that already does this.
