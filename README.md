# Gemini Client

This crate provides minimal clients for Google's Gemini APIs.

 - `gemini::v1beta::rest::Client` – asynchronous REST client for the HTTP API.
 - `gemini::v1beta::live::Client` – websocket client for the live streaming API.

Use the REST client when you just need a request/response interaction. Use the
live client for real-time streaming of text or audio.

```rust
use gemini::v1beta::{rest::Client, request::{Request, GenerationConfig}};

let client = Client::new("API_KEY", "models/gemini-2.0-pro");
let request = Request::new(vec![]).with_generation_config(GenerationConfig::new());
let response = client.generate_content(request).await?;
```

```rust
use gemini::v1beta::live::{Client, Setup};

let setup = Setup::new("models/gemini-2.0-pro");
let (client, mut stream) = Client::connect("API_KEY", setup).await?;
```

See the example programs under [`examples/`](./examples) for full usage.
`rest_text_in_text_out.rs` shows a simple REST request, while the other
examples demonstrate the streaming API.
