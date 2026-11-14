# System design and design system

The [six-page PDF](../../output/pdf/zaturobot-system-design.pdf) records the native-client design and deployed/implemented system boundaries as of 13 November 2026 (America/Cuiaba). The document is in English for engineering review; the owner interfaces default to Brazilian Portuguese.

The editable source is [generate_system_design.py](generate_system_design.py). It draws the supplied icon as PDF vector paths and reads the existing [brand tokens](../../assets/brand/tokens.json). The original [identity board](../../assets/brand/identity-board.png) and [SVG](../../assets/brand/zaturobot-icon.svg) remain unchanged. No credentials, host addresses or account identifiers are included.

The pages cover identity and accessible color use; the daily chat/activity view; runtime architecture; the preserved YouTube pipeline and account boundaries; expiring private Android viewing; and API, transfer and rollout contracts. The interface diagram is a conceptual layout, not a screenshot or a claim of live activity. The PDF distinguishes implementation evidence from the outstanding Google consent, physical USB and real video/upload checks.

## Rebuild

Use Python 3 with ReportLab and the four Noto Sans/Noto Sans Mono TTF files installed under `/usr/share/fonts/truetype/noto/`:

```sh
python3 docs/design/generate_system_design.py
```

Inter and JetBrains Mono are the brand targets. They were unavailable in the authoring environment, so this PDF embeds Noto Sans and Noto Sans Mono and labels them as fallbacks. The generator fails clearly if the expected fallback files are unavailable; it does not silently substitute fonts.

To render every page for visual review, install Poppler and run:

```sh
mkdir -p output/pdf/qa
pdftoppm -r 90 -png output/pdf/zaturobot-system-design.pdf output/pdf/qa/page
```

Inspect all six latest page images after changes. The generator also checks its text boxes against page margins, but that mechanical check cannot replace visual inspection. It reports page count, byte size, SHA-256, embedded fallback families and computed contrast ratios. The [architecture preview](../../output/pdf/zaturobot-system-design-preview.png) is the rendered third page.

## Sources and scope

Project Rust types, routes, local configuration and implementation records supply the product contracts. The PDF links to the official [Android ADB documentation](https://developer.android.com/tools/adb), [Android secure-window contract](https://developer.android.com/reference/android/view/WindowManager.LayoutParams#FLAG_SECURE), [Google desktop OAuth guide](https://developers.google.com/identity/protocols/oauth2/native-app) and [Slint guide](https://docs.slint.dev/latest/docs/slint/guide/getting-started/). These references explain their respective platforms; they do not claim that account or phone acceptance tests have been performed.

Ordinary encrypted files use the documented spool. Sensitive phone frames use the separate expiring RAM-only path and the Linux private viewer. The initial Android application supports chat/feed; private age image viewing is desktop-only. The work PC's existing 0.3 enrollment was owner-confirmed; the additive 0.4 installation must preserve its keys and worker journal.
