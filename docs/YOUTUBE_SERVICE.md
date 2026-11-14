# YoutubeBot service

The owner authorized improving the separate YoutubeBot and integrating it as a worker
service. Its original local checkout is named `chatgpt-recorder`; it was preserved.
The isolated updated repository keeps the original browser extension, recorder, prompts,
video pipeline and Studio upload source behavior. Harness/rule scaffolding is removed
there; concise AGENTS/component/accounts/service documentation replaces it. Its own
Google/ChatGPT browser windows and optional read-only OAuth perform human sign-in; the
hub never receives those account credentials. Exact channel identity is checked before
binding a profile. Existing local account fixes and thumbnail recovery notes are retained.

`zaturo-youtube` invokes only fixed typed JSON CLI verbs from a trusted absolute project
root and interpreter. It does not copy or reimplement the Python video pipeline. Root
configuration explicitly enables read/submit capabilities; scheduling additionally needs
`allow_schedule=true` and a typed owner-approved schedule request. Arguments are validated,
outputs curated/capped, subprocesses bounded and provider/owner/worker credentials removed
from their environment. Only explicit desktop-session variables are preserved as needed.

Service requests include profiles, channel runs, durable submit and operation status.
The worker journals intent before invoking the adapter. Repeating a committed result is
safe; an unresolved side-effecting submission reconciles by its operation UUID status,
never automatically submits again. The service's own durable ledger fences actor launch.
Independent per-account/channel locks protect signed-in browser use. Heavy generation
runs in fixed transient user systemd units, separate from the 512 MiB diagnostic worker:
12 GiB MemoryHigh, 16 GiB MemoryMax, CPUQuota 200%, TasksMax 256, 12-hour maximum and no
restart. The 1 GiB VPS never runs Chrome, Python/GPU video, ffmpeg, ADB or model workloads.

Job acceptance is not video completion. A typed submission result says accepted/running;
a separate bounded worker status observer publishes actual completed/failed/uncertain
transitions to the durable hub feed. Claimed completion requires the requested count and
matching scheduled-run evidence. Lost actors or uncertain uploads require reconciliation,
not retries of an external effect. Legacy direct generator/upload interfaces remain.

Example owner CLI (the JSON contains typed parameters, never a shell command):

```bash
zaturo-hub --config /private/hub-client.toml task create --kind service \
  --target work-pc --service-request /private/youtube-request.json
```

A profiles request is `{"action":"youtube_profiles"}`. Scheduling requires the explicit
`youtube_submit` action with a configured channel/account, count 1..10 and
`approved_schedule:true`. It is not inferred from file contents, model output or casual
chat text. Do not use a live upload as an automated test.

Meaningful local-only profiles, metadata and media are migrated through an encrypted,
segmented, checksummed archive and private staged restorer. Regenerable venv/model caches
are rebuilt; original data is not deleted. Old source overlays are retained for review
and cannot overwrite improved code. Existing work-PC SSH/age keys and worker.db remain
in place. Browser cookies may be OS-bound, so owner Google sign-in on the destination is
still interactive. The restore helper preserves conflicts and existing profile files.
See [migration helper](../deploy/services/restore_youtube.py) and its adjacent runbook.
