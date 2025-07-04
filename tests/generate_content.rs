use gemini::v1beta::{
    self, Content, Part, PartData, Role, request,
    rest::{Client, Error},
};
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

async fn start_server(
    body: &'static [u8],
    status: &'static str,
) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let status_line = status.to_string();
    let handle = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 1024];
        let _ = stream.read(&mut buf).await;
        let headers = format!(
            "HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
            body.len()
        );
        stream.write_all(headers.as_bytes()).await.unwrap();
        stream.write_all(body).await.unwrap();
    });
    (addr, handle)
}

#[tokio::test]
async fn generate_content_success() {
    let body = b"{\"candidates\": [{\"content\": {\"parts\": [{\"text\": \"hi\"}], \"role\": \"model\"}}]}";
    let (addr, handle) = start_server(body, "200 OK").await;

    let client = Client::new("key", "test").with_api_base(format!("http://{}/v1beta/models", addr));
    let req = request::Request::new(vec![Content::new(
        Role::User,
        vec![Part::new(PartData::Text("hi".into()))],
    )]);

    let resp = client.generate_content(req).await.expect("ok");
    handle.abort();
    assert_eq!(resp.candidates.len(), 1);
}

#[tokio::test]
async fn generate_content_error_status() {
    let (addr, handle) = start_server(b"bad", "400 BAD REQUEST").await;
    let client = Client::new("key", "test").with_api_base(format!("http://{}/v1beta/models", addr));
    let req = request::Request::new(vec![]);

    let err = client.generate_content(req).await.unwrap_err();
    handle.abort();
    match err {
        Error::ApiError(_) => {}
        other => panic!("unexpected error: {:?}", other),
    }
}

#[tokio::test]
async fn generate_content_invalid_json() {
    let (addr, handle) = start_server(b"invalid", "200 OK").await;
    let client = Client::new("key", "test").with_api_base(format!("http://{}/v1beta/models", addr));
    let req = request::Request::new(vec![]);

    let err = client.generate_content(req).await.unwrap_err();
    handle.abort();
    match err {
        Error::Reqwest(_) => {}
        other => panic!("unexpected error: {:?}", other),
    }
}
