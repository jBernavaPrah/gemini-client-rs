use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose};
use derive_new::new;
use derive_setters::Setters;
use ezsockets::{
    Bytes, Client as EzClient, ClientConfig, ClientExt, CloseFrame, Error as EzError, Utf8Bytes,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::mpsc::{Sender, channel};
use tokio::sync::oneshot;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, error, info};

/// Default websocket endpoint for Gemini Live API.
const DEFAULT_WS_ENDPOINT: &str = "wss://generativelanguage.googleapis.com/ws/google.ai.generativelanguage.v1beta.GenerativeService.BidiGenerateContent";
/// Default channel capacity for message streams.
const DEFAULT_CHANNEL_CAPACITY: usize = 1024;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Ez(#[from] EzError),
    #[error(transparent)]
    Url(#[from] url::ParseError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Join(#[from] tokio::task::JoinError),
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum ResponseModality {
    Text,
    Audio,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
pub enum ActivityHandling {
    #[serde(rename = "ACTIVITY_HANDLING_UNSPECIFIED")]
    #[default]
    Unspecified,
    #[serde(rename = "START_OF_ACTIVITY_INTERRUPTS")]
    StartOfActivityInterrupts,
    #[serde(rename = "NO_INTERRUPTION")]
    NoInterruption,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
pub enum StartSensitivity {
    #[serde(rename = "START_SENSITIVITY_UNSPECIFIED")]
    #[default]
    Unspecified,
    #[serde(rename = "START_SENSITIVITY_HIGH")]
    High,
    #[serde(rename = "START_SENSITIVITY_LOW")]
    Low,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
pub enum EndSensitivity {
    #[serde(rename = "END_SENSITIVITY_UNSPECIFIED")]
    #[default]
    Unspecified,
    #[serde(rename = "END_SENSITIVITY_HIGH")]
    High,
    #[serde(rename = "END_SENSITIVITY_LOW")]
    Low,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
pub enum TurnCoverage {
    #[default]
    #[serde(rename = "TURN_COVERAGE_UNSPECIFIED")]
    Unspecified,
    #[serde(rename = "TURN_INCLUDES_ONLY_ACTIVITY")]
    OnlyActivity,
    #[serde(rename = "TURN_INCLUDES_ALL_INPUT")]
    AllInput,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum MediaResolution {
    #[serde(rename = "MEDIA_RESOLUTION_UNSPECIFIED")]
    Unspecified,
    #[serde(rename = "MEDIA_RESOLUTION_LOW")]
    Low,
    #[serde(rename = "MEDIA_RESOLUTION_MEDIUM")]
    Medium,
    #[serde(rename = "MEDIA_RESOLUTION_HIGH")]
    High,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, new, Setters)]
#[setters(prefix = "with_", strip_option, into)]
#[serde(rename_all = "camelCase")]
pub struct PrebuiltVoiceConfig {
    #[new(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    voice_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, new, Setters)]
#[setters(prefix = "with_", strip_option)]
#[serde(rename_all = "camelCase")]
pub struct VoiceConfig {
    #[setters(skip)]
    prebuilt_voice_config: PrebuiltVoiceConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, new, Setters)]
#[setters(prefix = "with_", strip_option, into)]
#[serde(rename_all = "camelCase")]
pub struct SpeechConfig {
    #[setters(skip)]
    voice_config: VoiceConfig,
    #[new(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    language_code: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, new, Setters)]
#[setters(prefix = "with_", strip_option)]
#[serde(rename_all = "camelCase")]
pub struct ThinkingConfig {
    #[new(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    include_thoughts: Option<bool>,
    #[new(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking_budget: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum DynamicRetrievalMode {
    #[serde(rename = "MODE_UNSPECIFIED")]
    Unspecified,
    #[serde(rename = "MODE_DYNAMIC")]
    Dynamic,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, new, Setters)]
#[setters(prefix = "with_", strip_option)]
#[serde(rename_all = "camelCase")]
pub struct DynamicRetrievalConfig {
    #[new(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    mode: Option<DynamicRetrievalMode>,
    #[new(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    dynamic_threshold: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, new, Setters)]
#[setters(prefix = "with_", strip_option)]
#[serde(rename_all = "camelCase")]
pub struct GoogleSearchRetrieval {
    #[new(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    dynamic_retrieval_config: Option<DynamicRetrievalConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, new)]
pub struct CodeExecution;

#[derive(Debug, Serialize, Deserialize, Clone, Default, new)]
pub struct GoogleSearch;

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum FunctionCallingMode {
    #[serde(rename = "MODE_UNSPECIFIED")]
    Unspecified,
    #[serde(rename = "AUTO")]
    Auto,
    #[serde(rename = "ANY")]
    Any,
    #[serde(rename = "NONE")]
    None,
    #[serde(rename = "VALIDATED")]
    Validated,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, new, Setters)]
#[setters(prefix = "with_", strip_option)]
#[serde(rename_all = "camelCase")]
pub struct FunctionCallingConfig {
    #[new(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    mode: Option<FunctionCallingMode>,
    #[new(default)]
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    allowed_function_names: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, new, Setters)]
#[setters(prefix = "with_", strip_option)]
#[serde(rename_all = "camelCase")]
pub struct ToolConfig {
    #[new(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    function_calling_config: Option<FunctionCallingConfig>,
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

#[derive(Debug, Serialize, Deserialize, Clone, Default, new, Setters)]
#[setters(prefix = "with_", strip_option, into)]
#[serde(rename_all = "camelCase")]
pub struct FunctionDeclaration {
    #[setters(skip)]
    #[new(into)]
    name: String,
    #[new(default)]
    description: String,
    #[new(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    parameters: Option<serde_json::Value>,
    #[new(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    response: Option<serde_json::Value>,
    #[new(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    behavior: Option<FunctionBehavior>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum Tool {
    FunctionDeclarations(
        #[serde(skip_serializing_if = "Vec::is_empty", default)] Vec<FunctionDeclaration>,
    ),
    GoogleSearchRetrieval(GoogleSearchRetrieval),
    CodeExecution(CodeExecution),
    GoogleSearch(GoogleSearch),
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, new, Setters)]
#[setters(prefix = "with_", strip_option)]
#[serde(rename_all = "camelCase")]
/// Configuration options controlling text generation.
pub struct GenerationConfig {
    #[new(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    candidate_count: Option<u32>,
    #[new(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
    #[new(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[new(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[new(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<u32>,
    #[new(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
    #[new(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    seed: Option<i32>,
    #[new(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    presence_penalty: Option<f32>,
    #[new(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    frequency_penalty: Option<f32>,
    #[new(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    response_logprobs: Option<bool>,
    #[new(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    logprobs: Option<i32>,
    #[new(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    enable_enhanced_civic_answers: Option<bool>,
    #[new(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    response_modalities: Option<Vec<ResponseModality>>,
    #[new(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    speech_config: Option<SpeechConfig>,
    #[new(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking_config: Option<ThinkingConfig>,
    #[new(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    media_resolution: Option<MediaResolution>,
    #[new(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    response_mime_type: Option<String>,
    #[new(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    response_schema: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Clone, new, Setters)]
#[setters(prefix = "with_", strip_option, into)]
#[serde(rename_all = "camelCase")]
/// Parameters sent when opening a new streaming session.
pub struct Setup {
    #[new(into)]
    #[setters(skip)]
    model: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[new(default)]
    generation_config: Option<GenerationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[new(default)]
    system_instruction: Option<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[new(default)]
    tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[new(default)]
    realtime_input_config: Option<RealtimeInputConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[new(default)]
    session_resumption: Option<SessionResumptionConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[new(default)]
    context_window_compression: Option<ContextWindowCompressionConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[new(default)]
    output_audio_transcription: Option<AudioTranscriptionConfig>,
}

#[derive(Debug, Serialize, Clone, new, Setters)]
#[serde(rename_all = "camelCase")]
pub struct ClientContent {
    #[setters(skip)]
    turns: Vec<Content>,
    #[new(value = "false")]
    #[setters(rename = "is_turn_completed")]
    turn_complete: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct StartActivity;
#[derive(Debug, Serialize, Clone)]
pub struct EndActivity;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum RealtimeInput {
    Text(String),
    Audio(InlineData),
    Video(InlineData),
    StartActivity(StartActivity),
    EndActivity(EndActivity),
    AudioStreamEnd(bool),
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AutomaticActivityDetection {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_of_speech_sensitivity: Option<StartSensitivity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix_padding_ms: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_of_speech_sensitivity: Option<EndSensitivity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub silence_duration_ms: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Default, Setters, new)]
#[serde(rename_all = "camelCase")]
#[setters(prefix = "with_", strip_option, into)]
pub struct RealtimeInputConfig {
    #[new(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    automatic_activity_detection: Option<AutomaticActivityDetection>,
    #[new(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    activity_handling: Option<ActivityHandling>,
    #[new(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    turn_coverage: Option<TurnCoverage>,
}

#[derive(Debug, Clone, Serialize, Default, new)]
#[serde(rename_all = "camelCase")]
pub struct SessionResumptionConfig {
    #[new(into)]
    pub handle: String,
}

#[derive(Debug, Clone, Serialize, Default, new, Setters)]
#[serde(rename_all = "camelCase")]
#[setters(prefix = "with_", strip_option, into)]
pub struct ContextWindowCompressionConfig {
    #[new(into)]
    #[setters(skip)]
    sliding_window: SlidingWindow,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[new(default)]
    trigger_tokens: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Default, new)]
#[serde(rename_all = "camelCase")]
pub struct SlidingWindow {
    pub target_tokens: i64,
}

#[derive(Debug, Clone, Serialize, Default, new)]
pub struct AudioTranscriptionConfig {}

#[derive(Debug, Serialize, Deserialize, Clone, new)]
#[serde(rename_all = "camelCase")]
/// Binary data sent as part of a user message.
///
/// The bytes are encoded using base64 when serialized to JSON.
pub struct InlineData {
    #[new(into)]
    mime_type: String,
    #[serde(
        serialize_with = "serialize_base64",
        deserialize_with = "deserialize_base64"
    )]
    data: Vec<u8>,
}

impl InlineData {
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn mime_type(&self) -> &str {
        &self.mime_type
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

impl FileData {
    pub fn mime_type(&self) -> &str {
        &self.mime_type
    }

    pub fn file_uri(&self) -> &str {
        &self.file_uri
    }
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

#[derive(Debug, Clone, Deserialize, Serialize, new)]
pub struct ExecutableCode {
    #[new(into)]
    pub language: String,
    #[new(into)]
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Enumeration of possible outcomes of the code execution.
pub enum CodeExecutionOutcome {
    /// Code execution completed successfully.
    #[serde(rename = "OUTCOME_OK")]
    Ok,
    /// Code execution finished but with a failure. stderr should contain the reason.
    #[serde(rename = "OUTCOME_FAILED")]
    Failed,
    /// Code execution ran for too long, and was cancelled. There may or may not be a partial output present.
    #[serde(rename = "OUTCOME_DEADLINE_EXCEEDED")]
    DeadlineExceeded,
}

#[derive(Debug, Clone, Deserialize, Serialize, new, Setters)]
#[setters(prefix = "with_", strip_option, into)]
pub struct CodeExecutionResult {
    outcome: CodeExecutionOutcome,
    #[new(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    output: Option<String>,
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
    ExecutableCode(ExecutableCode),
    CodeExecutionResult(CodeExecutionResult),
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
    pub thought: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, new)]
#[serde(rename_all = "camelCase")]
pub struct Content {
    #[new(into)]
    pub role: Option<Role>,
    pub parts: Vec<Part>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Model,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Modality {
    ModalityUnspecified,
    Text,
    Image,
    Video,
    Audio,
    Document,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModalityTokenCount {
    pub modality: Modality,
    pub token_count: i32,
}

/// Serialize a byte array as a base64 encoded string.
fn serialize_base64<S>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&general_purpose::STANDARD.encode(bytes))
}

/// Deserialize a base64 encoded string into raw bytes.
fn deserialize_base64<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    general_purpose::STANDARD
        .decode(s)
        .map_err(serde::de::Error::custom)
}

#[derive(Debug, Serialize, Clone, new)]
#[serde(rename_all = "camelCase")]
pub struct ToolResponse {
    function_responses: Vec<FunctionResponse>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum ClientMessage {
    Setup(Setup),
    ClientContent(ClientContent),
    RealtimeInput(RealtimeInput),
    ToolResponse(ToolResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupComplete {}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ServerContent {
    ModelTurn(Content),

    #[serde(deserialize_with = "deserialize_ignore")]
    GenerationComplete,
    #[serde(deserialize_with = "deserialize_ignore")]
    TurnComplete,
    #[serde(deserialize_with = "deserialize_ignore")]
    Interrupted,

    GroundingMetadata(serde_json::Value),
    OutputTranscription(Transcription),
    InputTranscription(Transcription),
}

// Custom deserializer that ignores the boolean value and returns unit type
fn deserialize_ignore<'de, D>(deserializer: D) -> Result<(), D::Error>
where
    D: serde::Deserializer<'de>,
{
    // Deserialize the boolean but ignore its value
    let _ignored: serde_json::Value = serde_json::Value::deserialize(deserializer)?;
    Ok(())
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transcription {
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    #[serde(default)]
    pub function_calls: Vec<FunctionCall>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallCancellation {
    #[serde(default)]
    pub ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoAway {
    pub time_left: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionResumptionUpdate {
    pub new_handle: Option<String>,
    pub resumable: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageMetadata {
    pub prompt_token_count: Option<i32>,
    pub cached_content_token_count: Option<i32>,
    pub response_token_count: Option<i32>,
    pub tool_use_prompt_token_count: Option<i32>,
    pub thoughts_token_count: Option<i32>,
    pub total_token_count: Option<i32>,
    #[serde(default)]
    pub prompt_tokens_details: Vec<ModalityTokenCount>,
    #[serde(default)]
    pub cache_tokens_details: Vec<ModalityTokenCount>,
    #[serde(default)]
    pub response_tokens_details: Vec<ModalityTokenCount>,
    #[serde(default)]
    pub tool_use_prompt_tokens_details: Vec<ModalityTokenCount>,
}

#[derive(Debug, Clone)]
pub enum ServerMessage {
    SetupComplete,
    ServerContent {
        server_content: ServerContent,
        usage_metadata: Option<UsageMetadata>,
    },
    ToolCall(ToolCall),
    ToolCallCancellation(ToolCallCancellation),
    GoAway(GoAway),
    SessionResumptionUpdate(SessionResumptionUpdate),
}

impl<'de> Deserialize<'de> for ServerMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Helper {
            setup_complete: Option<SetupComplete>,
            server_content: Option<ServerContent>,
            usage_metadata: Option<UsageMetadata>,
            tool_call: Option<ToolCall>,
            tool_call_cancellation: Option<ToolCallCancellation>,
            go_away: Option<GoAway>,
            session_resumption_update: Option<SessionResumptionUpdate>,
        }

        let helper = Helper::deserialize(deserializer)?;
        if helper.setup_complete.is_some() {
            Ok(ServerMessage::SetupComplete)
        } else if let Some(content) = helper.server_content {
            Ok(ServerMessage::ServerContent {
                server_content: content,
                usage_metadata: helper.usage_metadata,
            })
        } else if let Some(call) = helper.tool_call {
            Ok(ServerMessage::ToolCall(call))
        } else if let Some(cancel) = helper.tool_call_cancellation {
            Ok(ServerMessage::ToolCallCancellation(cancel))
        } else if let Some(go) = helper.go_away {
            Ok(ServerMessage::GoAway(go))
        } else if let Some(update) = helper.session_resumption_update {
            Ok(ServerMessage::SessionResumptionUpdate(update))
        } else {
            Err(serde::de::Error::custom("unknown server message"))
        }
    }
}

#[derive(Debug, new)]
struct WsClient {
    setup: Setup,
    sender: Sender<ServerMessage>,
    inner: ezsockets::Client<Self>,
    #[new(into)]
    connected_sender: Option<oneshot::Sender<()>>,
    #[new(default)]
    session_resumption: Option<SessionResumptionConfig>,
}

#[async_trait]
impl ClientExt for WsClient {
    type Call = ClientMessage;

    async fn on_text(&mut self, text: Utf8Bytes) -> Result<(), EzError> {
        debug!("received message: {}", text);

        Ok(())
    }

    async fn on_binary(&mut self, bytes: Bytes) -> Result<(), EzError> {
        debug!("received binary message: {:?}", bytes);

        match serde_json::from_slice::<ServerMessage>(bytes.as_ref()) {
            Ok(msg) => {
                if let ServerMessage::SessionResumptionUpdate(update) = &msg {
                    if update.resumable == Some(true) {
                        if let Some(handle) = update.new_handle.clone() {
                            self.session_resumption.replace(SessionResumptionConfig {
                                handle: handle.clone(),
                            });
                        }
                    }
                }

                if self.sender.send(msg).await.is_err() {
                    return Err("failed to send message".into());
                }
            }
            Err(e) => {
                error!("failed to deserialize message: {}", e);
            }
        }

        Ok(())
    }

    async fn on_call(&mut self, call: Self::Call) -> Result<(), EzError> {
        let msg = serde_json::to_string(&call)?;
        match call {
            ClientMessage::RealtimeInput(_) => {}
            _ => debug!("sending message: {:?}", msg),
        };

        self.inner
            .text(msg)
            .map_err(|e| Error::from(EzError::from(e)))?;
        Ok(())
    }

    async fn on_connect(&mut self) -> Result<(), EzError> {
        let mut setup = self.setup.clone();
        if let Some(session_resumption) = self.session_resumption.clone() {
            setup.session_resumption = Some(session_resumption);
        }

        let _ = self.inner.call(ClientMessage::Setup(setup));

        if let Some(tx) = self.connected_sender.take() {
            let _ = tx.send(());
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
/// Client for interacting with the Gemini Live websocket API.
pub struct Client {
    client: EzClient<WsClient>,
}

impl Client {
    /// Establish a websocket connection using the provided API key and setup.
    ///
    /// Returns the [`Client`] and a stream of incoming [`ServerMessage`]s.
    pub async fn connect(
        api_key: impl Into<String>,
        setup: Setup,
    ) -> Result<(Self, ReceiverStream<ServerMessage>), Error> {
        Self::connect_with_endpoint(api_key, setup, DEFAULT_WS_ENDPOINT).await
    }

    /// Establish a websocket connection using the provided API key, setup and
    /// custom endpoint. Returns the [`Client`] and a stream of
    /// incoming [`ServerMessage`]s.
    #[tracing::instrument(
        skip(api_key, setup),
        fields(endpoint = %endpoint, setup = ?setup)
    )]
    pub async fn connect_with_endpoint(
        api_key: impl Into<String>,
        setup: Setup,
        endpoint: &str,
    ) -> Result<(Self, ReceiverStream<ServerMessage>), Error> {
        let config = ClientConfig::new(endpoint).query_parameter("key", &api_key.into());
        let (tx, rx) = channel(DEFAULT_CHANNEL_CAPACITY);
        let (tx_connected, rx_connected) = oneshot::channel();
        let setup_clone = setup.clone();
        let (handle, _fut) =
            ezsockets::connect(move |h| WsClient::new(setup, tx, h, tx_connected), config).await;

        match rx_connected.await {
            Ok(_) => info!(endpoint = %endpoint, ?setup_clone, "websocket connection established"),
            Err(e) => error!(endpoint = %endpoint, ?setup_clone, ?e, "websocket connection failed"),
        }

        Ok((Self { client: handle }, ReceiverStream::new(rx)))
    }

    /// Send a message to the server.
    pub fn call(&self, message: ClientMessage) -> Result<(), Error> {
        Ok(self.client.call(message).map_err(EzError::from)?)
    }

    /// Close the websocket connection.
    pub fn disconnect(self, reason: Option<CloseFrame>) -> Result<(), Error> {
        self.client.close(reason).map_err(EzError::from)?;
        Ok(())
    }
}
