//! Actual fixed subprocess/pipe/lease behavior; every process, profile and observation is synthetic.
use serde_json::json;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use zaturo_computer::{ComputerConfig, approval_digest, capture, execute};
use zaturo_core::{ComputerAction, ComputerBackend, ComputerLease};
fn write(path: &Path, bytes: impl AsRef<[u8]>, mode: u32) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}
fn fixture(root: &Path, backend: ComputerBackend, script: &str) -> ComputerConfig {
    fs::set_permissions(root, fs::Permissions::from_mode(0o700)).unwrap();
    let profile = root.join("profile.toml");
    write(&profile, "enabled=true\n", 0o600);
    let filename = match backend {
        ComputerBackend::Desktop => "linux_mcp.py",
        ComputerBackend::Browser => "browser_mcp.py",
        ComputerBackend::Social => "social_service.py",
        ComputerBackend::Bitlupa => "bitlupa_service.py",
    };
    write(&root.join(filename), script, 0o700);
    ComputerConfig {
        python: "/usr/bin/python3".into(),
        scripts_root: root.into(),
        linux_profile: Some(profile.clone()),
        browser_profile: Some(profile.clone()),
        social_profile: Some(profile),
        bitlupa_project: Some(root.into()),
        bitlupa_database: None,
        capture_target: None,
        wayland_portal_capture: false,
    }
}
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}
fn lease() -> ComputerLease {
    ComputerLease {
        operation_id: "00000000-0000-4000-8000-000000000001".into(),
        lease_token: "synthetic-private-lease-for-computer-adapter".into(),
        lease_expires_at: now() + 30_000,
        deadline_at: now() + 30_000,
        owner_approved: true,
    }
}
fn action(backend: ComputerBackend, tool: &str) -> ComputerAction {
    ComputerAction {
        backend,
        tool: tool.into(),
        arguments: json!({}),
        target: json!({"window_id":1,"pid":1000,"app_id":"synthetic-editor"}),
        expected_snapshot_sha256: None,
    }
}
#[tokio::test]
async fn fixed_native_envelope_has_private_mode_exact_digest_and_no_ambient_credentials() {
    let temp = tempfile::tempdir().unwrap();
    let config = fixture(
        temp.path(),
        ComputerBackend::Desktop,
        r#"import sys,json,os,stat,hashlib
assert sys.argv[1]=='--config' and sys.argv[3]=='action' and sys.argv[4]=='--request-json'
p=sys.argv[5]
assert stat.S_IMODE(os.stat(p).st_mode)==0o600
assert not any(k.startswith(('ZATURO_','OPENCODE_')) for k in os.environ)
r=json.load(open(p));assert r['owner_confirmed'] is True
proof={k:r.get(k) for k in ('tool','arguments','target','expected_snapshot_sha256')}
assert hashlib.sha256(json.dumps(proof,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()).hexdigest()==r['approved_plan_sha256']
print(json.dumps({'schema':1,'status':'completed','tool':r['tool'],'data':{'request_path':p,'nodes':[{'index':1,'name':'Documento fictício'}]},'snapshot_sha256':'a'*64}))
"#,
    );
    let mut request = action(ComputerBackend::Desktop, "get_app_state");
    request.arguments = json!({});
    let result = execute(&config, &request, &lease()).await.unwrap();
    assert_eq!(result.outcome, "completed");
    assert_eq!(result.data["snapshot_sha256"], "a".repeat(64));
    let staged = result.data["data"]["request_path"].as_str().unwrap();
    assert!(!Path::new(staged).exists());
    assert!(
        !serde_json::to_string(&result)
            .unwrap()
            .contains("synthetic-private-lease")
    );
    request.arguments = json!({"b":{"z":"Olá","a":1},"a":"ação"});
    assert_eq!(approval_digest(&request).unwrap().len(), 64);
}
#[tokio::test]
async fn native_dispatch_is_partial_and_a_model_cannot_approve_itself() {
    let temp = tempfile::tempdir().unwrap();
    let config = fixture(
        temp.path(),
        ComputerBackend::Desktop,
        "print('{\"schema\":1,\"status\":\"completed\",\"data\":{\"dispatched\":true,\"verified_external_effect\":false}}')\n",
    );
    let mut request = action(ComputerBackend::Desktop, "click");
    request.arguments = json!({"element_index":2});
    request.expected_snapshot_sha256 = Some("a".repeat(64));
    let mut unapproved = lease();
    unapproved.owner_approved = false;
    assert!(execute(&config, &request, &unapproved).await.is_err());
    let result = execute(&config, &request, &lease()).await.unwrap();
    assert_eq!(result.outcome, "partial");
    assert!(result.text.contains("ainda precisa ser verificado"));
    request.tool = "run_shell".into();
    assert!(execute(&config, &request, &lease()).await.is_err());
}
#[tokio::test]
async fn social_stdin_closes_and_acceptance_is_never_publication() {
    let temp = tempfile::tempdir().unwrap();
    let config = fixture(
        temp.path(),
        ComputerBackend::Social,
        r#"import sys,json
r=json.load(sys.stdin)
assert sys.argv[1]=='execute' and sys.argv[2]=='--config'
assert r['owner_approved'] is True and 'lease_token' not in r
print(json.dumps({'schema':1,'status':'accepted','code':'postiz_accepted','summary_pt':'Pedido aceito.','post_id':'synthetic-post'}))
"#,
    );
    let mut request = action(ComputerBackend::Social, "execute");
    request.target = json!({});
    request.arguments = json!({"operation_id":"00000000-0000-4000-8000-000000000002","approved_draft_hash":"a".repeat(64)});
    let result = execute(&config, &request, &lease()).await.unwrap();
    assert_eq!(result.outcome, "partial");
    assert!(result.text.contains("não confirma"));
    request.arguments["owner_approved"] = json!(true);
    assert!(execute(&config, &request, &lease()).await.is_err());
    request
        .arguments
        .as_object_mut()
        .unwrap()
        .remove("owner_approved");
    write(
        &temp.path().join("social_service.py"),
        "print('{\"schema\":1,\"status\":\"blocked\",\"code\":\"reconciliation_required\",\"summary_pt\":\"Resultado incerto; não reenviar.\"}')\n",
        0o700,
    );
    assert_eq!(
        execute(&config, &request, &lease()).await.unwrap().outcome,
        "uncertain"
    );
}
#[tokio::test]
async fn timeout_and_output_caps_do_not_echo_private_stderr_or_retry_side_effects() {
    let temp = tempfile::tempdir().unwrap();
    let config = fixture(
        temp.path(),
        ComputerBackend::Desktop,
        "import sys,time\nprint('synthetic-private-error-never-echo',file=sys.stderr)\ntime.sleep(60)\n",
    );
    let mut limited = lease();
    limited.lease_expires_at = now() + 200;
    let mut request = action(ComputerBackend::Desktop, "click");
    request.expected_snapshot_sha256 = Some("a".repeat(64));
    let result = execute(&config, &request, &limited).await.unwrap();
    assert_eq!(result.outcome, "uncertain");
    assert!(
        !serde_json::to_string(&result)
            .unwrap()
            .contains("synthetic-private-error")
    );
    write(
        &temp.path().join("linux_mcp.py"),
        "print('x'*40000)\n",
        0o700,
    );
    request.tool = "get_app_state".into();
    let result = execute(&config, &request, &lease()).await.unwrap();
    assert_eq!(result.outcome, "blocked");
    assert_eq!(result.data["code"], "adapter_output_limit");
}
#[tokio::test]
async fn dropping_execution_kills_only_the_owned_helper_process_group() {
    let temp = tempfile::tempdir().unwrap();
    let record = temp.path().join("owned-pids.json");
    let script = format!(
        "import subprocess,sys,os,json,time\np=subprocess.Popen([sys.executable,'-c','import time;time.sleep(60)'])\njson.dump([os.getpid(),p.pid],open({:?},'w'))\ntime.sleep(60)\n",
        record.to_string_lossy()
    );
    let config = fixture(temp.path(), ComputerBackend::Desktop, &script);
    let request = action(ComputerBackend::Desktop, "get_app_state");
    let grant = lease();
    let running = tokio::spawn(async move { execute(&config, &request, &grant).await });
    tokio::time::timeout(Duration::from_secs(3), async {
        while !record.exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let pids: Vec<u32> = serde_json::from_slice(&fs::read(record).unwrap()).unwrap();
    running.abort();
    let _ = running.await;
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let active = pids.iter().any(|pid| {
                fs::read_to_string(format!("/proc/{pid}/stat")).is_ok_and(|s| {
                    s.rsplit_once(") ")
                        .is_some_and(|(_, r)| !r.starts_with('Z') && !r.starts_with('X'))
                })
            });
            if !active {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn private_capture_is_raw_zeroizing_png_without_a_plaintext_file() {
    let temp = tempfile::tempdir().unwrap();
    let mut config = fixture(
        temp.path(),
        ComputerBackend::Desktop,
        r#"import sys,json,struct,zlib
assert sys.argv[3]=='capture'
r=json.load(open(sys.argv[5]));assert r['tool']=='screenshot' and r['owner_confirmed']
def chunk(tag,data):return struct.pack('>I',len(data))+tag+data+struct.pack('>I',zlib.crc32(tag+data)&0xffffffff)
png=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',2,2,8,6,0,0,0))+chunk(b'IDAT',zlib.compress((b'\0'+b'\xff\xdd\0\xff'*2)*2))+chunk(b'IEND',b'')
sys.stdout.buffer.write(png)
"#,
    );
    let before = fs::read_dir(temp.path()).unwrap().count();
    let target = action(ComputerBackend::Desktop, "get_app_state").target;
    config.capture_target = Some(target.clone());
    let png = capture(&config, &lease(), target).await.unwrap();
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert_eq!(before, fs::read_dir(temp.path()).unwrap().count());
    let mut denied = lease();
    denied.owner_approved = false;
    assert!(capture(&config, &denied, json!({})).await.is_err());
}
#[tokio::test]
async fn stored_price_results_format_observed_brl_variant_links_and_caveats() {
    let temp = tempfile::tempdir().unwrap();
    let raw = json!({"schema":1,"status":"completed","summary_pt":"Ofertas observadas.","live_collection":false,"lowest_observed_offer_id":1,"best_value_offer_id":null,"offers":[{"id":1,"product_id":1,"brand":"Xiaomi","model":"POCO X7 Pro","storage_gb":512,"ram_gb":12,"source_id":"kabum","source_name":"KaBuM!","price_cents":210099,"price_kind":"pix","currency":"BRL","availability":"in_stock","url":"https://www.kabum.com.br/produto/synthetic","collected_at":"2026-11-14T12:00:00Z","freshness":"fresh"}]});
    let script = format!("print({:?})\n", serde_json::to_string(&raw).unwrap());
    let config = fixture(temp.path(), ComputerBackend::Bitlupa, &script);
    let mut request = action(ComputerBackend::Bitlupa, "search");
    request.arguments = json!({"query":"POCO X7 Pro 512GB"});
    request.target = json!({});
    let result = execute(&config, &request, &lease()).await.unwrap();
    assert_eq!(result.outcome, "completed", "{}", result.text);
    assert!(result.text.contains("R$ 2.100,99"));
    assert!(result.text.contains("512 GB / 12 GB RAM"));
    assert!(
        result
            .text
            .contains("https://www.kabum.com.br/produto/synthetic")
    );
    assert!(
        result
            .text
            .contains("Melhor custo-benefício total não comprovado")
    );
    assert!(result.text.len() <= 2048);
}

#[tokio::test]
async fn helper_cannot_leak_lease_material_or_change_the_locally_bound_capture_target() {
    let temp = tempfile::tempdir().unwrap();
    let mut config = fixture(
        temp.path(),
        ComputerBackend::Desktop,
        r#"import sys,json
r=json.load(open(sys.argv[5]))
print(json.dumps({'schema':1,'status':'completed','data':{'untrusted_text':r['lease_token']}}))
"#,
    );
    let grant = lease();
    let request = action(ComputerBackend::Desktop, "get_app_state");
    let observed = execute(&config, &request, &grant).await.unwrap();
    assert_eq!(observed.outcome, "blocked");
    assert!(
        !serde_json::to_string(&observed)
            .unwrap()
            .contains(&grant.lease_token)
    );
    config.capture_target = Some(request.target);
    assert!(
        capture(
            &config,
            &grant,
            json!({"window_id":2,"pid":2000,"app_id":"other-application"})
        )
        .await
        .is_err()
    );
}
