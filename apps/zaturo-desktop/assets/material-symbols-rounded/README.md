# Desktop interface icons

These 16 local **Material Symbols Rounded** SVGs come from Google's
[official repository](https://github.com/google/material-design-icons/tree/10ecc14790612ca8fd164e3f4ffe9c16fc50d5b7)
at revision `10ecc14790612ca8fd164e3f4ffe9c16fc50d5b7`, matching the Android subset.
The [provenance](provenance.json) records the exact original URLs and hashes.
The full [Apache-2.0 license](LICENSE.txt) and local [attribution notice](NOTICE.txt)
are included; they do not change the separate brand mark's ownership or project license.

All SVG bytes, fills, viewBoxes and path data are preserved unchanged. Set Slint image
`colorize` for selected, inactive and disabled states; size the display independently of
the 24-pixel upstream asset. Use localized text/content descriptions for icon-only actions
and avoid duplicating decorative icons beside an existing accessible label. Generic
`ic_youtube.svg` is the smart_display symbol, not a YouTube logo.

Navigation: `ic_chat`, `ic_services`, `ic_sync`, `ic_settings`.
Feed/actions: `ic_send`, `ic_filter`, `ic_search`, `ic_calendar`, `ic_check`, `ic_close`.
Service/connection: `ic_terminal`, `ic_youtube`, `ic_phone`, `ic_shield`, `ic_folder`, `ic_key`.
All are `.svg` files in this directory; no font, dependency or runtime download is needed.

Verify downloaded bytes from this directory with `sha256sum -c SHA256SUMS`.
The import validated all 16 SVGs as XML and matched their hashes and the license hash
against the mobile provenance before writing any asset. Parent client build and visual
checks validate Slint integration; an asset import alone does not establish final UI QA.
