use anyhow::{Context, Result};
use colored_json::{ColorMode, ColoredFormatter, PrettyFormatter};
use serde_json::Value;
use std::env;

mod api;
mod cli;
mod test;

pub use api::{ApiClient, ApiRequest, ApiResponse, Choice, Message, ResponseFormat};
pub use cli::{build_cli, map_model};

#[tokio::main]
async fn main() -> Result<()> {
  let matches = build_cli().get_matches();

  let model_input = matches.get_one::<String>("model").unwrap();
  let model = map_model(model_input).map_err(|e| anyhow::anyhow!(e))?;

  let query = matches.get_one::<String>("query").unwrap();
  let json_output = matches.get_flag("json");
  let temperature = matches.get_one::<f32>("temperature").copied();
  let max_tokens = matches.get_one::<u32>("max_tokens").copied();
  let api_key =
    env::var("DEEPSEEK_API_KEY").context("DEEPSEEK_API_KEY environment variable not set")?;

  let client = ApiClient::new(api_key);
  let response = client
    .call_api(&model, query, temperature, max_tokens, json_output)
    .await?;

  if json_output {
    handle_json_output(&response)?;
  } else {
    handle_text_output(&response);
  }

  Ok(())
}

fn handle_json_output(response: &ApiResponse) -> Result<()> {
  // Parse the content as JSON value
  if let Ok(content_json) = serde_json::from_str::<Value>(&response.choices[0].message.content) {
    // Format and colorize the JSON
    let formatter = ColoredFormatter::new(PrettyFormatter::new());
    let colored = formatter
      .to_colored_json(&content_json, ColorMode::On)
      .context("Failed to colorize JSON")?;
    println!("{}", colored);
  } else {
    // If content is not valid JSON, print it as plain text
    println!("{}", response.choices[0].message.content);
  }
  Ok(())
}

fn handle_text_output(response: &ApiResponse) {
  println!("{}", response.choices[0].message.content);
}
