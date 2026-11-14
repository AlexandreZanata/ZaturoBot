# Commerce foundation

Status: F1 domain foundation, with no executable integration. The
[refined plan](planning/commerce-refined/ZATURO-COMMERCE-REFINED-PLAN.md) supersedes the
[original planning baseline](planning/autonomous-commerce/README.md) where they differ.
Rust types in `zaturo-core::commerce` define implemented contracts; the backlog defines
later phases. Actual verification belongs to the [F1 report](COMMERCE_F1_REPORT.md).

## Implemented boundary

`zaturo-core::commerce` contains pure functions only. It performs no database, filesystem,
network, provider, account or publication operation. Callers supply UTC milliseconds and
observations. Computing a policy hash, template or slot grants no execution authority.
There is no new task/service capability, Hub endpoint or active commerce configuration.
Existing Go PublicCoding and explicitly opted-in Personal chat/planning routes retain
their current contracts. F6's proposed CommerceDraft purpose is not available in F1.

- `money`: BRL integer minor units; decimal strings with comma/dot, checked arithmetic,
  no negative values, grouping, exponent or floating-point input.
- `offer`: bounded item/title/category, price, observation/expiry, exact HTTPS program
  authority, evidence digest, optional commission/sub-ID and explicit listing benefits.
  Imported title controls are removed; canonical fields are validated again before use.
  A missing validity time uses observation plus six hours; explicit offer expiry remains
  distinct from the configured price-age limit. No link bytes are rewritten.
- `content`: explicit content transitions; an uncertain effect cannot be retried through
  a normal transition. A separate evidenced owner-resolution contract does not send or
  schedule anything; the later service must authenticate the resolving owner.
- `policy`: strict four-mode action policy, caps, one program per channel, categories,
  BRL price band, nonoverlapping windows, disclosure and kill thresholds. Canonical JSON
  and SHA-256 bind revisions. Validation checks values, not owner consent or activation.
- `claims` and the template renderer: advertising first, approved disclosure before the
  link, factual benefit lines, forbidden physical-test/superlative/comparison claims,
  number/link checks and bounded HTML escaping. Fresh prices include their observation
  time; stale prices are omitted. A stale amount embedded in imported title/benefits
  blocks rendering instead of being silently rewritten. An expired offer is refused
  even without a price. F1 accepts exactly two affirmative disclosure wordings: the
  exported default and the refined plan's "Link de afiliado: posso receber comissão se
  você comprar." New wording needs a reviewed contract change, not fuzzy text acceptance.
- `slots`: stable campaign/local-date/index keys and UTC window boundaries calculated
  with pinned IANA rules. Missed windows are skipped. This is calculation, not a running
  scheduler, durable admission, dispatch or a promise of exactly-once effects.
- `kill_rules`: deterministic pause/demotion/price/model decisions from explicit known
  counters and injected time. Missing measurements do not become zero revenue/clicks;
  complete fresh observation windows are required for zero-result thresholds. The
  commission threshold uses the explicitly supplied start of automatic operation,
  rather than campaign age or time spent preparing drafts.
- `attribution`: bounded program-scoped exact sub-ID associations. Unknown IDs remain
  unattributed. Generated candidates are never truncated or inserted into an affiliate
  link; official link creation and tracking permission remain external prerequisites.

Public/deserialized structures must be validated before use. A digest is a binding to
provided facts, not independent verification that a product exists, its price is real,
its media is licensed or its channel is approved. A matching host is syntax admission,
not affiliate enrollment or proof that a URL came from an official link generator.
Use only owner-verified imports or an authorized API in later phases. Claim checks are a
conservative validator for the bounded templates, not a general truth detector.

## Adoption and next phases

The owner's implementation request authorizes F1 and GitHub code publication after
validation. Brand/niche/account/media/allowance choices remain unresolved; fixture values
do not adopt the proposed public brand or create a production policy. The remaining
onboarding questions block activation, not parameterized pure-library development.

Next is F2 (BL-201 onward): current-schema additive persistence, bounded official imports,
ranking, stored template batches and truthful Hub summaries/status under explicit scopes.
The refined plan's assumed generic artifact-upload API must be rechecked: current artifact
routes expose encrypted-transfer metadata, not a plaintext commerce blob store. Allocate
migration numbers after re-reading the then-current database streams.

F3 adds policy activation, stop, occurrences and owner approvals; F4 adds a separately
authorized Telegram transport; F5 adds verified conversions; F6 adds bounded drafting
through the existing model router. No later phase is implied by F1. MeuAssistente remains
read-only, experimental WhatsApp remains synthetic-only, and YoutubeBot is unchanged.

See [dependency rationale](DEPENDENCIES.md) for the pinned timezone library and reused
hash/URL parsers. [F1 acceptance](planning/commerce-refined/IMPLEMENTATION-BACKLOG.md)
requires offline behavioral tests and the full repository quality command.

Run the synthetic local demonstration with
`cargo run -p zaturo-core --example commerce_draft --locked`. It prints one pt-BR draft
from fixed fixture facts and a fixture URL; it does not read private configuration,
contact a platform, save a campaign or publish. This is a domain demonstration, not a
new owner CLI/API integration. The output is also recorded in the F1 verification.
