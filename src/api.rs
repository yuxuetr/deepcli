use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct ApiRequest {
  pub model: String,
  pub messages: Vec<Message>,
  pub temperature: Option<f32>,
  pub max_tokens: Option<u32>,
  pub stream: bool,
  pub response_format: Option<ResponseFormat>,
}

#[derive(Debug, Serialize)]
pub struct ResponseFormat {
  #[serde(rename = "type")]
  pub format_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
  pub role: String,
  pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse {
  pub choices: Vec<Choice>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Choice {
  pub message: Message,
}

pub struct ApiClient {
  client: Client,
  api_key: String,
}

impl ApiClient {
  pub fn new(api_key: String) -> Self {
    Self {
      client: Client::new(),
      api_key,
    }
  }

  pub async fn call_api(
    &self,
    model: &str,
    query: &str,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    json_mode: bool,
  ) -> Result<ApiResponse> {
    let request = self.build_request(model, query, temperature, max_tokens, json_mode);
    self.send_request(request).await
  }

  fn build_request(
    &self,
    model: &str,
    query: &str,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    json_mode: bool,
  ) -> ApiRequest {
    // Update system message when in JSON mode
    let system_message = if json_mode {
      "You are a helpful assistant. You must output your response in a valid JSON format."
        .to_string()
    } else {
      "You are a helpful assistant.".to_string()
    };

    ApiRequest {
      model: model.to_string(),
      messages: vec![
        Message {
          role: "system".to_string(),
          content: system_message,
        },
        Message {
          role: "user".to_string(),
          content: query.to_string(),
        },
      ],
      temperature,
      max_tokens,
      stream: false,
      response_format: if json_mode {
        Some(ResponseFormat {
          format_type: "json_object".to_string(),
        })
      } else {
        None
      },
    }
  }

  async fn send_request(&self, request: ApiRequest) -> Result<ApiResponse> {
    let response = self
      .client
      .post("https://api.deepseek.com/chat/completions")
      .header("Content-Type", "application/json")
      .header("Authorization", format!("Bearer {}", self.api_key))
      .json(&request)
      .send()
      .await
      .context("API request failed")?;

    if !response.status().is_success() {
      let status = response.status();
      let error_text = response
        .text()
        .await
        .unwrap_or_else(|_| "Unknown error".into());
      anyhow::bail!("API Error {}: {}", status, error_text);
    }

    response
      .json()
      .await
      .context("Failed to parse API response")
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_api_client_creation() {
    let client = ApiClient::new("test_key".to_string());
    assert_eq!(client.api_key, "test_key");
  }

  #[test]
  fn test_request_building() {
    let client = ApiClient::new("test_key".to_string());
    let request = client.build_request("deepseek-chat", "test query", Some(1.0), Some(100), true);

    assert_eq!(request.model, "deepseek-chat");
    assert_eq!(request.temperature, Some(1.0));
    assert_eq!(request.max_tokens, Some(100));
    assert!(!request.stream);
    assert!(request.response_format.is_some());
    assert_eq!(request.response_format.unwrap().format_type, "json_object");
    assert_eq!(request.messages.len(), 2);
  }

  #[test]
  fn test_message_creation() {
    let message = Message {
      role: "user".to_string(),
      content: "Hello".to_string(),
    };
    assert_eq!(message.role, "user");
    assert_eq!(message.content, "Hello");
  }

  #[test]
  fn test_json_mode_system_message() {
    let client = ApiClient::new("test_key".to_string());

    // Test JSON mode
    let json_request = client.build_request("deepseek-chat", "test", None, None, true);
    assert!(json_request.messages[0].content.contains("JSON format"));

    // Test normal mode
    let normal_request = client.build_request("deepseek-chat", "test", None, None, false);
    assert!(!normal_request.messages[0].content.contains("JSON format"));
  }
}
