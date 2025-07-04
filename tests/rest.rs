use gemini::v1beta::{Content, Part, PartData, Role, request::Request, response::Response};
use serde_json::json;

#[test]
fn request_serializes_text() {
    let request = Request::new(vec![Content::new(
        Role::User,
        vec![Part::new(PartData::Text("hi".into()))],
    )]);
    let json = serde_json::to_value(&request).expect("serialize");
    assert_eq!(json["contents"][0]["role"], "user");
    assert_eq!(json["contents"][0]["parts"][0]["text"], "hi");
}

#[test]
fn response_deserializes() {
    let data = json!({
        "candidates": [
            {
                "content": {"parts": [{"text": "hi"}], "role": "model"},
                "finishReason": "STOP",
                "index": 0,
                "safetyRatings": []
            }
        ],
        "usageMetadata": {"promptTokenCount": 5, "candidatesTokenCount": 10, "totalTokenCount": 15}
    });
    let resp: Response = serde_json::from_value(data).unwrap();
    assert_eq!(resp.candidates.len(), 1);
    assert!(resp.usage_metadata.is_some());
}
