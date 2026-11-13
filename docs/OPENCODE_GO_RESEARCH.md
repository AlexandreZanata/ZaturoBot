# OpenCode Go adapter evidence

Checked on 2026-11-13 against the official sources below. This is dated integration
evidence, not a promise that upstream policy, availability, or limits remain unchanged.
The owner selected only `deepseek-v4.1-flash` and separately authorized bounded paid
verification. No other model is configured as a fallback.

## Verified Go contract

DeepSeek's published Go route is
`https://opencode.ai/inference/go/openai/v1/chat/completions`. Direct clients identify
their product and send a stable `x-opencode-session`. Go targets coding-agent traffic;
DeepSeek requires Global processing in workspace privacy settings. The provider lists
no training and zero-day retention for DeepSeek, with an agreement currently valid
through 2026-11-30. Go allowance windows are five-hour, weekly, and monthly; usage may
also debit paid balance when Extra Usage is enabled. Responses report
`usage.opencode.allowance`. These facts do not authorize unrelated personal automation.
[Official Go reference](https://opencode.ai/docs/console/go/).

## Authentication and usage

The Console API accepts bearer API keys. A narrowly scoped key needs model-request
permission; read-only model-list permission does not authorize inference. Personal and
service-account credentials have different account and budget semantics. Do not assume
that a service-account key inherits an individual subscription.
[API overview](https://opencode.ai/docs/console/api/).

Chat Completions is a JSON POST API. Its documented response usage can contain
`prompt_tokens`, `completion_tokens`, optional `cost`, and OpenCode billing/token
metadata. Missing price data remains absent; it is not inferred as zero. Local token
limits apply independently of upstream plans or remaining credit.
[Inference API](https://opencode.ai/docs/console/api/inference/).

## Supported subset and unverified capabilities

The implemented adapter uses bounded text messages and locally validates a small JSON
action grammar. It does not rely on native function calling, vision input, or a provider
guarantee of JSON-schema enforcement. No numeric provider context-window or per-minute
request limit is asserted here: the current integration uses its own conservative
context/output/request limits and handles throttling as a classified failure. The
[Go model catalog](https://opencode.ai/inference/go/v1/models) is the current metadata
source for a later capability expansion; fetching it is not an inference test.

## Terms and data-policy boundary

The reviewed terms cover hosted inference, internal use, restrictions on abusive use,
and third-party provider policies. The documented coding-agent API is the basis for this
bounded coding diagnostic; it is not a blanket authorization for general-purpose or
arbitrary background automation. No account rotation, quota circumvention, scraping of
the service, resale, or sharing of the owner's credential is implemented. Ambiguous
non-coding/unattended workloads require a separate eligibility decision.
[Terms of Use](https://opencode.ai/legal/terms-of-service).

The privacy policy distinguishes pass-through inference from provider and hosted-agent
retention. A provider's data policy still applies to submitted content. ZaturoBot sends
only explicitly classified public coding requests in this phase; private code, mail,
documents, credentials, and screenshots are outside the enabled route.
[Privacy Policy](https://opencode.ai/legal/privacy-policy).

Recheck these references before changing endpoints, adding a model/data category,
expanding workload eligibility, or relying on a retention agreement beyond its date.
