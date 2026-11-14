# Bounded Linux computer use

The registered integration ID is `semantic-computer`.

The owner-authorized computer extension adds semantic desktop/browser steps, explicit
owner confirmations, read-only BitLupa results and a social-service boundary to the
existing native chat. It preserves the hub/worker architecture and existing YoutubeBot.
It is a controlled foundation, not unrestricted OS access, a general shell, or proof that
every Linux application works. Local profiles and actual destination-PC acceptance are
required before a capability is operational.

## Chosen open-source foundations

The Linux backend is [computer-use-linux v0.7.53](https://github.com/agent-sh/computer-use-linux/tree/4ee79ac80bb4db157892cf68f0d6fba5547538be),
MIT, commit `4ee79ac80bb4db157892cf68f0d6fba5547538be`. Its AT-SPI semantic tree and
window-targeted input fit the existing Rust/Linux workers and text-only DeepSeek route.
It supports X11/EWMH and compositor-specific Wayland integrations; that is upstream
support, not measured compatibility with the owner's notebook. Run the actual `doctor`
checks in the intended desktop session. ZaturoBot uses a fixed stdio MCP bridge rather
than adopting another assistant's account, provider or authorization stack.

The browser backend is [Microsoft Playwright MCP v0.0.85](https://github.com/microsoft/playwright-mcp/tree/4e0794dd0a0176519830a5ed27b491137cf9338f),
Apache-2.0, commit `4e0794dd0a0176519830a5ed27b491137cf9338f`. Semantic accessibility
snapshots and element references avoid routine screenshot/vision calls. Only a small
subset of its tools is exposed. Code execution, cookies, arbitrary JavaScript, file
access, uploads and downloads are not forwarded.

[OpenClaw](https://github.com/openclaw/openclaw) remains a useful reference, but importing
its whole gateway/client/provider stack would duplicate working ZaturoBot components.
[Agent S](https://github.com/simular-ai/Agent-S) is a broader screenshot/grounding stack;
its additional visual-model requirements do not fit this single-model, small-context
slice. [agent-workspace-linux](https://github.com/agent-sh/agent-workspace-linux) is a
possible future session manager, not an installed dependency or an OS security boundary.
A same-user virtual desktop still shares files, network and potentially keyrings.

Source pins document the reviewed interface. Runtime profiles separately require the
SHA-256 of the actual reviewed executable/Node/CLI artifacts. A version label alone
cannot authorize a replaced local program. No upstream source is copied into the hub.

## Chat and execution flow

Both native clients submit explicit assistant requests. Owner-facing commands include:

- `/computador <task>` starts a bounded custom computer task. `no computador: <task>`
  and `tarefa no pc: <task>` are equivalent explicit prefixes.
- `/precos <exact product>` or `/preços <exact product>` queries the selected worker's
  BitLupa bridge without a model call. For example, `/precos POCO X7` does not authorize
  substituting POCO X7 Pro.
- `/aprovar UUID` approves exactly the pending action displayed in that confirmation.
- `/recusar UUID` rejects that pending action. It grants no permission to a replacement.

`UUID` is the displayed confirmation/step ID, not a guessed task or operation ID.
Ordinary free conversation keeps its separate assistant route. The native composer sends
explicit confirmation commands using the dedicated `computer_confirm` transport action.
An `/aprovar` or `/recusar` prefix sent as ordinary `assistant` chat is inert, including
an old MCP `bot_chat` request using that fixed action. MCP tools cannot select this native
confirmation action. Website text, MCP replies, posts and model reports cannot approve
themselves; string filtering alone is not the authorization boundary.

The home worker advertises `computer.agent.v1` and uses its separate
[computer model profile](../config/computer-models.example.toml). It plans through the
existing Console transport and durable spend ledger. The work PC advertises
`computer.action.v1` and executes independently configured local adapters. Planner and
executor are distinct worker identities; no provider key is copied to the work PC.
Direct price search uses `price.search.v1`; private desktop frames use
`computer.private.v1`. Missing registrations/profiles produce an explicit blocker.

The coordinator stores parent tasks, ordered child steps and approval state. A parent
has a five-minute bound and at most eight model calls/steps; token, call and deadline
limits can stop it earlier. Each child has one attempt and a short deadline constrained
by its active parent and approval. Confirmation expires within three minutes and cannot
outlive the parent. Only the current authenticated owner/app can approve input or social
execution; a worker/model proposal cannot supply approval authority.

Read-only discovery and observations can run under their independent local grants.
Desktop/browser input waits for the exact native-owner confirmation. Social execution
also requires an earlier same-parent prepared draft, matching operation ID/hash and a
readable exact preview. Parent cancellation, pause, expiry and lost leases invalidate
future steps. A failed or uncertain step stops continuation; changing a sequence number
or reconnecting does not authorize repeating it. Stored results can reconcile without
repeating an external action.

## Semantic observations and permissions

The model receives the current bounded semantic observation plus compact receipts, not
the full chat, raw process output, credentials, screenshots or private reasoning history.
The Linux bridge bounds the tree to 64 nodes and pages it in groups of 16; one sanitized
observation is bounded to 8192 JSON bytes. Browser snapshots are similarly bounded.
Protected/password-like fields and known credential forms are omitted. These filters
reduce exposure; they are not a general sensitive-data classifier.

Desktop targets bind the observed `window_id`, process ID and exact `app_id`. Input
requires the current sanitized snapshot hash, observed element and focused target.
A changed window, process, tree, focus or protected field blocks the action instead of
silently choosing another window or retrying coordinates. Terminal, keyring, password,
privileged settings and launcher applications are excluded. Semantic/input access also
refuses browsers and code/agent-control applications, so desktop input cannot substitute
for the dedicated browser path or operate its own approval UI. No arbitrary executable,
argv, shell, clipboard, file selector or privileged setup tool is exposed.

Browser targets bind the single dedicated page. The profile is private and separate
from the owner's normal browser; its marker, PID command line, debugging endpoint and
artifact hashes must agree. CDP is literal `127.0.0.1`, not a public port. Page navigation
requires an exact configured public HTTPS origin, with subdomains granted separately.
Initial navigation rejects unapproved origins, private destinations and credential-bearing
URLs. After navigation, a page redirected outside its granted origin is refused for
semantic results/input; the browser may already have contacted that redirect destination.
This is not a firewall or a guarantee that browser subrequests stay on approved origins.
Input requires the current semantic snapshot/ref, local input grant and exact owner approval.

App, window, browser origin, input and capture permissions are independent. An allowlist
or origin check is not a filesystem/network sandbox: an allowed application or web page
can still have its normal operating-system and network capabilities. Prefer a dedicated
unprivileged Unix account/session; never assume Xvfb or another workspace manager creates
that isolation. Do not grant a normal browser through desktop permissions as a shortcut
around the dedicated browser policy.

Each Linux/browser profile's private `state_dir/STOP` is an emergency input stop. Create
that file to prevent or interrupt bridge RPCs; do not delete it automatically on reconnect.
Resume only after independent inspection and fresh state. The helper's owned process
guard cleans up its subprocess tree, including on outer-helper interruption. This is
process cleanup, not containment against hostile local code. Stop/cancel cannot reverse
an already dispatched click or a publication already accepted by an external service.
Social-service revocation and accepted schedules are described separately below.

## Local setup and bridge interfaces

Keep profiles mode0600 and private state/profile directories mode0700, outside Git. The
reviewed helpers are in [deploy/computer](../deploy/computer/linux_mcp.py); the Rust
[adapter](../crates/computer/src/lib.rs) selects fixed helper names and bounded arguments.
The installed worker's optional `computer` section supplies absolute Python/script paths,
Linux/browser/social profile paths, a BitLupa project/database and a fixed capture target.
Its `computer_models_config` is separate from ordinary chat and coding profiles.

The [local installer](../deploy/computer/install.py) first prints an inert plan:
`python3 deploy/computer/install.py`. Add `--apply` to copy the four reviewed helpers
into `~/.local/lib/zaturobot/computer` and create disabled private profiles at
`~/.config/zaturobot/computer-linux.toml` and `computer-browser.toml`. Existing profiles
are preserved; changed helpers receive private recoverable backups. It grants no app,
window, origin, input or capture access and changes no worker credentials or OS services.
An already reviewed official Linux artifact/license can be supplied with
`--backend-binary ABS --backend-sha256 HEX --backend-license ABS`; the installer verifies
the pinned release/license hashes before copying. It does not download or build them.
`--apply --pause desktop` or `--apply --pause browser` creates that backend's stop marker;
`--apply --resume desktop` or `--apply --resume browser` explicitly clears it after local
inspection. With an existing profile, these commands use its validated `state_dir` and
report the exact marker path; an invalid state configuration blocks the request. Resume
does not restore stale UI state or replay a task.

The Linux TOML profile contains `enabled`, `binary`, `binary_sha256`, `state_dir`,
`allowed_apps`, `allow_input`, `allow_capture` and `timeout_seconds` (1–30). Enable only
the intended ordinary app IDs; input/capture default to absent grants. The reviewed
upstream binary is named `computer-use-linux`. The target session needs working D-Bus,
AT-SPI and an actually supported display/compositor; no GPU is assumed.
Semantic observation/input can use independently supported Wayland integrations. Private
PC screenshots currently require a native X11 session: the bridge fixes upstream
`COMPUTER_USE_LINUX_SCREENSHOT_BACKEND=x11` with no fallback. The reviewed upstream
GNOME Shell, portal and `gnome-screenshot` routes use temporary PNG files, so they remain
blocked until a separately verified memory-safe Wayland capture route exists.

The browser TOML profile uses the same state/input/timeout controls, plus `node`,
`node_sha256`, `playwright_cli`, `playwright_cli_sha256`, `playwright_tree_sha256`,
`cdp_endpoint`, `profile_dir`, `browser_pid` and `allowed_origins`. The reviewed CLI file
is `cli.js` in a private dedicated npm tree. Its tree digest binds canonical relative
file paths and file SHA-256 values across the exact reviewed `@playwright/mcp`,
`playwright` and `playwright-core` package versions; changing a dependency while keeping
`cli.js` unchanged still blocks execution. Record that digest only after independent
artifact/integrity review; hashing unknown code alone does not establish trust. Generated
`.bin`/`.package-lock.json` metadata is excluded and never executed by the helper. Package
symlinks, unexpected packages and oversized trees are rejected. The dedicated
browser's private marker `.zaturobot-dedicated-browser` records schema1, purpose
`zaturobot-browser`, its exact PID and CDP endpoint. Its command line must bind that
profile, loopback debugging address and port. Unsafe `--no-sandbox`, unrestricted
remote-origin and disabled web-security flags are rejected. Keep one page, initially
`about:blank` or an explicitly allowed origin. The bridge does not launch/attach the
owner's normal browser or perform login/consent for them.

After installing independently verified runtime artifacts on the work PC, read-only
checks use these fixed interfaces:

```bash
python3 deploy/computer/linux_mcp.py --config /absolute/private/linux.toml doctor
python3 deploy/computer/browser_mcp.py --config /absolute/private/browser.toml doctor
```

Actual actions use `action --request-json /absolute/private/request.json`; the worker
writes that short-lived request with its lease/approval envelope. It is not a public
arbitrary-command interface. Never put lease tokens or credentials in command arguments,
Git, screenshots or logs. The helpers do not install upstream tools or run desktop setup.
No browser, model, Postiz, Docker or project compilation belongs on the lightweight VPS.

## BitLupa price evidence

The bridge reads [BitLupa](https://github.com/AlexandreZanata/BitLupa/tree/ead5e0f6caa63db9a3bf640f0e8652df6548a25f),
MIT, at `ead5e0f6caa63db9a3bf640f0e8652df6548a25f`. That source is a configuration/schema
foundation: it has no executable live collector/API and no enabled collection source.
ZaturoBot does not pretend to finish that separate project's missing collector, scrape
blocked stores, reinterpret an old fixture as today's price, or modify its repository.

`bitlupa_service.py --project-root PATH [--database PATH] search --query PRODUCT` reads
only normalized, provenance-backed stored offers when a compatible local database exists.
The category is new cellphones. Exact model/storage/RAM matching preserves X7 versus
X7 Pro and other variant distinctions. Stores need explicit source/robots/terms review
and approved retailer hosts. BRL price, payment type, availability, observation time,
matching confidence and source URL are validated; freshness is at most six hours and
can be shortened by an offer expiry. Stale offers are labelled, never called current.

Native results format observed prices and store links. The lowest displayed eligible
cash/Pix observation is distinct from a broad best-value recommendation. Shipping,
warranty and reputation are not verified, so `best_value_offer_id` stays absent. Coverage
and freshness limitations remain visible. Missing data or an untracked base POCO X7
returns a precise blocker, never fabricated prices or a substituted Pro model.

## Requested desktop screenshots

A task asking for a print/capture can receive a completion hint in the feed. A hint or
button authorizes no historical screenshot claim: it requests the current locally
configured window when the owner's request/confirmation is active. It is not evidence
that an API post was published or that this was the exact frame at completion time.
The native Services page also offers explicit private capture of the configured window.

The local `capture_target` fixes window/PID/app and the Linux profile separately grants
capture. The fixed native X11 capture route reads pixels in memory and has no disk-based
fallback; Wayland private PC capture stays blocked. Raw PNG stays in protected endpoint
memory, is standard-age encrypted on the
worker and travels through the existing one-time RAM relay. Native clients decrypt and
show a temporary Hub card; image bytes never enter normal feed SQL, transfer spool,
local screenshot files or the model. Fixed capture metadata may remain in the feed.
The image clears on close, navigation/focus loss or expiry within 60 seconds. OS-protected
windows, missing focus/grants and unavailable sessions stay blocked. This is a fresh
request, not a live stream or permanent screenshot archive. Python/managed runtimes do
not guarantee perfect erasure of every internal heap copy.

## Verification and actual prerequisites

Synthetic tests cover MCP/HTTP framing, scope/hash/stale-target rejection, process-tree
cleanup, exact owner confirmations, cancellation, bounded observations and private PNG
transport. They do not establish actual notebook/compositor compatibility, network login,
BitLupa collection, Postiz connections or a real work-PC screenshot. Record those only
after observed acceptance on the intended machine.

An actual hash-verified v0.7.53 artifact was exercised in an isolated session without a
display/session bus. Stdio initialization reported `computer-use-linux` version0.7.53;
`doctor` reported `can_register_mcp_tools=true`, with accessibility-tree, window query,
app/window focus, development input and screenshot capture all false; capture status was
`unavailable`. This verifies the pinned MCP interface and records the missing session.
It does not establish host or destination-notebook desktop readiness. The upstream CLI
rejects `--version`; use the MCP initialization/doctor evidence instead.
The separate upstream screenshot source review confirms that forced native X11 uses
GetImage and an in-memory PNG encoder. No actual X11 capture has been accepted on the
owner's host or work PC. The upstream's broader screenshot readiness does not prove
availability of this stricter private capture route.

The previous owner-authorized Console live check returned HTTP402 (balance/billing
permission unavailable). This remains an external prerequisite for model-planned custom
tasks; no coding-only Go fallback, model switch, automatic retry or top-up is used.
Read-only typed price searches and existing services keep their independent requirements.
See [social management](SOCIAL_MANAGEMENT.md), [assistant/shared coordination](ASSISTANT_AND_SHARED_TASKS.md),
[native clients](NATIVE_CLIENTS.md) and [sensitive screens](ANDROID_SENSITIVE_SCREENS.md).
