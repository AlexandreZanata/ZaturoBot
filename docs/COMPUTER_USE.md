# Linux computer use

Status: planned for Phase 3. No desktop, screenshot, input, browser automation, streaming,
or human-takeover implementation is included in the foundation.

Use a dedicated notebook Unix user and isolated automation session, initially Xvfb plus
Openbox on Debian Stable; Ubuntu LTS remains an alternative. Xfce may provide an
interactive session. Do not assume GPU, CUDA, or NVENC. Prefer semantic browser and
accessibility operations before coordinates. Never place a desktop on the hub.

## Read-only reference assessment, 2026-11-13

[agent-workspace-linux](https://github.com/agent-sh/agent-workspace-linux) offers isolated
agent-owned desktops through MCP stdio, with X11 as its default backend and native
Wayland alternatives. Its documented X11 setup uses Xvfb/Openbox. It is MIT licensed.
Its repository includes CI and releases; no maintenance SLA or notebook compatibility
has been measured. It is a candidate session manager, subject to pinned-version review.

[computer-use-linux](https://github.com/agent-sh/computer-use-linux) exposes MCP, native
agent tools, and CLI desktop operations. Its documented features include AT-SPI semantic
trees, screenshots, window targeting, and X11/EWMH plus compositor-specific Wayland
support. It is MIT licensed and targets the actual desktop. Any ZaturoBot adapter must
bind it to the dedicated session and prevent access to the owner's normal session.
CI/releases exist; current release cadence and runtime suitability remain unverified.

[noVNC](https://github.com/novnc/noVNC) provides a JavaScript browser VNC client, including
mobile browsers. It requires a VNC server with WebSockets or a bridge such as websockify;
it does not create an isolated desktop or enforce ZaturoBot policy. Its core is MPL-2.0,
with separate asset/component licenses described in its
[license document](https://github.com/novnc/noVNC/blob/master/LICENSE.txt).
The repository has established CI and contribution mechanisms. No dependency is adopted.

## Session contract and acceptance

Each tool requires stable ID, typed input/output, permission class, session/workspace
scope, timeout, resource budget, cancellation behavior, and audit metadata. Start with
`computer.screenshot`, semantic inspection, and bounded browser navigation before input.
Input ownership is a lock with `agent`, `human`, and `stopped` states. Human takeover
first revokes agent input, waits for in-flight input to finish, then grants human control.
Resume needs an explicit owner action and fresh screen/context inspection.

Dependencies: authenticated observation transport, policy grants, artifact retention, and
owned process cleanup. Threats include session escape, screenshot disclosure, downloads,
clipboard secrets, and conflicting inputs. Acceptance must verify session isolation,
revocation, reconnect behavior, headless operation without GPU, and explicit human control.
