use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use zaturo_desktop::{api::HubClient, config::DesktopConfig, image::received_png};
const TOKEN: &str = "synthetic-native-client-token-for-local-tests";
#[tokio::test]
async fn actual_http_bounds_filters_and_hides_sensitive_items() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = vec![0; 16384];
        let count = socket.read(&mut request).await.unwrap();
        let headers = String::from_utf8_lossy(&request[..count]);
        assert!(
            headers
                .to_ascii_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}"))
        );
        assert!(headers.contains("q=x%26offset%3D99"));
        let body=serde_json::json!({"items":[{"id":"one","created_at":1000,"kind":"message","role":"owner","text":"a permitted request","task_id":null,"artifact_id":null,"sensitive":false},{"id":"two","created_at":1000,"kind":"android","role":"system","text":"synthetic sensitive-screen fixture","task_id":null,"artifact_id":null,"sensitive":true}],"has_more":false}).to_string();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        socket.write_all(response.as_bytes()).await.unwrap();
    });
    let dir = tempfile::tempdir().unwrap();
    let mut config = DesktopConfig::defaults(dir.path());
    config.hub_url = format!("http://{address}");
    let client = HubClient::new(&config, TOKEN).unwrap();
    let page = client
        .feed("2026-11-13", -240, "message", "x&offset=99", 0)
        .await
        .unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].id, "one");
    server.await.unwrap();
}
#[test]
fn previews_only_private_received_png_with_bounded_dimensions() {
    let root = tempfile::tempdir().unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    let folder = root.path().join(&id);
    fs::create_dir(&folder).unwrap();
    fs::set_permissions(&folder, fs::Permissions::from_mode(0o700)).unwrap();
    let path = folder.join("received.png");
    let mut header = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    header.extend_from_slice(&1u32.to_be_bytes());
    header.extend_from_slice(&1u32.to_be_bytes());
    fs::write(&path, &header).unwrap();
    assert_eq!(received_png(root.path(), &id).unwrap(), path);
    header[16..20].copy_from_slice(&99999u32.to_be_bytes());
    fs::write(&path, &header).unwrap();
    assert!(received_png(root.path(), &id).is_err());
    fs::remove_file(&path).unwrap();
    let external = root.path().join("private.txt");
    fs::write(&external, b"private data").unwrap();
    symlink(external, &path).unwrap();
    assert!(received_png(root.path(), &id).is_err());
    assert!(received_png(root.path(), "../private").is_err());
}

#[test]
fn private_png_decodes_in_memory_and_enforces_size_limits() {
    let pixels = image::RgbaImage::from_pixel(2, 2, image::Rgba([20, 30, 40, 255]));
    let mut encoded = std::io::Cursor::new(Vec::new());
    pixels
        .write_to(&mut encoded, image::ImageFormat::Png)
        .unwrap();
    let decoded = zaturo_desktop::private_view::decode_png(encoded.get_ref()).unwrap();
    assert_eq!((decoded.width, decoded.height), (2, 2));
    assert_eq!(decoded.bytes.len(), 16);
    assert!(zaturo_desktop::private_view::decode_png(b"not a PNG").is_err());
    assert!(zaturo_desktop::private_view::decode_png(&vec![0; 8 * 1024 * 1024 + 1]).is_err());
}
