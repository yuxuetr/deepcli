use anyhow::{Context, Result};
use base64::Engine;
use futures_util::Stream;
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::pin::Pin;

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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Message {
  Simple { role: String, content: String },
  MultiModal { role: String, content: Vec<Content> },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Content {
  Text(TextContent),
  Image(ImageContent),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TextContent {
  #[serde(rename = "type")]
  pub content_type: String,
  pub text: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImageContent {
  #[serde(rename = "type")]
  pub content_type: String,
  pub image_url: ImageUrl,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImageUrl {
  pub url: String,
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

  pub async fn call_api_with_history(
    &self,
    model: &str,
    messages: Vec<Message>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    json_mode: bool,
  ) -> Result<ApiResponse> {
    let request =
      self.build_request_with_history(model, messages, temperature, max_tokens, json_mode);
    self.send_request(request).await
  }

  pub async fn call_api_with_file(
    &self,
    model: &str,
    query: &str,
    file_path: &Path,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    json_mode: bool,
  ) -> Result<ApiResponse> {
    let request =
      self.build_request_with_file(model, query, file_path, temperature, max_tokens, json_mode)?;
    self.send_request(request).await
  }

  pub async fn call_api_with_history_stream(
    &self,
    model: &str,
    messages: Vec<Message>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    json_mode: bool,
  ) -> Result<Pin<Box<dyn Stream<Item = Result<(String, Option<String>)>> + Send>>> {
    use futures_util::stream;
    use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
    use serde_json::Value;

    let mut request =
      self.build_request_with_history(model, messages, temperature, max_tokens, json_mode);
    request.stream = true;

    let client = &self.client;
    let api_key = &self.api_key;
    let resp = client
      .post("https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions")
      .header(CONTENT_TYPE, "application/json")
      .header(AUTHORIZATION, format!("Bearer {}", api_key))
      .json(&request)
      .send()
      .await
      .context("API request failed")?;

    let stream = resp.bytes_stream();
    let buffer = Vec::new();
    let finished = false;

    let s = stream::unfold(
      (stream, buffer, finished),
      |(mut stream, mut buffer, mut finished)| async move {
        if finished {
          return None;
        }
        while let Some(item) = stream.next().await {
          match item {
            Ok(chunk) => {
              buffer.extend_from_slice(&chunk);
              // 尝试按行分割
              while let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
                let line = buffer.drain(..=pos).collect::<Vec<u8>>();
                let line_str = String::from_utf8_lossy(&line).trim().to_string();
                if line_str.is_empty() {
                  continue;
                }
                if let Some(data) = line_str.strip_prefix("data: ") {
                  if data == "[DONE]" {
                    finished = true;
                    return Some((
                      Ok((String::new(), Some("length".to_string()))),
                      (stream, buffer, finished),
                    ));
                  }
                  // 解析json
                  if let Ok(json) = serde_json::from_str::<Value>(data) {
                    // 兼容OpenAI风格
                    if let Some(choices) = json.get("choices") {
                      if let Some(choice) = choices.get(0) {
                        let finish_reason = choice
                          .get("finish_reason")
                          .and_then(|v| v.as_str())
                          .map(|s| s.to_string());
                        if let Some(delta) = choice.get("delta") {
                          if let Some(content) = delta.get("content") {
                            if let Some(s) = content.as_str() {
                              return Some((
                                Ok((s.to_string(), finish_reason)),
                                (stream, buffer, finished),
                              ));
                            }
                          }
                        }
                        // deepseek 可能直接有 message.content
                        if let Some(message) = choice.get("message") {
                          if let Some(content) = message.get("content") {
                            if let Some(s) = content.as_str() {
                              return Some((
                                Ok((s.to_string(), finish_reason)),
                                (stream, buffer, finished),
                              ));
                            }
                          }
                        }
                        // 如果有 finish_reason 但没有内容，也要传递
                        if finish_reason.is_some() {
                          return Some((
                            Ok((String::new(), finish_reason)),
                            (stream, buffer, finished),
                          ));
                        }
                      }
                    }
                  }
                }
              }
            }
            Err(e) => {
              return Some((
                Err::<(String, Option<String>), _>(anyhow::anyhow!(e)),
                (stream, buffer, true),
              ));
            }
          }
        }
        None
      },
    );
    Ok(Box::pin(s))
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
        Message::Simple {
          role: "system".to_string(),
          content: system_message,
        },
        Message::Simple {
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

  fn build_request_with_history(
    &self,
    model: &str,
    messages: Vec<Message>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    json_mode: bool,
  ) -> ApiRequest {
    ApiRequest {
      model: model.to_string(),
      messages,
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

  fn build_request_with_file(
    &self,
    model: &str,
    query: &str,
    file_path: &Path,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    json_mode: bool,
  ) -> Result<ApiRequest> {
    let file_content = self.read_file_content(file_path)?;
    let mime_type = mime_guess::from_path(file_path)
      .first_or_octet_stream()
      .to_string();

    let content = if mime_type.starts_with("image/") {
      vec![
        Content::Text(TextContent {
          content_type: "text".to_string(),
          text: query.to_string(),
        }),
        Content::Image(ImageContent {
          content_type: "image_url".to_string(),
          image_url: ImageUrl {
            url: format!("data:{};base64,{}", mime_type, file_content),
          },
        }),
      ]
    } else {
      vec![Content::Text(TextContent {
        content_type: "text".to_string(),
        text: format!("{}\n\n文件内容:\n{}", query, file_content),
      })]
    };

    let system_message = if json_mode {
      "You are a helpful assistant. You must output your response in a valid JSON format."
        .to_string()
    } else {
      "You are a helpful assistant.".to_string()
    };

    Ok(ApiRequest {
      model: model.to_string(),
      messages: vec![
        Message::Simple {
          role: "system".to_string(),
          content: system_message,
        },
        Message::MultiModal {
          role: "user".to_string(),
          content,
        },
      ],
      temperature,
      max_tokens,
      stream: true,
      response_format: if json_mode {
        Some(ResponseFormat {
          format_type: "json_object".to_string(),
        })
      } else {
        None
      },
    })
  }

  fn read_file_content(&self, file_path: &Path) -> Result<String> {
    let mime_type = mime_guess::from_path(file_path)
      .first_or_octet_stream()
      .to_string();

    if mime_type.starts_with("image/") {
      // 读取图像文件并转换为base64
      let image_data =
        std::fs::read(file_path).context(format!("Failed to read image file: {:?}", file_path))?;
      Ok(base64::engine::general_purpose::STANDARD.encode(image_data))
    } else {
      // 读取文本文件
      let content = std::fs::read_to_string(file_path)
        .context(format!("Failed to read file: {:?}", file_path))?;
      Ok(content)
    }
  }

  async fn send_request(&self, request: ApiRequest) -> Result<ApiResponse> {
    let response = self
      .client
      .post("https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions")
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
    let message = Message::Simple {
      role: "user".to_string(),
      content: "Hello".to_string(),
    };
    match message {
      Message::Simple { role, content } => {
        assert_eq!(role, "user");
        assert_eq!(content, "Hello");
      }
      _ => panic!("Expected simple message"),
    }
  }

  #[test]
  fn test_json_mode_system_message() {
    let client = ApiClient::new("test_key".to_string());

    // Test JSON mode
    let json_request = client.build_request("deepseek-chat", "test", None, None, true);
    if let Message::Simple { content, .. } = &json_request.messages[0] {
      assert!(content.contains("JSON format"));
    } else {
      panic!("Expected simple message");
    }

    // Test normal mode
    let normal_request = client.build_request("deepseek-chat", "test", None, None, false);
    if let Message::Simple { content, .. } = &normal_request.messages[0] {
      assert!(!content.contains("JSON format"));
    } else {
      panic!("Expected simple message");
    }
  }
}
