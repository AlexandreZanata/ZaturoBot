# Android application testing

Status: planned integration beyond the USB foundation. An attached physical phone can
become an opted-in test target after the owner registers its exact serial and separately
authorizes package installation/testing. The current implementation does not build,
install, launch, clear data or uninstall Android applications.

The future workflow reads the selected authorized repository's instructions, discovers
its existing Gradle/test tasks, builds in an isolated workspace, validates the resulting
APK/package, then obtains the applicable install/test grant. Structured UI Automator or
the project's existing instrumentation tests should provide reproducible evidence;
screenshots supplement semantic observations when permitted. The phone is not assumed
to have a specific app, a particular Android version or an unlocked test profile.

Keep install, uninstall and data-clearing capabilities separate. Do not grant those
operations merely because a repository contains a command or a model proposes it. Real
personal application data must not be erased as part of a generic test reset. Test APKs
must come from the opted-in build, not an arbitrary downloaded URL.

CI uses fake ADB and synthetic screens. Real-device acceptance remains opt-in, requires
an explicit registered serial, and records OS/API/package/build/test identifiers without
personal messages or authentication material. See official
[UI Automator](https://developer.android.com/training/testing/other-components/ui-automator)
and [instrumented tests](https://developer.android.com/training/testing/instrumented-tests).
