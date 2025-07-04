use gemini::v1beta::{self, Content, Part, PartData, Role, request, rest::Client};
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_stream::StreamExt;

async fn start_server(chunks: Vec<&'static [u8]>) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 1024];
        // Read request, ignore contents
        let _ = stream.read(&mut buf).await;
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\n")
            .await
            .unwrap();
        for chunk in chunks {
            stream.write_all(chunk).await.unwrap();
            stream.flush().await.unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    });
    (addr, handle)
}

#[tokio::test]
async fn stream_content_parses_sse() {
    let (addr, handle) = start_server(vec![
        b"data: {\"candidates\": []}\n\n",
        b"data: {\"usageMetadata\": {}}\n\n",
    ])
    .await;

    let client = Client::new("key", "test").with_api_base(format!("http://{}/v1beta/models", addr));
    let req = request::Request::new(vec![Content::new(
        Role::User,
        vec![Part::new(PartData::Text("hi".into()))],
    )]);

    let mut stream = client.stream_content(req).await.expect("stream");
    let mut items = Vec::new();
    while let Some(item) = stream.next().await {
        items.push(item.unwrap());
    }
    handle.abort();
    assert_eq!(items.len(), 2);
    assert!(items[0].usage_metadata.is_none());
    assert!(items[1].usage_metadata.is_some());
}

#[tokio::test]
async fn stream_content_invalid_json() {
    let (addr, handle) = start_server(vec![b"data: invalid\n\n"]).await;
    let client = Client::new("key", "test").with_api_base(format!("http://{}/v1beta/models", addr));
    let req = request::Request::new(vec![]);
    let mut stream = client.stream_content(req).await.expect("stream");
    let res = stream.next().await.expect("item");
    handle.abort();
    assert!(res.is_err());
}
