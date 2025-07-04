use derive_new::new;
use derive_setters::Setters;
use serde::{Deserialize, Serialize};
use std::fmt::Formatter;

pub const API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta/models";

#[derive(Debug, Clone, Deserialize, Serialize, new)]
pub struct Content {
    pub role: Role,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub parts: Vec<Part>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PartData {
    Text(String),
    InlineData(InlineData),
    FileData(FileData),
    VideoMetadata(VideoMetadata),
    FunctionCall(FunctionCall),
    FunctionResponse(FunctionResponse),
}

#[derive(Debug, Clone, Deserialize, Serialize, new, Setters)]
#[serde(rename_all = "camelCase")]
#[setters(prefix = "with_", into, strip_option)]
pub struct Part {
    #[serde(flatten)]
    #[setters(skip)]
    pub data: PartData,
    #[serde(default)]
    #[new(default)]
    pub thought: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, new)]
#[serde(rename_all = "camelCase")]
pub struct FunctionCall {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[new(into)]
    pub id: Option<String>,
    #[new(into)]
    pub name: String,
    #[new(into)]
    pub args: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, new)]
#[serde(rename_all = "camelCase")]
pub struct FunctionResult {
    #[new(into)]
    pub result: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FunctionResponseScheduling {
    /// Only add the result to the conversation context, do not interrupt or trigger generation.
    Silent,
    /// Add the result to the conversation context, and prompt to generate output without interrupting ongoing generation.
    WhenIdle,
    /// Add the result to the conversation context, interrupt ongoing generation and prompt to generate output.
    Interrupt,
}

#[derive(Debug, Clone, Serialize, Deserialize, new, Setters)]
#[serde(rename_all = "camelCase")]
#[setters(prefix = "with_", strip_option, into)]
pub struct FunctionResponse {
    #[new(into)]
    #[setters(skip)]
    pub id: Option<String>,
    #[new(into)]
    #[setters(skip)]
    pub name: String,
    #[new(into)]
    #[setters(skip)]
    pub response: FunctionResult,

    #[serde(default)]
    #[new(default)]
    pub will_continue: Option<bool>,

    #[serde(default)]
    #[new(default)]
    pub scheduling: Option<FunctionResponseScheduling>,
}

#[derive(Clone, Deserialize, Serialize, new)]
#[serde(rename_all = "camelCase")]
pub struct InlineData {
    #[new(into)]
    pub(crate) mime_type: String,
    #[new(into)]
    pub(crate) data: String, // Base64 encoded string
}

impl std::fmt::Debug for InlineData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InlineData")
            .field("mime_type", &self.mime_type)
            .field("data", &"[BASE64_DATA_REMOVED_FOR_LOGGING]")
            .finish()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, new)]
#[serde(rename_all = "camelCase")]
pub struct FileData {
    #[new(into)]
    mime_type: String,
    #[new(into)]
    file_uri: String,
}
#[derive(Debug, Clone, Deserialize, Serialize, new)]
#[serde(rename_all = "camelCase")]
pub struct VideoMetadata {
    start_offset: StartOffset,
    end_offset: EndOffset,
}
#[derive(Debug, Clone, Deserialize, Serialize, new)]
pub struct StartOffset {
    #[new(default)]
    seconds: i64,
    #[new(default)]
    nanos: i32,
}
#[derive(Debug, Clone, Deserialize, Serialize, new)]
pub struct EndOffset {
    #[new(default)]
    seconds: i64,
    #[new(default)]
    nanos: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Model,
}

pub mod safety {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum HarmCategory {
        HarmCategoryUnspecified,
        HarmCategorySexuallyExplicit,
        HarmCategoryHateSpeech,
        HarmCategoryHarassment,
        HarmCategoryDangerousContent,
    }
    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum HarmProbability {
        HarmProbabilityUnspecified,
        Negligible,
        Low,
        Medium,
        High,
    }
    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum HarmBlockThreshold {
        HarmBlockThresholdUnspecified,
        BlockNone,
        BlockLowAndAbove,
        BlockMedAndAbove,
        #[serde(rename = "BLOCK_HIGH_AND_ABOVE")]
        BlockOnlyHigh,
    }
}

pub mod request {
    use derive_new::new;
    use derive_setters::Setters;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Deserialize, Serialize, new, Setters)]
    #[setters(prefix = "with_")]
    #[setters(into, strip_option)]
    #[serde(rename_all = "camelCase")]
    pub struct Request {
        #[setters(skip)]
        #[new(into)]
        contents: Vec<super::Content>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        #[new(default)]
        tools: Vec<Tools>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        #[serde(default)]
        #[new(default)]
        safety_settings: Vec<SafetySettings>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[new(default)]
        generation_config: Option<GenerationConfig>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[new(default)]
        system_instruction: Option<SystemInstructionContent>,
    }

    #[derive(Debug, Clone, Deserialize, Serialize, new)]
    #[serde(rename_all = "camelCase")]
    pub struct Tools {
        // e.g. function_calling_config, etc. if API supports more.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        function_declarations: Vec<FunctionDeclaration>,
    }

    #[derive(Debug, Serialize, Deserialize, Clone, Default)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum FunctionBehavior {
        /// If set, the system will wait to receive the function response before continuing the conversation.
        #[default]
        Blocking,
        /// If set, the system will not wait to receive the function response. Instead, it will attempt to handle function responses as they become available while maintaining the conversation between the user and the model.
        NonBlocking,
    }

    #[derive(Debug, Clone, Deserialize, Serialize, new, Setters)]
    #[setters(prefix = "with_")]
    #[setters(into, strip_option)]
    #[serde(rename_all = "camelCase")]
    pub struct FunctionDeclaration {
        #[setters(skip)]
        #[new(into)]
        name: String,
        #[setters(skip)]
        #[new(into)]
        description: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[new(default)]
        parameters: Option<serde_json::Value>, // OpenAPI Schema
        #[serde(skip_serializing_if = "Option::is_none")]
        #[new(default)]
        response: Option<serde_json::Value>, // OpenAPI Schema for response (less common for Gemini req)

        #[new(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        behavior: Option<FunctionBehavior>,
    }

    #[derive(Debug, Clone, Deserialize, Serialize, new)]
    #[serde(rename_all = "camelCase")]
    pub struct SafetySettings {
        category: super::safety::HarmCategory,
        threshold: super::safety::HarmBlockThreshold,
    }
    #[derive(Debug, Clone, Deserialize, Serialize, Setters, new, Default)]
    #[setters(prefix = "with_")]
    #[setters(into, strip_option)]
    #[serde(rename_all = "camelCase")]
    pub struct GenerationConfig {
        #[serde(skip_serializing_if = "Option::is_none")]
        #[new(default)]
        temperature: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[new(default)]
        top_p: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[new(default)]
        top_k: Option<i32>, // Or u32 if always positive
        #[serde(skip_serializing_if = "Option::is_none")]
        #[new(default)]
        candidate_count: Option<i32>, // Or u32
        #[serde(skip_serializing_if = "Option::is_none")]
        #[new(default)]
        max_output_tokens: Option<i32>, // Or u32
        #[serde(skip_serializing_if = "Option::is_none")]
        #[new(default)]
        stop_sequences: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[new(default)]
        response_mime_type: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[new(default)]
        response_schema: Option<serde_json::Value>, // OpenAPI Schema
    }

    #[derive(Debug, Clone, Deserialize, Serialize, new)]
    #[serde(rename_all = "camelCase")]
    pub struct SystemInstructionContent {
        // Gemini API might expect "role: system" here or specific structure.
        // The current structure implies parts directly under system_instruction.
        parts: Vec<SystemInstructionPart>,
    }

    #[derive(Debug, Clone, Deserialize, Serialize, new)]
    #[serde(rename_all = "camelCase")]
    pub struct SystemInstructionPart {
        text: String,
    }
}

pub mod response {
    use serde::Deserialize;

    #[derive(Debug, Clone, Deserialize, Default)]
    #[serde(rename_all = "camelCase")]
    pub struct Response {
        #[serde(default)]
        pub candidates: Vec<Candidate>,
        #[serde(default)]
        pub prompt_feedback: Option<PromptFeedback>,
        #[serde(default)]
        pub usage_metadata: Option<UsageMetadata>,
    }

    #[derive(Debug, Clone, Deserialize, Default)]
    #[serde(rename_all = "camelCase")]
    pub struct Candidate {
        #[serde(default)]
        pub content: Option<super::Content>,
        #[serde(default)]
        pub finish_reason: Option<FinishReason>,
        #[serde(default)]
        pub index: Option<i32>,
        #[serde(default)]
        pub safety_ratings: Vec<SafetyRating>,
    }
    #[derive(Debug, Clone, Deserialize, Default)]
    #[serde(rename_all = "camelCase")]
    pub struct UsageMetadata {
        pub prompt_token_count: Option<u32>,
        pub candidates_token_count: Option<u32>,
        pub total_token_count: Option<u32>,
    }
    #[derive(Debug, Clone, Deserialize, Default)]
    #[serde(rename_all = "camelCase")]
    pub struct PromptFeedback {
        #[serde(default)]
        pub safety_ratings: Vec<SafetyRating>,
    }

    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SafetyRating {
        pub category: super::safety::HarmCategory,
        pub probability: super::safety::HarmProbability,
        #[serde(default)]
        pub blocked: bool,
    }

    #[derive(Debug, Clone, Deserialize, PartialEq)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum FinishReason {
        FinishReasonUnspecified,
        Stop,
        MaxTokens,
        Safety,
        Recitation,
        Other,
    }
}

pub mod live;
pub mod rest;
