use dotenv::dotenv;
use gemini::v1beta::live::{
    Client, ClientMessage, GenerationConfig, InlineData, RealtimeInput, ResponseModality,
    ServerMessage, Setup,
};
use tokio_stream::StreamExt;
use tracing::info;

#[path = "common/utils.rs"]
mod utils;
use utils::{OutputAudioConfig, listen_from_default_input};

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

    let setup = Setup::new(format!("models/{model}")).with_generation_config(
        GenerationConfig::default().with_response_modalities(vec![ResponseModality::Text]),
    );

    let (client, mut stream) = Client::connect(&api_key, setup.clone()).await?;

    let (mut audio_stream, _input) = listen_from_default_input(OutputAudioConfig {
        sample_rate: 16_000,
        channels: 1,
        bits_per_sample: 16,
        batch_size: 1920,
    })
    .await
    .expect("input");

    let client2 = client.clone();
    tokio::spawn(async move {
        while let Some(audio) = audio_stream.next().await {
            let _ = client2.call(ClientMessage::RealtimeInput(RealtimeInput::Audio(
                InlineData::new("audio/pcm;rate=16000", audio),
            )));
        }
    });

    while let Some(msg) = stream.next().await {
        info!("message: {:?}", msg);
    }

    client.disconnect(None)?;
    Ok(())
}
