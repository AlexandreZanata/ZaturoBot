//! Synthetic USB metadata/PNG fixtures and fake local ADB processes only.
use std::{fs, os::unix::fs::PermissionsExt, path::Path, sync::Arc, time::Duration};
use zaturo_android::{AdbClient, AdbLimits, Error, SessionManager, parse_devices};
use zaturo_core::{
    AndroidAuthorization, AndroidCapability, AndroidControlActor, AndroidDeviceState,
    AndroidTransport, AndroidWatchRequest, RegisteredAndroidDevice,
};
// These fixtures create executable inodes. Another test's fork can briefly inherit a writer FD
// before CLOEXEC runs, making execve fail with Linux ETXTBSY even after the creator closes it.
// Serialize fixture lifecycles only; takeover and cancellation still run concurrent tasks below.
static EXECUTABLE_FIXTURES: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
fn registration() -> RegisteredAndroidDevice {
    RegisteredAndroidDevice {
        device_id: "owner-phone".into(),
        adb_serial: "USB_SYNTHETIC_01".into(),
        display_name: Some("Synthetic phone".into()),
        capabilities: vec![
            AndroidCapability::DeviceStatus,
            AndroidCapability::SensitiveScreenView,
        ],
    }
}
fn fake(path: &Path, script: &str) -> std::path::PathBuf {
    let file = path.join("adb-fake");
    fs::write(&file, format!("#!/usr/bin/python3\n{script}")).unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o700)).unwrap();
    file
}
const FIXTURE: &str = r#"import sys,os,json,zlib,struct
from pathlib import Path
root=Path(__file__).parent
args=sys.argv[1:]
assert args[:4]==['-H','127.0.0.1','-P','5037']
assert 'ADB_SERVER_SOCKET' not in os.environ and 'ANDROID_SERIAL' not in os.environ
args=args[4:]
with (root/'arguments.jsonl').open('a') as log:log.write(json.dumps(args)+'\n')
if args==['devices','-l']:
 print('List of devices attached\nUSB_SYNTHETIC_01\tdevice usb:1-2 product:test model:Fake_Phone transport_id:1\nOTHER_USB\tunauthorized usb:1-3 transport_id:2')
elif args[:3]==['-s','USB_SYNTHETIC_01','shell']:
 assert args[3]=='getprop'
 print({'ro.product.manufacturer':'Synthetic','ro.product.model':'Fixture Phone','ro.build.version.release':'15','ro.build.version.sdk':'35'}[args[4]])
elif args==['-s','USB_SYNTHETIC_01','exec-out','screencap','-p']:
 count=int((root/'captures').read_text()) if (root/'captures').exists() else 0
 (root/'captures').write_text(str(count+1))
 shade=255 if (root/'change').exists() and count else 0
 def chunk(kind,data):return struct.pack('!I',len(data))+kind+data+struct.pack('!I',zlib.crc32(kind+data)&0xffffffff)
 png=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('!IIBBBBB',1,1,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(bytes([0,shade,0,0,255])))+chunk(b'IEND',b'')
 sys.stdout.buffer.write(png)
else:raise RuntimeError('unsupported fixed operation')
"#;
#[test]
fn typed_discovery_handles_authorization_multiple_transports_and_bad_input() {
    let devices=parse_devices("List of devices attached\nUSB_1\tdevice usb:1-2 model:Fake_Phone\nUSB_2\tunauthorized\nUSB_3\toffline\nUSB_4\tno permissions (missing udev rules)\n127.0.0.1:5555\tdevice model:Network\nemulator-5554\tdevice",1000).unwrap();
    assert_eq!(devices.len(), 6);
    assert_eq!(devices[0].model.as_deref(), Some("Fake Phone"));
    assert_eq!(devices[0].transport, AndroidTransport::Usb);
    assert_eq!(devices[1].state, AndroidDeviceState::Unauthorized);
    assert_eq!(devices[2].state, AndroidDeviceState::Connecting);
    assert_eq!(
        devices[3].authorization,
        AndroidAuthorization::UsbPermissionDenied
    );
    assert_eq!(devices[4].transport, AndroidTransport::Network);
    assert_eq!(devices[5].transport, AndroidTransport::Emulator);
    assert!(parse_devices("unexpected device output", 1).is_err());
    assert!(parse_devices("List of devices attached\nUSB_1 device\nUSB_1 device", 1).is_err());
    assert!(parse_devices("List of devices attached\nserial;command device", 1).is_err());
    assert!(
        parse_devices(
            &format!(
                "List of devices attached\n{}",
                (0..33)
                    .map(|n| format!("USB_{n} device\n"))
                    .collect::<String>()
            ),
            1
        )
        .is_err()
    );
}
#[test]
fn leases_are_exclusive_expiring_revocable_and_bound_to_identity() {
    let device = registration();
    let sessions = SessionManager::new(vec![device.clone()]).unwrap();
    let agent = sessions
        .acquire(&device.device_id, AndroidControlActor::Agent, 10, 1000)
        .unwrap();
    assert!(agent.authorize_sensitive(&device, 1001).is_err());
    assert!(matches!(
        sessions.acquire(&device.device_id, AndroidControlActor::Owner, 10, 1001),
        Err(Error::Busy)
    ));
    let owner = sessions
        .owner_takeover(&device.device_id, 10, 1001)
        .unwrap();
    assert!(sessions.release(&agent).is_err());
    owner.authorize_sensitive(&device, 1002).unwrap();
    let mut other = device.clone();
    other.adb_serial = "OTHER_USB".into();
    assert!(owner.authorize_sensitive(&other, 1002).is_err());
    assert!(owner.authorize_sensitive(&device, 11001).is_err());
    sessions.release(&owner).unwrap();
    assert!(owner.authorize_sensitive(&device, 1003).is_err());
    assert!(sessions.active(&device.device_id, 1003).unwrap().is_none());
    let mut duplicate = device.clone();
    duplicate.device_id = "other-phone".into();
    assert!(SessionManager::new(vec![device, duplicate]).is_err());
}
#[tokio::test]
async fn fixed_commands_metadata_private_capture_and_visual_change_watch() {
    let _fixture = EXECUTABLE_FIXTURES.lock().await;
    let dir = tempfile::tempdir().unwrap();
    let path = fake(dir.path(), FIXTURE);
    let adb = AdbClient::new(&path).unwrap();
    let device = registration();
    let snapshot = adb.inspect(&device, 1000).await.unwrap();
    assert_eq!(snapshot.model.as_deref(), Some("Fixture Phone"));
    assert_eq!(snapshot.api_level, Some(35));
    let manager = SessionManager::new(vec![device.clone()]).unwrap();
    let lease = manager
        .acquire(&device.device_id, AndroidControlActor::Owner, 30, 1000)
        .unwrap();
    let frame = adb.capture(&device, &lease, 1000).await.unwrap();
    assert_eq!(frame.dimensions(), (1, 1));
    assert!(frame.bytes().starts_with(b"\x89PNG"));
    drop(frame);
    fs::remove_file(dir.path().join("captures")).unwrap();
    fs::write(dir.path().join("change"), b"enabled").unwrap();
    let changed = adb
        .watch(
            &device,
            &lease,
            &AndroidWatchRequest {
                duration_seconds: 5,
                poll_seconds: 2,
            },
            1000,
        )
        .await
        .unwrap();
    assert!(changed.is_some());
    let invocations = fs::read_to_string(dir.path().join("arguments.jsonl")).unwrap();
    assert!(!invocations.contains("UNAUTHORIZED"));
    assert!(!invocations.contains("OTHER_USB"));
    assert!(!invocations.contains("content://"));
    assert!(
        dir.path().read_dir().unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".png")),
        "images remain in memory"
    );
}
#[tokio::test]
async fn timeout_output_caps_and_opaque_errors_do_not_echo_device_data() {
    let _fixture = EXECUTABLE_FIXTURES.lock().await;
    let dir = tempfile::tempdir().unwrap();
    let slow = fake(dir.path(), "import time\ntime.sleep(30)\n");
    let client = AdbClient::with_limits(
        &slow,
        AdbLimits {
            command_timeout: Duration::from_millis(50),
            ..Default::default()
        },
    )
    .unwrap();
    let start = tokio::time::Instant::now();
    assert_eq!(client.discover(1).await.unwrap_err(), Error::Timeout);
    assert!(start.elapsed() < Duration::from_secs(2));
    fake(dir.path(), "import sys\nsys.stdout.write('x'*10000)\n");
    let client = AdbClient::with_limits(
        &slow,
        AdbLimits {
            max_text_bytes: 64,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(client.discover(1).await.unwrap_err(), Error::OutputLimit);
    fake(
        dir.path(),
        "import sys\nsys.stderr.write('SYNTHETIC_SENSITIVE_SCREEN_DATA')\nsys.exit(1)\n",
    );
    let client = AdbClient::new(&slow).unwrap();
    let error = client.discover(1).await.unwrap_err();
    assert_eq!(error, Error::CommandFailed);
    assert!(!format!("{error:?} {error}").contains("SYNTHETIC_SENSITIVE_SCREEN_DATA"));
}
#[tokio::test]
async fn owner_takeover_cancels_an_inflight_capture_without_releasing_pixels() {
    let _fixture = EXECUTABLE_FIXTURES.lock().await;
    let dir = tempfile::tempdir().unwrap();
    let code = FIXTURE.replace("count=int(", "import time\n time.sleep(30)\n count=int(");
    let executable = fake(dir.path(), &code);
    let client = Arc::new(AdbClient::new(&executable).unwrap());
    let device = registration();
    let sessions = SessionManager::new(vec![device.clone()]).unwrap();
    let lease = sessions
        .acquire(&device.device_id, AndroidControlActor::Owner, 60, 1000)
        .unwrap();
    let task = {
        let adb = client.clone();
        let device = device.clone();
        tokio::spawn(async move { adb.capture(&device, &lease, 1000).await.map(|_| ()) })
    };
    tokio::time::sleep(Duration::from_millis(150)).await;
    sessions
        .owner_takeover(&device.device_id, 60, 1200)
        .unwrap();
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(1), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap_err(),
        Error::PermissionDenied
    );
}
#[tokio::test]
async fn disconnected_unauthorized_and_unknown_transport_are_not_controllable() {
    let _fixture = EXECUTABLE_FIXTURES.lock().await;
    let dir = tempfile::tempdir().unwrap();
    let device = registration();
    let sessions = SessionManager::new(vec![device.clone()]).unwrap();
    let lease = sessions
        .acquire(&device.device_id, AndroidControlActor::Owner, 60, 1000)
        .unwrap();
    for (line, expected) in [
        ("OTHER_USB device usb:1-3", Error::Disconnected),
        ("USB_SYNTHETIC_01 unauthorized", Error::Unauthorized),
        ("USB_SYNTHETIC_01 device", Error::PermissionDenied),
    ] {
        let code = format!("print({:?})\n", format!("List of devices attached\n{line}"));
        let executable = fake(dir.path(), &code);
        let client = AdbClient::new(&executable).unwrap();
        assert_eq!(
            client.capture(&device, &lease, 1000).await.err(),
            Some(expected)
        );
    }
}

#[tokio::test]
async fn cancellation_and_timeout_kill_owned_descendant_processes() {
    let _fixture = EXECUTABLE_FIXTURES.lock().await;
    fn running(pid: u32) -> bool {
        fs::read_to_string(format!("/proc/{pid}/stat"))
            .ok()
            .and_then(|s| s.rsplit_once(") ").map(|(_, tail)| tail.to_owned()))
            .is_some_and(|tail| !tail.starts_with('Z') && !tail.starts_with('X'))
    }
    for abort in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let executable = fake(
            dir.path(),
            r#"import os,json,time,subprocess
from pathlib import Path
child=subprocess.Popen(['/bin/sleep','30'])
pending=Path(__file__).with_name('pids.pending')
pending.write_text(json.dumps([os.getpid(),child.pid]))
pending.replace(Path(__file__).with_name('pids.json'))
time.sleep(30)
"#,
        );
        let client = AdbClient::with_limits(
            &executable,
            AdbLimits {
                command_timeout: if abort {
                    Duration::from_secs(10)
                } else {
                    Duration::from_millis(250)
                },
                ..Default::default()
            },
        )
        .unwrap();
        let task = tokio::spawn(async move { client.discover(1).await });
        let pidfile = dir.path().join("pids.json");
        tokio::time::timeout(Duration::from_secs(2), async {
            while !pidfile.exists() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        let pids: Vec<u32> = fixture_pids(&fs::read_to_string(pidfile).unwrap());
        if abort {
            task.abort();
            let _ = task.await;
        } else {
            assert_eq!(task.await.unwrap().err(), Some(Error::Timeout));
        }
        tokio::time::timeout(Duration::from_secs(2), async {
            while pids.iter().any(|p| running(*p)) {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("owned ADB process group survived cancellation");
    }
}
// The fixture writes only two numeric PIDs; avoid adding a JSON dependency for it.
fn fixture_pids(value: &str) -> Vec<u32> {
    value
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(|s| s.trim().parse().unwrap())
        .collect()
}

#[tokio::test]
async fn process_start_failures_report_safe_os_kind_without_private_paths_or_output() {
    let _fixture = EXECUTABLE_FIXTURES.lock().await;
    let dir = tempfile::tempdir().unwrap();
    let executable = fake(dir.path(), "print('SYNTHETIC_PRIVATE_DEVICE_CONTENT')\n");
    let client = AdbClient::new(&executable).unwrap();
    fs::remove_file(&executable).unwrap();
    let error = client.discover(1).await.unwrap_err();
    assert!(matches!(
        error,
        Error::SpawnFailed {
            kind: std::io::ErrorKind::NotFound,
            os_code: Some(2)
        }
    ));
    let diagnostic = format!("{error:?} {error}");
    assert!(!diagnostic.contains(&dir.path().to_string_lossy().to_string()));
    assert!(!diagnostic.contains("SYNTHETIC_PRIVATE_DEVICE_CONTENT"));
}

#[tokio::test]
async fn unchanged_screen_watch_ends_without_an_image() {
    let _fixture = EXECUTABLE_FIXTURES.lock().await;
    let dir = tempfile::tempdir().unwrap();
    let executable = fake(dir.path(), FIXTURE);
    let client = AdbClient::new(&executable).unwrap();
    let device = registration();
    let sessions = SessionManager::new(vec![device.clone()]).unwrap();
    let lease = sessions
        .acquire(&device.device_id, AndroidControlActor::Owner, 30, 1000)
        .unwrap();
    let result = tokio::time::timeout(
        Duration::from_secs(7),
        client.watch(
            &device,
            &lease,
            &AndroidWatchRequest {
                duration_seconds: 5,
                poll_seconds: 2,
            },
            1000,
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(result.is_none());
    assert!(dir.path().read_dir().unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".png")
    }));
}
