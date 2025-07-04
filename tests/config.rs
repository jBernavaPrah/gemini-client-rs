use base64::Engine;
use gemini::v1beta::live::{Content, GenerationConfig, InlineData, Role, Setup};

#[test]
fn inline_data_encodes_base64() {
    let data = vec![0x1, 0x2, 0x3];
    let inline = InlineData::new("image/png", data.clone());
    let json = serde_json::to_value(&inline).expect("serialize");
    let encoded = json["data"].as_str().unwrap();
    assert_eq!(
        encoded,
        base64::engine::general_purpose::STANDARD.encode(data)
    );
}

#[test]
fn generation_config_serializes_optional() {
    let cfg = GenerationConfig::new()
        .with_max_output_tokens(10)
        .with_stop_sequences(vec!["END".to_string()]);
    let value = serde_json::to_value(&cfg).expect("serialize");
    assert_eq!(value["maxOutputTokens"], 10);
    assert_eq!(value["stopSequences"][0], "END");
    assert!(value.get("temperature").is_none());
}

#[test]
fn setup_system_instruction_serializes_content() {
    let instruction = Content {
        parts: vec![],
        role: Some(Role::Model),
    };
    let setup = Setup::new("models/test").with_system_instruction(instruction.clone());
    let json = serde_json::to_value(&setup).expect("serialize");
    assert_eq!(
        json["systemInstruction"],
        serde_json::to_value(instruction).unwrap()
    );
}
