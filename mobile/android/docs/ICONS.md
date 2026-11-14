# Android interface icons

The native Android interface includes a small local subset of Google's **Material
Symbols Rounded**, licensed under Apache-2.0. These are official SVG paths converted
to Android `VectorDrawable` XML, rather than hand-drawn approximations or Unicode
characters. No icon font, web request, new dependency or Material Components runtime
is required. The separate owner-provided ZaturoBot brand mark is unchanged.

Primary sources:

- [Google Material Symbols guide](https://developers.google.com/fonts/docs/material_symbols).
- [Google's official icon repository at the pinned revision](https://github.com/google/material-design-icons/tree/10ecc14790612ca8fd164e3f4ffe9c16fc50d5b7).
- [Upstream license](https://github.com/google/material-design-icons/blob/10ecc14790612ca8fd164e3f4ffe9c16fc50d5b7/LICENSE).

The full license, local adaptation notice and per-icon source URLs/SHA-256 hashes
ship in [the application assets](../app/src/main/assets/licenses/material-symbols-rounded/).
The repository revision is `10ecc14790612ca8fd164e3f4ffe9c16fc50d5b7`.

## Resources

Navigation:

- `ic_chat`: `chat_bubble`, the hub/conversation destination.
- `ic_services`: `grid_view`, the available service destination.
- `ic_sync`: `sync`, explicit reconnection.
- `ic_settings`: `settings`, private application settings.

Conversation and filters:

- `ic_send`: `send`, the message composer action.
- `ic_filter`: `tune`, feed filters.
- `ic_search`: `search`, history search.
- `ic_calendar`: `calendar_today`, day selection.
- `ic_check`: `check_circle`, a confirmed result.
- `ic_close`: `close`, a dismiss action.

Services and private connection:

- `ic_terminal`: `terminal`, the bounded diagnostic action.
- `ic_youtube`: `smart_display`, the existing video service. This is a generic video
  symbol, not Google's YouTube logo or a claim of affiliation.
- `ic_phone`: `smartphone`, Android/USB discovery.
- `ic_shield`: `shield_lock`, private connection status.
- `ic_folder`: `folder_open`, the private connection document picker.
- `ic_key`: `key`, the scoped application credential.

## Rendering and accessibility

Every drawable has a 24dp intrinsic size. The official paths are unchanged. A group
translation of `+960` on Y maps the SVG `viewBox="0 -960 960 960"` into Android's
positive 960-by-960 viewport. All paths default to opaque white without a baked-in
`tint`; the native view applies the appropriate selected/inactive/disabled color.
Do not replace these symbols with text glyphs or change their path geometry to match
an inactive state.

Size the surrounding touch target independently from the 24dp glyph (at least 48dp
for icon-only actions). Give an icon-only action a localized content description;
a decorative icon beside a visible label should not repeat that label to accessibility
services. Use explicit selected/disabled states and preserve text labels for bottom
navigation. The brand logo retains its own geometry and ownership and does not receive
the Material icon license.

## Verification

The imported files were parsed as XML and compiled with Android SDK Build Tools 36's
`aapt2 compile` before integration. The parent UI build/lint and physical-device review
verify their final sizing, contrast, touch targets and selected-state tint. Resource
compilation alone does not claim a complete interface or a successful device test.
