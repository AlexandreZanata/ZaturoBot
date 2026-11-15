#!/usr/bin/python3
"""One consented Wayland window through ScreenCast/PipeWire, PNG only on stdout.

No Screenshot portal, persistence, restore token, monitor, fallback, files, OCR or input.
The trusted worker invokes this only for a claimed native private-view grant. The portal
is the source authority: its single window stream stays bound to this one live session.
"""
from __future__ import annotations
import os
import signal
import sys
import time
import uuid

MAX_IMAGE = 8 * 1024 * 1024

class Rejected(Exception):
    pass

def need(condition):
    if not condition:
        raise Rejected()

def selected_stream(result):
    streams = result.get("streams", [])
    need(isinstance(streams, (list, tuple)) and len(streams) == 1)
    node, properties = streams[0]
    need(type(node) is int and 0 < node < 2**32 and properties.get("source_type") == 2)
    size = properties.get("size")
    need(isinstance(size, (list, tuple)) and len(size) == 2)
    width, height = size
    need(type(width) is int and type(height) is int and 0 < width <= 4096 and 0 < height <= 4096 and width * height <= 16777216)
    serial = properties.get("pipewire-serial")
    need(serial is None or (type(serial) is int and 0 < serial < 2**64))
    ratio = min(1.0, 1920 / width, 1080 / height)
    return node, serial, max(1, int(width * ratio)), max(1, int(height * ratio))

class Portal:
    def __init__(self):
        import gi
        gi.require_version("Gst", "1.0")
        from gi.repository import Gio, GLib, Gst
        self.Gio, self.GLib, self.Gst = Gio, GLib, Gst
        self.bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
        self.session = None
        self.pending = None
        self.fd = None
        self.pipeline = None
        self.end = time.monotonic() + 30
        self.closed = False
        self.session_signal = None

    def call(self, method, variant):
        GLib, Gio = self.GLib, self.Gio
        need(time.monotonic() < self.end)
        return self.bus.call_sync("org.freedesktop.portal.Desktop", "/org/freedesktop/portal/desktop", "org.freedesktop.portal.ScreenCast", method, variant, None, Gio.DBusCallFlags.NONE, 3000, None)

    def request(self, method, signature, arguments):
        GLib, Gio = self.GLib, self.Gio
        token = "zaturo_" + uuid.uuid4().hex
        path = "/org/freedesktop/portal/desktop/request/" + self.bus.get_unique_name()[1:].replace(".", "_") + "/" + token
        options = arguments[-1].copy()
        options["handle_token"] = GLib.Variant("s", token)
        arguments = (*arguments[:-1], options)
        result = []
        loop = GLib.MainLoop()
        def response(_bus, _sender, _path, _interface, _signal, parameters):
            result.append(parameters.unpack()); loop.quit()
        subscription = self.bus.signal_subscribe("org.freedesktop.portal.Desktop", "org.freedesktop.portal.Request", "Response", path, None, Gio.DBusSignalFlags.NONE, response)
        timer = GLib.timeout_add(max(1, int((self.end-time.monotonic())*1000)), lambda: (loop.quit(), False)[1])
        self.pending = path
        try:
            returned = self.call(method, GLib.Variant(signature, arguments)).unpack()[0]
            need(returned == path)
            if not result:
                loop.run()
            need(len(result) == 1 and result[0][0] == 0 and not self.closed)
            return result[0][1]
        finally:
            self.bus.signal_unsubscribe(subscription)
            if GLib.MainContext.default().find_source_by_id(timer): GLib.source_remove(timer)
            if not result: self.close_path(path, "org.freedesktop.portal.Request")
            self.pending = None

    def close_path(self, path, interface):
        try:
            self.bus.call_sync("org.freedesktop.portal.Desktop", path, interface, "Close", None, None, self.Gio.DBusCallFlags.NONE, 1000, None)
        except Exception:
            pass

    def capture(self):
        GLib, Gio, Gst = self.GLib, self.Gio, self.Gst
        need(os.environ.get("XDG_SESSION_TYPE") == "wayland" and bool(os.environ.get("WAYLAND_DISPLAY")))
        session = self.request("CreateSession", "(a{sv})", ({"session_handle_token": GLib.Variant("s", "zaturo_"+uuid.uuid4().hex)},)).get("session_handle")
        need(isinstance(session, str) and session.startswith("/org/freedesktop/portal/desktop/session/"))
        self.session = session
        def closed(*_args): self.closed = True
        self.session_signal = self.bus.signal_subscribe("org.freedesktop.portal.Desktop", "org.freedesktop.portal.Session", "Closed", session, None, Gio.DBusSignalFlags.NONE, closed)
        self.request("SelectSources", "(oa{sv})", (session, {"types":GLib.Variant("u",2),"multiple":GLib.Variant("b",False),"cursor_mode":GLib.Variant("u",1),"persist_mode":GLib.Variant("u",0)}))
        node, serial, width, height = selected_stream(self.request("Start", "(osa{sv})", (session, "", {})))
        result, descriptors = self.bus.call_with_unix_fd_list_sync("org.freedesktop.portal.Desktop", "/org/freedesktop/portal/desktop", "org.freedesktop.portal.ScreenCast", "OpenPipeWireRemote", GLib.Variant("(oa{sv})",(session,{})), GLib.VariantType.new("(h)"), Gio.DBusCallFlags.NONE, 3000, None, None)
        self.fd = descriptors.get(result.unpack()[0])
        Gst.init(None)
        pipeline = Gst.Pipeline.new("zaturo-private-window")
        self.pipeline = pipeline
        elements = [Gst.ElementFactory.make(name, None) for name in ["pipewiresrc","capsfilter","videoconvert","videoscale","capsfilter","pngenc","appsink"]]
        need(all(element is not None for element in elements))
        source, raw, convert, scale, reduced, encoder, sink = elements
        source.set_property("fd", self.fd)
        if serial is not None:
            need(source.find_property("target-object") is not None)
            source.set_property("target-object", str(serial))
        else:
            source.set_property("path", str(node))
        source.set_property("num-buffers", 1)
        source.set_property("min-buffers", 1); source.set_property("max-buffers", 2)
        source.set_property("resend-last", False)
        raw.set_property("caps", Gst.Caps.from_string("video/x-raw,width=(int)[1,4096],height=(int)[1,4096]"))
        reduced.set_property("caps", Gst.Caps.from_string(f"video/x-raw,format=RGB,width={width},height={height}"))
        encoder.set_property("snapshot", True)
        sink.set_property("max-buffers", 1); sink.set_property("drop", True); sink.set_property("sync", False)
        sink.set_property("enable-last-sample", False)
        for element in elements: pipeline.add(element)
        for first, second in zip(elements,elements[1:]): need(first.link(second))
        need(pipeline.set_state(Gst.State.PLAYING) != Gst.StateChangeReturn.FAILURE)
        sample = None
        while time.monotonic() < self.end and not self.closed:
            while GLib.MainContext.default().pending(): GLib.MainContext.default().iteration(False)
            sample = sink.emit("try-pull-sample", 100 * Gst.MSECOND)
            if sample is not None: break
            message = pipeline.get_bus().pop_filtered(Gst.MessageType.ERROR | Gst.MessageType.EOS)
            if message is not None: raise Rejected()
        need(sample is not None and not self.closed and time.monotonic() < self.end)
        buffer = sample.get_buffer()
        need(buffer is not None and 57 <= buffer.get_size() <= MAX_IMAGE)
        success, mapped = buffer.map(Gst.MapFlags.READ)
        need(success)
        try:
            data = mapped.data
            need(data[:8] == b"\x89PNG\r\n\x1a\n" and data[-8:-4] == b"IEND")
            need(int.from_bytes(data[16:20],"big") == width and int.from_bytes(data[20:24],"big") == height)
            sys.stdout.buffer.write(data); sys.stdout.buffer.flush()
        finally:
            buffer.unmap(mapped)

    def close(self):
        if self.pipeline is not None: self.pipeline.set_state(self.Gst.State.NULL)
        if self.fd is not None: os.close(self.fd); self.fd = None
        if self.pending is not None: self.close_path(self.pending,"org.freedesktop.portal.Request")
        if self.session is not None: self.close_path(self.session,"org.freedesktop.portal.Session")
        if self.session_signal is not None: self.bus.signal_unsubscribe(self.session_signal)

def main():
    portal = None
    try:
        need(len(sys.argv) == 1)
        # Never inherit debug dumps, tracers or registry output paths that could persist pixels.
        for key in list(os.environ):
            if key.startswith(("GST_", "PIPEWIRE_", "SPA_", "G_MESSAGES_")): del os.environ[key]
        portal = Portal()
        def interrupted(_signal,_frame): raise Rejected()
        signal.signal(signal.SIGTERM, interrupted); signal.signal(signal.SIGINT, interrupted)
        portal.capture()
        return 0
    except Exception:
        sys.stderr.write("wayland_private_capture_unavailable\n")
        return 1
    finally:
        if portal is not None: portal.close()

if __name__ == "__main__": raise SystemExit(main())
