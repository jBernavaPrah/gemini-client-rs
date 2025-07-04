//! Live text in/text out example.
//! Connects to the streaming API and echoes Gemini's text responses for your typed input.

use dotenv::dotenv;
use gemini::v1beta::live::{
    Client, ClientContent, ClientMessage, Content, GenerationConfig, Part, PartData,
    ResponseModality, Role, ServerMessage, Setup,
};
use tokio_stream::StreamExt;
use tracing::info;

#[path = "common/utils.rs"]
mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenv().ok();

    if std::env::var("RUST_LOG").is_err() {
        unsafe {
            std::env::set_var("RUST_LOG", "debug");
        }
    }
    tracing_subscriber::fmt::init();

    let api_key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not set");
    let model = std::env::var("GEMINI_LIVE_MODEL").expect("GEMINI_LIVE_MODEL not set");

    let setup = Setup::new(format!("models/{model}"))
        .with_system_instruction(Content::new(
            None,
            vec![Part::new(PartData::Text(String::from(
                "You are a helpful assistant.",
            )))],
        ))
        .with_generation_config(
            GenerationConfig::default()
                .with_max_output_tokens(64)
                .with_response_modalities(vec![ResponseModality::Text]),
        );

    let (client, mut stream) = Client::connect(&api_key, setup.clone()).await?;

    let _client = client.clone();
    tokio::spawn(async move {
        let mut stream_in = utils::stdin_stream();

        while let Some(line) = stream_in.next().await {
            if _client
                .call(ClientMessage::ClientContent(
                    ClientContent::new(vec![Content::new(
                        Role::User,
                        vec![Part::new(PartData::Text(line))],
                    )])
                    .is_turn_completed(true),
                ))
                .is_err()
            {
                info!("error sending message");
                break;
            }
        }
    });

    while let Some(msg) = stream.next().await {
        info!("message: {:?}", msg);
    }

    client.disconnect(None)?;
    Ok(())
}
