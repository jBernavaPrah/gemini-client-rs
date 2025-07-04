//! Rust client for Google's Gemini APIs.
//!
//! This crate provides strongly typed wrappers over the public **v1beta** endpoints.
//! Use [`v1beta::rest::Client`] for request/response style interactions over HTTP
//! or [`v1beta::live::Client`] for the websocket based streaming API.
//!
//! ## Example
//!
//! ```no_run
//! use gemini::v1beta::{rest::Client, request::{Request, GenerationConfig}, Content, Part, PartData, Role};
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let client = Client::new("API_KEY", "models/gemini-2.0-pro");
//! let request = Request::new(vec![Content::new(
//!     Role::User,
//!     vec![Part::new(PartData::Text("Hello".into()))],
//! )])
//! .with_generation_config(GenerationConfig::new().with_max_output_tokens(64));
//!
//! let response = client.generate_content(request).await?;
//! println!("{:?}", response);
//! # Ok(())
//! # }
//! ```
//!
//! See the [`examples`](../examples) directory for additional programs demonstrating streaming and audio usage.

pub mod v1beta;
