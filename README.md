# Gemini Client

A lightweight Rust client for Google's Gemini Generative AI APIs.

This crate exposes both HTTP and WebSocket interfaces to the public **v1beta** endpoints. It is async-first and builds on `tokio`, `reqwest` and `ezsockets`.

## Features

- **REST Client** – [`gemini::v1beta::rest::Client`](src/v1beta/rest.rs) supports `generateContent` and `streamGenerateContent` requests over HTTP.
- **Live Client** – [`gemini::v1beta::live::Client`](src/v1beta/live.rs) provides a WebSocket connection for real‑time streaming of text or audio.

## Installation

Add the crate to your `Cargo.toml`. Until a crates.io release is available you can depend on the repository directly:

```toml
# Cargo.toml
[dependencies]
gemini-client-rs = { git = "https://github.com/yourname/gemini-client-rs" }
```

## REST example

```rust
use gemini::v1beta::{rest::Client, request::{Request, GenerationConfig}, Content, Part, PartData, Role};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new("API_KEY", "models/gemini-2.0-pro");
    let request = Request::new(vec![Content::new(
        Role::User,
        vec![Part::new(PartData::Text("Hello".into()))],
    )])
    .with_generation_config(GenerationConfig::new().with_max_output_tokens(64));

    let response = client.generate_content(request).await?;
    println!("{:?}", response);
    Ok(())
}
```

## Live example

```rust
use gemini::v1beta::live::{Client, Setup};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let setup = Setup::new("models/gemini-2.0-pro");
    let (client, mut stream) = Client::connect("API_KEY", setup).await?;
    // handle messages from `stream`...
    Ok(())
}
```

See the programs under [`examples/`](./examples) for complete usage including streaming audio.

## License

This project is licensed under the MIT license.
