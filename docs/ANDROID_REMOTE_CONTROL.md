# Android remote visual control

Status: architecture for later controlled operation; no unattended interactive stream
is currently activated. The Rust adapter provides device identity, local leases and private bounded captures;
the dedicated endpoint-encrypted desktop viewer can display one authorized screenshot
or the first changed screen from a finite watch. This is separate from continuous video
and interactive remote control. The native Android application is an owner client, not an
SMS permission agent or a replacement for USB debugging.

A future notebook display pipeline may use explicitly selected USB ADB, scrcpy, a
separate X11 display/lightweight window manager and loopback VNC/noVNC. The notebook
runs this workload; the 1 GB VPS coordinates authenticated sessions only. Do not route
continuous screen video through the VPS or expose ADB, VNC or noVNC publicly. A reviewed
private tunnel is required for a first prototype. An unimplemented WebRTC design must
not be reported as a working stream.

Use typed configuration for serial, resolution, frame rate, video bitrate and a maximum
viewing duration. Start no stream until an owner is viewing; stop all owned processes
on expiry/disconnect. `run cell -s SERIAL` remains a useful independently operated local
reference, but its unrestricted passthrough is not a model tool. See upstream
[scrcpy video controls](https://github.com/Genymobile/scrcpy/blob/master/doc/video.md)
and [connection behavior](https://github.com/Genymobile/scrcpy/blob/master/doc/connection.md).

Human and agent input must share a device-control lease. Owner takeover revokes the old
lease, cancels pending atomic input, grants manual ownership and requires fresh device
observation before any later agent grant. Current `SessionManager` implements local
lease fencing but no interactive input commands. Tap/swipe/text/keyevent, rotation,
UI Automator selectors, observation arbitration and browser-view input remain planned.
No caller may inject arbitrary ADB arguments to bridge those missing operations.
