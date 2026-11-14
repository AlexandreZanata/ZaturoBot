# Fixed YoutubeBot service adapter

Own bounded subprocess transport and whitelisted typed results for the existing Python
YoutubeBot service. Preserve its extension, generation and upload implementations; never
add shell execution, credentials, login automation or an alternative video pipeline here.
The configured project/interpreter are trusted local installation paths, not task input.

Submit requires an explicit owner-approved schedule request AND local allow_schedule.
The worker must journal intent before invoking it. If the worker loses the response,
reconcile only with status for the same operation; never automatically submit it again.
Accepted/running means admitted background work, not completed/generated/uploaded videos.
Only matched completed evidence can produce completed submission. Poll terminal job states
honestly and record errors/uncertainty without repeating the external effect.

Keep the child environment minimal: no inherited Zaturo owner, worker or model credentials.
The Python project reads only its own private account/configuration. Responses are bounded,
curated metadata; do not pass through raw JSON fields, log strings, account tokens or
absolute artifact paths. Tests use fake executables and temporary fake projects, never
real Google authorization, generation or uploads. Domain request/result contracts live
in core; durable replay fencing lives in db and application dispatch lives in worker.
