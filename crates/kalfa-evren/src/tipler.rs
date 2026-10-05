//! EVREN'in OpenAI uyumlu JSON şemasının Rust karşılıkları.
//!
//! Bilinmeyen alanlar yok sayılır. EVREN'e özgü alanlar — `reasoning`,
//! `usage.evren`, `error.evren`, katalog uzantıları — açıkça modellenir
//! (spec §3.3).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Modelin düşünce metni. Yalnızca yanıtta gelir; isteğe geri gönderilmez.
    #[serde(default, skip_serializing)]
    pub reasoning: Option<String>,
}

impl ChatMessage {
    fn bos(role: Role) -> Self {
        Self {
            role,
            content: None,
            tool_calls: None,
            tool_call_id: None,
            reasoning: None,
        }
    }

    pub fn system(metin: impl Into<String>) -> Self {
        Self {
            content: Some(metin.into()),
            ..Self::bos(Role::System)
        }
    }

    pub fn user(metin: impl Into<String>) -> Self {
        Self {
            content: Some(metin.into()),
            ..Self::bos(Role::User)
        }
    }

    pub fn assistant(content: Option<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            content,
            tool_calls: if tool_calls.is_empty() {
                None
            } else {
                Some(tool_calls)
            },
            ..Self::bos(Role::Assistant)
        }
    }

    pub fn tool(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            content: Some(content.into()),
            tool_call_id: Some(tool_call_id.into()),
            ..Self::bos(Role::Tool)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type", default = "function_turu")]
    pub kind: String,
    pub function: FunctionCall,
}

fn function_turu() -> String {
    "function".to_string()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolDef {
    #[serde(rename = "type")]
    pub kind: String,
    pub function: FunctionDef,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionDef {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

impl ToolDef {
    pub fn function(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: Value,
    ) -> Self {
        Self {
            kind: function_turu(),
            function: FunctionDef {
                name: name.into(),
                description: description.into(),
                parameters,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolDef>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<StreamOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

impl ChatRequest {
    pub fn new(model: impl Into<String>, messages: Vec<ChatMessage>) -> Self {
        Self {
            model: model.into(),
            messages,
            tools: Vec::new(),
            stream: false,
            stream_options: None,
            response_format: None,
            max_tokens: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct StreamOptions {
    pub include_usage: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ChatResponse {
    pub id: String,
    pub model: String,
    pub choices: Vec<Choice>,
    #[serde(default)]
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Choice {
    #[serde(default)]
    pub index: u32,
    pub message: ChatMessage,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    #[serde(default)]
    pub prompt_tokens: u64,
    #[serde(default)]
    pub completion_tokens: u64,
    #[serde(default)]
    pub total_tokens: u64,
    #[serde(default)]
    pub completion_tokens_details: Option<CompletionTokensDetails>,
    #[serde(default)]
    pub evren: Option<UsageEvren>,
}

impl Usage {
    /// `reasoning_tokens`, `completion_tokens`'ın dökümüdür; ona eklenmez (§5.2).
    pub fn reasoning_tokens(&self) -> u64 {
        self.completion_tokens_details
            .as_ref()
            .map_or(0, |d| d.reasoning_tokens)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompletionTokensDetails {
    #[serde(default)]
    pub reasoning_tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageEvren {
    pub credits_held_cr: Decimal,
    pub credits_remaining_cr: Decimal,
    #[serde(default)]
    pub routed_model: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ChatChunk {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub choices: Vec<ChunkChoice>,
    #[serde(default)]
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ChunkChoice {
    #[serde(default)]
    pub index: u32,
    #[serde(default)]
    pub delta: Delta,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Delta {
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub reasoning: Option<String>,
    #[serde(default)]
    pub tool_calls: Option<Vec<ToolCallDelta>>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ToolCallDelta {
    pub index: u32,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub function: Option<FunctionDelta>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct FunctionDelta {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub arguments: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ModelList {
    pub data: Vec<ModelInfo>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub task: String,
    #[serde(default)]
    pub modalities: Vec<String>,
    #[serde(default)]
    pub pricing: Option<Pricing>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Pricing {
    pub prompt_token_price: Decimal,
    pub completion_token_price: Decimal,
    pub currency: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TermsStatus {
    pub current_version: u32,
    pub is_material: bool,
    pub accepted: bool,
    #[serde(default)]
    pub doc_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TermsText {
    pub version: u32,
    pub content: String,
    #[serde(default)]
    pub doc_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ApiErrorBody {
    pub error: ApiError,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ApiError {
    #[serde(default)]
    pub message: String,
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub evren: Option<ErrorEvren>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ErrorEvren {
    #[serde(default)]
    pub required_terms_version: Option<u32>,
    #[serde(default)]
    pub suggested_models: Vec<String>,
}
