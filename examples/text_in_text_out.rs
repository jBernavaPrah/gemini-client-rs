use dotenv::dotenv;
use gemini::v1beta::live::{
    Client, ClientContent, ClientMessage, Content, FunctionBehavior, FunctionCall,
    FunctionDeclaration, FunctionResponse, FunctionResponseScheduling, FunctionResult,
    GenerationConfig, Part, PartData, ResponseModality, Role, ServerContent, ServerMessage, Setup,
    Tool, ToolResponse,
};
use serde_json::json;
use tokio_stream::StreamExt;
use tracing::info;

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
        .with_system_instruction(Content::new(None, vec![Part::new(PartData::Text(String::from("You are a helpful assistant. Your name is Alice.")))]))
        .with_tools(vec![Tool::FunctionDeclarations(
            vec![
                FunctionDeclaration::new("end_conversation")
                    .with_description("Use this tool to end the conversation after you have addressed all the user's inquiries.
Important: The tool will end the conversation immediately between you and the user.")
                    .with_behavior(FunctionBehavior::NonBlocking),
                FunctionDeclaration::new("time")
                    .with_description("Anytime you need to know the current time, use this tool.")
                    .with_behavior(FunctionBehavior::NonBlocking)
                ,
                FunctionDeclaration::new("test_function")
                    .with_description("Call this function when prompted to do. You will receive a response with the status of the function. If status is `completed`, tell to the user tha the test function is completed. If any other status, tell to the user that the test function is still running. ")
                    .with_behavior(FunctionBehavior::NonBlocking),
            ]
        )])

        .with_generation_config(
        GenerationConfig::default()
            .with_max_output_tokens(64)
            .with_response_modalities(vec![ResponseModality::Text]),
    );

    let (client, mut stream) = Client::connect(&api_key, setup.clone()).await?;

    let _client = client.clone();
    tokio::spawn(async move {
        let mut stream = utils::stdin_stream();

        while let Some(line) = stream.next().await {
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

    let _ = client.call(ClientMessage::ClientContent(
        ClientContent::new(vec![Content::new(
            Role::User,
            vec![Part::new(PartData::Text(
                "Could you execute the test_function?".to_string(),
            ))],
        )])
        .is_turn_completed(true),
    ));

    // let _ = client.call(ClientMessage::ClientContent(
    //     ClientContent::new(vec![Content::new(
    //         Role::Model,
    //         vec![
    //             Part::new(PartData::FunctionCall(FunctionCall::new(
    //                 "call1".to_string(),
    //                 "test_function",
    //                 json!({}),
    //             ))),
    //             Part::new(PartData::FunctionResponse(
    //                 FunctionResponse::new(
    //                     "call1".to_string(),
    //                     "test_function",
    //                     FunctionResult::new(json!({
    //                                                 "status": "completed"
    //                                             })),
    //                 ),
    //             )),
    //         ],
    //     )])
    //         .is_turn_completed(true),
    // ));

    while let Some(msg) = stream.next().await {
        info!("message: {:?}", msg);

        match msg {
            ServerMessage::ToolCall(tool_call) => {
                for tool_call in tool_call.function_calls {
                    if tool_call.name == "end_conversation" {
                        info!("end_conversation function called, ending the conversation");
                        break;
                    }

                    if tool_call.name == "time" {
                        info!("time function called, sending time");
                        let _ = client.call(ClientMessage::ToolResponse(ToolResponse::new(vec![
                            FunctionResponse::new(
                                tool_call.id.clone(),
                                tool_call.name.clone(),
                                FunctionResult::new(json!({
                                    "time": chrono::Utc::now().format("%H:%M:%S %Z").to_string()
                                })),
                            )
                            .with_will_continue(false),
                        ])));
                    }

                    if tool_call.name == "test_function" {
                        info!("test_function function called, sending date and time");
                        let _ = client.call(ClientMessage::ToolResponse(ToolResponse::new(vec![
                            FunctionResponse::new(
                                tool_call.id.clone(),
                                tool_call.name.clone(),
                                FunctionResult::new(json!({
                                    "status": "running"
                                })),
                            )
                            .with_will_continue(true),
                        ])));

                        let _client = client.clone();

                        tokio::spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_secs(5)).await;

                            let _ =
                                _client.call(ClientMessage::ToolResponse(ToolResponse::new(vec![
                                    FunctionResponse::new(
                                        tool_call.id.clone(),
                                        tool_call.name.clone(),
                                        FunctionResult::new(json!({
                                            "status": "completed"
                                        })),
                                    )
                                    .with_will_continue(false),
                                ])));
                        });
                    }
                }
            }
            _ => {}
        }
    }

    client.disconnect(None)?;
    Ok(())
}
