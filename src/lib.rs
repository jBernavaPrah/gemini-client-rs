//! Clients for Google's Gemini APIs.
//!
//! Use [`v1beta::rest::Client`] for simple REST interactions over HTTP.
//! Use [`v1beta::live::Client`] when you need the websocket streaming API.

pub mod v1beta;
