#[cfg(test)]
mod tests {
  use crate::cli::{map_model, validate_temperature};
  use crate::{ApiRequest, ApiResponse, Choice, Message, ResponseFormat};
  use colored_json::{ColorMode, ColoredFormatter, PrettyFormatter};
  use serde_json::Value;
  use std::error::Error;

  #[tokio::test]
  async fn test_api_request_building() {
    let model = "deepseek-reasoner";
    let query = "test query";
    let temperature = Some(1.0);
    let max_tokens = Some(100);
    let json_mode = true;

    // Create the request object
    let request = build_api_request(model, query, temperature, max_tokens, json_mode);

    // Verify request structure
    assert_eq!(request.model, "deepseek-reasoner");
    assert_eq!(request.temperature, Some(1.0));
    assert_eq!(request.max_tokens, Some(100));
    assert!(!request.stream);
    assert_eq!(
      request.response_format.as_ref().unwrap().format_type,
      "json_object"
    );

    // Verify messages
    assert_eq!(request.messages.len(), 2);
    assert_eq!(request.messages[0].role, "system");
    assert_eq!(
      request.messages[0].content,
      "You are a helpful assistant. You must output your response in a valid JSON format."
    );
    assert_eq!(request.messages[1].role, "user");
    assert_eq!(request.messages[1].content, "test query");
  }

  fn build_api_request(
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

  #[test]
  fn test_output_handling() {
    // Test valid JSON output
    let mut response = ApiResponse {
      choices: vec![Choice {
        message: Message {
          role: "assistant".to_string(),
          content: r#"{"key": "value"}"#.to_string(),
        },
      }],
    };

    let result = handle_output(&response, true);
    assert!(result.is_ok());

    // Test plain text output
    let result = handle_output(&response, false);
    assert!(result.is_ok());

    // Test invalid JSON
    response.choices[0].message.content = "invalid json".to_string();
    let result = handle_output(&response, true);
    assert!(result.is_ok());
  }

  fn handle_output(response: &ApiResponse, json_output: bool) -> Result<(), Box<dyn Error>> {
    if json_output {
      if let Ok(content_json) = serde_json::from_str::<Value>(&response.choices[0].message.content)
      {
        let formatter = ColoredFormatter::new(PrettyFormatter::new());
        let colored = formatter.to_colored_json(&content_json, ColorMode::On)?;
        println!("{}", colored);
      } else {
        println!("{}", response.choices[0].message.content);
      }
    } else {
      println!("{}", response.choices[0].message.content);
    }
    Ok(())
  }

  #[test]
  fn test_json_parsing() {
    let valid_json = r#"{"name": "test", "value": 42}"#;
    let invalid_json = "not json at all";

    // Test valid JSON parsing
    assert!(serde_json::from_str::<Value>(valid_json).is_ok());

    // Test invalid JSON parsing
    assert!(serde_json::from_str::<Value>(invalid_json).is_err());
  }

  #[test]
  fn test_integration_workflow() {
    // Test the complete workflow without actual API calls
    let model_input = "r1";
    let model = map_model(model_input).unwrap();
    assert_eq!(model, "deepseek-reasoner");

    let temperature = 1.0;
    let validated_temp = validate_temperature(temperature).unwrap();
    assert_eq!(validated_temp, 1.0);

    // Test building request
    let request = build_api_request(&model, "test query", Some(validated_temp), Some(100), false);
    assert_eq!(request.model, "deepseek-reasoner");
    assert_eq!(request.temperature, Some(1.0));
  }

  #[test]
  fn test_error_scenarios() {
    // Test invalid model
    assert!(map_model("invalid_model").is_err());

    // Test invalid temperature
    assert!(validate_temperature(-1.0).is_err());
    assert!(validate_temperature(3.0).is_err());
  }
}
