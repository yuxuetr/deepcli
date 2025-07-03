use anyhow::{Context, Result};
use crossterm::style::{Color, Print, ResetColor, SetForegroundColor};
use futures_util::StreamExt;
use std::env;
use std::io::{self, Write};

mod api;
mod cli;

pub use api::{ApiClient, Message};
pub use cli::{build_cli, map_model};

fn get_model_max_tokens(model: &str) -> u32 {
  match model {
    "deepseek-r1" => 65536,
    "deepseek-chat" => 8192,
    _ => 4096,
  }
}

fn get_model_max_input_tokens(_model: &str) -> usize {
  65536 // 64K tokens
}

fn estimate_tokens(text: &str) -> usize {
  // 粗略估算，1 token ≈ 4 字符
  text.chars().count() / 4 + 1
}

const MAX_AUTO_CONTINUE: usize = 5;

#[tokio::main]
async fn main() -> Result<()> {
  let matches = build_cli().get_matches();
  let model_input = matches.get_one::<String>("model").unwrap();
  let model = map_model(model_input).map_err(|e| anyhow::anyhow!(e))?;
  let temperature = matches.get_one::<f32>("temperature").copied();
  let max_tokens = matches
    .get_one::<u32>("max_tokens")
    .copied()
    .unwrap_or_else(|| get_model_max_tokens(&model));
  let api_key =
    env::var("DASHSCOPE_API_KEY").context("DASHSCOPE_API_KEY environment variable not set")?;
  let client = ApiClient::new(api_key);

  let mut history: Vec<Message> = vec![];
  let stdin = io::stdin();
  let mut stdout = io::stdout();

  loop {
    print_red_prompt(&mut stdout);
    stdout.flush()?;
    let mut input = String::new();
    stdin.read_line(&mut input)?;
    let input = input.trim();
    if input.is_empty() {
      continue;
    }
    if input == "\\q" {
      break;
    }
    if input == "\\c" {
      history.clear();
      continue;
    }
    // 添加到历史
    history.push(Message::Simple {
      role: "user".to_string(),
      content: input.to_string(),
    });
    // 构造带历史的消息
    let mut messages = vec![Message::Simple {
      role: "system".to_string(),
      content: "You are a helpful assistant.".to_string(),
    }];
    messages.extend(history.iter().cloned());
    // 检查token数，超限则自动摘要
    let max_input_tokens = get_model_max_input_tokens(&model);
    let total_tokens: usize = messages
      .iter()
      .map(|m| match m {
        Message::Simple { content, .. } => estimate_tokens(content),
        Message::MultiModal { content, .. } => content
          .iter()
          .map(|c| match c {
            api::Content::Text(t) => estimate_tokens(&t.text),
            api::Content::Image(_) => 0,
          })
          .sum(),
      })
      .sum();
    if total_tokens > max_input_tokens {
      // 自动摘要历史
      let history_text = messages
        .iter()
        .filter_map(|m| match m {
          Message::Simple { role, content } => {
            if role == "user" || role == "assistant" {
              Some(format!("{}: {}", role, content))
            } else {
              None
            }
          }
          _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
      let summary_prompt = format!(
        "请用中文总结以下对话内容，保留关键信息，便于后续继续对话：\n{}",
        history_text
      );
      print_green_prompt(&mut stdout);
      stdout.flush()?;
      let mut summary = String::new();
      match client
        .call_api_with_history_stream(
          &model,
          vec![
            Message::Simple {
              role: "system".to_string(),
              content: "你是一个对话历史摘要助手。".to_string(),
            },
            Message::Simple {
              role: "user".to_string(),
              content: summary_prompt,
            },
          ],
          temperature,
          Some(2048),
          false,
        )
        .await
      {
        Ok(mut stream) => {
          while let Some(chunk) = stream.next().await {
            match chunk {
              Ok((s, _)) => {
                print!("{}", s);
                stdout.flush()?;
                summary.push_str(&s);
              }
              Err(e) => {
                eprintln!("[摘要API流错误]: {}", e);
                break;
              }
            }
          }
          println!(" ");
        }
        Err(e) => {
          println!("[摘要API错误]: {}", e);
        }
      }
      // 用摘要替换历史
      history.clear();
      history.push(Message::Simple {
        role: "user".to_string(),
        content: format!("[历史摘要] {}", summary),
      });
      // 重新构造messages
      messages = vec![Message::Simple {
        role: "system".to_string(),
        content: "You are a helpful assistant.".to_string(),
      }];
      messages.extend(history.iter().cloned());
    }
    // 自动续写主流程
    let mut reply = String::new();
    let mut auto_continue_count = 0;
    loop {
      print_green_prompt(&mut stdout);
      stdout.flush()?;
      let mut last_reason = None;
      // eprintln!("[DEBUG] max_tokens: {}", max_tokens);
      match client
        .call_api_with_history_stream(
          &model,
          messages.clone(),
          temperature,
          Some(max_tokens),
          false,
        )
        .await
      {
        Ok(mut stream) => {
          while let Some(chunk) = stream.next().await {
            match chunk {
              Ok((s, reason)) => {
                print!("{}", s);
                stdout.flush()?;
                reply.push_str(&s);
                // if let Some(ref r) = reason {
                //   eprintln!("[DEBUG] finish_reason: {}", r);
                // }
                if reason.is_some() {
                  last_reason = reason;
                }
              }
              Err(e) => {
                eprintln!("[API流错误]: {}", e);
                break;
              }
            }
          }
          println!(" ");
        }
        Err(e) => {
          println!("[API错误]: {}", e);
          break;
        }
      }
      history.push(Message::Simple {
        role: "assistant".to_string(),
        content: reply.clone(),
      });

      // 检查是否需要自动续写
      let should_continue = if let Some(reason) = last_reason.as_deref() {
        // eprintln!("[DEBUG] Detected finish_reason: {}", reason);
        reason == "length"
      } else {
        // 如果没有finish_reason，检查回复是否看起来被截断了
        let trimmed = reply.trim();
        trimmed.ends_with("（")
          || trimmed.ends_with("、")
          || trimmed.ends_with("，")
          || trimmed.ends_with("：")
          || trimmed.ends_with("-")
          || trimmed.ends_with("**")
          || (trimmed.len() > 100
            && !trimmed.ends_with("。")
            && !trimmed.ends_with("！")
            && !trimmed.ends_with("？"))
      };

      if should_continue && auto_continue_count < MAX_AUTO_CONTINUE {
        auto_continue_count += 1;
        // eprintln!(
        //   "[DEBUG] Auto-continuing (attempt {}/{})",
        //   auto_continue_count, MAX_AUTO_CONTINUE
        // );
        history.push(Message::Simple {
          role: "user".to_string(),
          content: "请继续".to_string(),
        });
        messages = vec![Message::Simple {
          role: "system".to_string(),
          content: "You are a helpful assistant.".to_string(),
        }];
        messages.extend(history.iter().cloned());
        reply.clear();
        continue;
      }
      break;
    }
  }
  Ok(())
}

fn print_red_prompt(stdout: &mut io::Stdout) {
  let _ = crossterm::queue!(
    stdout,
    SetForegroundColor(Color::Red),
    Print("> "),
    ResetColor
  );
}

fn print_green_prompt(stdout: &mut io::Stdout) {
  let _ = crossterm::queue!(
    stdout,
    SetForegroundColor(Color::Green),
    Print("> "),
    ResetColor
  );
}
