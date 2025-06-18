use clap::{Arg, Command, builder::ValueParser};

pub fn build_cli() -> Command {
  Command::new("deepcli")
    .about("DeepSeek command-line interface")
    .arg(
      Arg::new("model")
        .long("model")
        .short('m')
        .value_name("MODEL")
        .help("Model to use: r1 (deepseek-reasoner) or chat (deepseek-chat)")
        .default_value("chat"),
    )
    .arg(
      Arg::new("temperature")
        .long("temperature")
        .short('t')
        .value_name("TEMPERATURE")
        .help("Sampling temperature (0.0-2.0)")
        .value_parser(ValueParser::new(|s: &str| {
          s.parse::<f32>()
            .map_err(|e| e.to_string())
            .and_then(|temp| {
              if (0.0..=2.0).contains(&temp) {
                Ok(temp)
              } else {
                Err("Temperature must be between 0.0 and 2.0".to_string())
              }
            })
        })),
    )
    .arg(
      Arg::new("max_tokens")
        .long("max_tokens")
        .short('l')
        .value_name("MAX_TOKENS")
        .help("Maximum number of tokens to generate")
        .value_parser(clap::value_parser!(u32)),
    )
    .arg(
      Arg::new("json")
        .long("json")
        .help("Output response as formatted JSON")
        .action(clap::ArgAction::SetTrue),
    )
    .arg(
      Arg::new("query")
        .help("Query to send to the model")
        .required(true)
        .index(1),
    )
}

#[allow(dead_code)]
pub fn validate_temperature(temp: f32) -> Result<f32, String> {
  if (0.0..=2.0).contains(&temp) {
    Ok(temp)
  } else {
    Err("Temperature must be between 0.0 and 2.0".to_string())
  }
}

pub fn map_model(model: &str) -> Result<String, String> {
  match model {
    "r1" => Ok("deepseek-reasoner".to_string()),
    "chat" => Ok("deepseek-chat".to_string()),
    _ => Err("Invalid model. Use 'r1' or 'chat'.".to_string()),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_cli_building() {
    let cmd = build_cli();
    assert_eq!(cmd.get_name(), "deepcli");
  }

  #[test]
  fn test_cli_arguments() {
    // Test default values
    let matches = build_cli().get_matches_from(vec!["deepcli", "hello"]);
    assert_eq!(matches.get_one::<String>("model").unwrap(), "chat");
    assert_eq!(matches.get_one::<String>("query").unwrap(), "hello");
    assert!(!matches.get_flag("json"));

    // Test model selection
    let matches = build_cli().get_matches_from(vec!["deepcli", "-m", "r1", "hello"]);
    assert_eq!(matches.get_one::<String>("model").unwrap(), "r1");

    // Test temperature
    let matches = build_cli().get_matches_from(vec!["deepcli", "-t", "1.5", "hello"]);
    assert_eq!(matches.get_one::<f32>("temperature").unwrap(), &1.5);

    // Test max tokens
    let matches = build_cli().get_matches_from(vec!["deepcli", "-l", "100", "hello"]);
    assert_eq!(matches.get_one::<u32>("max_tokens").unwrap(), &100);

    // Test JSON flag
    let matches = build_cli().get_matches_from(vec!["deepcli", "--json", "hello"]);
    assert!(matches.get_flag("json"));
  }

  #[test]
  fn test_temperature_validation() {
    // Test valid temperature values
    assert!(validate_temperature(0.0).is_ok());
    assert!(validate_temperature(1.0).is_ok());
    assert!(validate_temperature(2.0).is_ok());

    // Test invalid temperature values
    assert!(validate_temperature(-0.1).is_err());
    assert!(validate_temperature(2.1).is_err());
  }

  #[test]
  fn test_model_mapping() {
    assert_eq!(map_model("r1").unwrap(), "deepseek-reasoner");
    assert_eq!(map_model("chat").unwrap(), "deepseek-chat");
    assert!(map_model("invalid").is_err());
  }

  #[test]
  fn test_invalid_temperature_parsing() {
    let result = build_cli().try_get_matches_from(vec!["deepcli", "-t", "3.0", "hello"]);
    assert!(result.is_err());
  }

  #[test]
  fn test_missing_query() {
    let result = build_cli().try_get_matches_from(vec!["deepcli"]);
    assert!(result.is_err());
  }
}
