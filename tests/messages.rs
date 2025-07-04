use gemini::v1beta::live::{
    ClientContent, Content, Part, PartData, Role, ServerContent, ServerMessage,
};

#[test]
fn serializes_turn_messages() {
    let content = ClientContent::new(vec![Content::new(
        Role::User,
        vec![Part::new(PartData::Text("hi".into()))],
    )])
    .is_turn_completed(true);
    let json = serde_json::to_value(&content).expect("serialize");
    assert_eq!(json["turns"][0]["role"], "user");
    assert_eq!(json["turns"][0]["parts"][0]["text"], "hi");
    assert!(json["turnComplete"].as_bool().unwrap());
}

#[test]
fn deserializes_server_content_v1() {
    let data = b"{\n  \"setupComplete\": {}\n}\n";
    let msg: ServerMessage = serde_json::from_slice(data).unwrap();

    match msg {
        ServerMessage::SetupComplete => {}
        other => panic!("unexpected message: {:?}", other),
    }
}

#[test]
fn deserializes_server_content_v2() {
    let data = b"{\n  \"serverContent\": {\n    \"turnComplete\": true\n  },\n  \"usageMetadata\": {\n    \"promptTokenCount\": 536,\n    \"responseTokenCount\": 10,\n    \"totalTokenCount\": 546,\n    \"promptTokensDetails\": [\n      {\n        \"modality\": \"TEXT\",\n        \"tokenCount\": 535\n      },\n      {\n        \"modality\": \"AUDIO\",\n        \"tokenCount\": 1\n      }\n    ],\n    \"responseTokensDetails\": [\n      {\n        \"modality\": \"TEXT\",\n        \"tokenCount\": 10\n      }\n    ]\n  }\n}\n";
    let msg: ServerMessage = serde_json::from_slice(data).unwrap();

    match msg {
        ServerMessage::ServerContent {
            server_content,
            usage_metadata,
        } => {
            assert!(matches!(server_content, ServerContent::TurnComplete));
            assert!(!matches!(server_content, ServerContent::GenerationComplete));
            assert!(usage_metadata.is_some());
        }
        other => panic!("unexpected message: {:?}", other),
    }
}

#[test]
fn deserializes_server_content_v3() {
    let data = b"{\n  \"serverContent\": {\n    \"generationComplete\": true\n  }\n}\n";
    let msg: ServerMessage = serde_json::from_slice(data).unwrap();

    match msg {
        ServerMessage::ServerContent {
            server_content,
            usage_metadata,
        } => {
            assert!(matches!(server_content, ServerContent::GenerationComplete));
            assert!(!matches!(server_content, ServerContent::TurnComplete));
            assert!(usage_metadata.is_none());
        }
        other => panic!("unexpected message: {:?}", other),
    }
}

#[test]
fn deserializes_server_content_with_inline() {
    let data = b"{\n  \"serverContent\": {\n    \"modelTurn\": {\n      \"parts\": [\n        {\n          \"inlineData\": {\n            \"mimeType\": \"audio/pcm;rate=24000\",\n            \"data\": \"AAAAAA==\"\n          }\n        }\n      ]\n    }\n  }\n}\n";

    let msg: ServerMessage = serde_json::from_slice(data).unwrap();
    match msg {
        ServerMessage::ServerContent { server_content, .. } => {
            assert!(matches!(server_content, ServerContent::ModelTurn(..)));

            match server_content {
                ServerContent::ModelTurn(turn) => match &turn.parts[0].data {
                    PartData::InlineData(inline_data) => {
                        assert_eq!(inline_data.mime_type(), "audio/pcm;rate=24000");
                    }
                    other => panic!("unexpected part: {:?}", other),
                },
                _ => panic!(),
            }
        }
        other => panic!("unexpected message: {:?}", other),
    }
}
#[test]
fn deserializes_tool_call() {
    let data = br#"{
        "toolCall": {
            "functionCalls": [
                {"id": "1", "name": "foo", "args": {"x": 1}}
            ]
        }
    }"#;

    let msg: ServerMessage = serde_json::from_slice(data).unwrap();
    match msg {
        ServerMessage::ToolCall(call) => {
            assert_eq!(call.function_calls.len(), 1);
            assert_eq!(call.function_calls[0].name, "foo");
        }
        other => panic!("unexpected message: {:?}", other),
    }
}

#[test]
fn deserializes_tool_call_cancellation() {
    let data = br#"{
        "toolCallCancellation": {"ids": ["abc", "def"]}
    }"#;

    let msg: ServerMessage = serde_json::from_slice(data).unwrap();
    match msg {
        ServerMessage::ToolCallCancellation(cancel) => {
            assert_eq!(cancel.ids, ["abc", "def"]);
        }
        other => panic!("unexpected message: {:?}", other),
    }
}

#[test]
fn deserializes_go_away() {
    let data = br#"{
        "goAway": {"timeLeft": "10s"}
    }"#;

    let msg: ServerMessage = serde_json::from_slice(data).unwrap();
    match msg {
        ServerMessage::GoAway(go) => {
            assert_eq!(go.time_left.as_deref(), Some("10s"));
        }
        other => panic!("unexpected message: {:?}", other),
    }
}

#[test]
fn deserializes_session_resumption_update() {
    let data = br#"{
        "sessionResumptionUpdate": {"newHandle": "h", "resumable": true}
    }"#;

    let msg: ServerMessage = serde_json::from_slice(data).unwrap();
    match msg {
        ServerMessage::SessionResumptionUpdate(update) => {
            assert_eq!(update.new_handle.as_deref(), Some("h"));
            assert_eq!(update.resumable, Some(true));
        }
        other => panic!("unexpected message: {:?}", other),
    }
}

#[test]
fn deserializes_executable_code() {
    let data = b"{\n  \"serverContent\": {\n    \"modelTurn\": {\n      \"parts\": [\n        {\n          \"executableCode\": {\n            \"language\": \"PYTHON\",\n            \"code\": \"print(default_api.time())\\n\"\n          }\n        }\n      ]\n    }\n  }\n}\n";

    let msg: ServerMessage = serde_json::from_slice(data).unwrap();
    match msg {
        ServerMessage::ServerContent { server_content, .. } => {
            assert!(matches!(server_content, ServerContent::ModelTurn(..)));

            match server_content {
                ServerContent::ModelTurn(turn) => match &turn.parts[0].data {
                    PartData::ExecutableCode(executable) => {
                        assert_eq!(executable.language, "PYTHON");
                        assert_eq!(executable.code, "print(default_api.time())\n");
                    }
                    other => panic!("unexpected part: {:?}", other),
                },
                _ => panic!(),
            }
        }
        other => panic!("unexpected message: {:?}", other),
    }
}

#[test]
fn deserializes_part_variants() {
    let text = serde_json::json!({"text": "hi"});
    let part: Part = serde_json::from_value(text).unwrap();
    match part.data {
        PartData::Text(text) => assert_eq!(text, "hi"),
        _ => panic!(),
    }

    let inline = serde_json::json!({"inlineData": {"mimeType": "image/png", "data": "AA=="}});
    let part: Part = serde_json::from_value(inline).unwrap();
    match part.data {
        PartData::InlineData(inline_data) => assert_eq!(inline_data.mime_type(), "image/png"),
        _ => panic!(),
    }

    let call = serde_json::json!({"functionCall": {"name": "foo", "args": {}}});
    let part: Part = serde_json::from_value(call).unwrap();
    match part.data {
        PartData::FunctionCall(function_call) => assert_eq!(function_call.name, "foo"),
        _ => panic!(),
    }

    let resp = serde_json::json!({"functionResponse": {"name": "foo", "response": {"result": 1}}});
    let part: Part = serde_json::from_value(resp).unwrap();
    match part.data {
        PartData::FunctionResponse(function_response) => assert_eq!(function_response.name, "foo"),
        _ => panic!(),
    }

    let file = serde_json::json!({"fileData": {"mimeType": "text/plain", "fileUri": "p"}});
    let part: Part = serde_json::from_value(file).unwrap();
    if let PartData::FileData(file_data) = part.data {
        assert_eq!(file_data.mime_type(), "text/plain");
        assert_eq!(file_data.file_uri(), "p");
    } else {
        panic!();
    }
}
