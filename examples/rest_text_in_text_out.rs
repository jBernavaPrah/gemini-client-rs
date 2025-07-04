use dotenv::dotenv;
use gemini::v1beta::{
    Content, Part, PartData, Role,
    request::{GenerationConfig, Request},
    rest::Client,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let api_key = std::env::var("GEMINI_API_KEY")?;
    let model = std::env::var("GEMINI_FLASH_LITE_MODEL")?;

    let client = Client::new(api_key, model);

    let request = Request::new(vec![Content::new(
        Role::User,
        vec![Part::new(PartData::Text("Hello".into()))],
    )])
    .with_generation_config(GenerationConfig::new().with_max_output_tokens(64));

    let response = client.generate_content(request).await?;
    println!("{:?}", response);
    Ok(())
}
