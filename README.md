# DeepCLI - DeepSeek Command Line Interface

DeepCLI is a command-line tool that allows you to interact with DeepSeek's large language models (LLMs) directly from your terminal. It supports both the `deepseek-reasoner` and `deepseek-chat` models and provides JSON output formatting capabilities.

## Features

- Query DeepSeek models directly from your terminal
- Choose between `r1` (reasoning) and `chat` (conversational) models
- Control output creativity with temperature parameter
- Limit response length with max tokens parameter
- Get beautifully formatted, colorized JSON output
- Simple setup and easy to use
- File input support (text and image)
- Chat history preservation
- Beautiful terminal interface

## Installation

### Prerequisites

- Rust (version 1.65 or higher)
- DeepSeek API key

### Install from source

```bash
# Clone the repository
git clone https://github.com/yuxuetr/deepcli.git
cd deepcli

# Build the project
cargo build --release

# The binary will be at target/release/deepcli
```

### Install via Cargo

```bash
cargo install deepcli
```

## Configuration

Set your DeepSeek API key as an environment variable:

```bash
export DASHSCOPE_API_KEY=your_api_key_here
```

Add this to your shell profile (`.bashrc`, `.zshrc`, etc.) to make it permanent.

## Usage

### Interactive Mode

Start the interactive mode:

```bash
./target/release/deepcli -i
```

In interactive mode:
- Type text directly for conversation
- Use `\file <file_path>` to analyze a file
- Use `\clear` to clear current input (without clearing history)
- Press `Ctrl+C` to exit

### Single Query Mode

```bash
# Basic query
./target/release/deepcli "你好，请介绍一下自己"

# Specify model
./target/release/deepcli -m r1 "请帮我分析这个问题"

# Set parameters
./target/release/deepcli -t 0.7 -l 1000 "请详细解释这个概念"

# JSON output
./target/release/deepcli --json "请以JSON格式返回结果"
```

### Command Line Parameters

- `-m, --model <MODEL>`: Choose model (`r1` or `chat`, default: `chat`)
- `-t, --temperature <TEMPERATURE>`: Set temperature (0.0-2.0)
- `-l, --max-tokens <MAX_TOKENS>`: Set maximum token count
- `-i, --interactive`: Start interactive mode
- `--json`: Output response as formatted JSON
- `-h, --help`: Display help information

### File Support

#### Text Files

```bash
# In interactive mode
\file /path/to/document.txt

# Or analyze file content directly
./target/release/deepcli "分析这个文件" --file /path/to/document.txt
```

#### Image Files

Supports common image formats (PNG, JPG, JPEG, etc.):

```bash
# In interactive mode
\file /path/to/image.png
```

## Development Setup

### Prerequisites

- Rust toolchain (install via [rustup](https://rustup.rs/))
- DeepSeek API key

### Recommended VSCode Plugins

- rust-analyzer: Rust language support
- crates: Rust package management
- Better TOML: TOML file support
- GitLens: Git enhancements
- Error Lens: Enhanced error highlighting

### Build and Run

```bash
# Build in release mode
cargo build --release

# Run with debug output
cargo run -- "Your query here"
```

### Testing

```bash
cargo test
```

## Contributing

Contributions are welcome! Please follow these steps:

1. Fork the repository
2. Create a new branch (`git checkout -b feature/your-feature`)
3. Commit your changes (`git commit -am 'Add some feature'`)
4. Push to the branch (`git push origin feature/your-feature`)
5. Create a new Pull Request

### Pre-commit Checks

Install pre-commit hooks to ensure code quality:

```bash
pipx install pre-commit
pre-commit install
```

### Dependency Security

Check dependencies for security issues with:

```bash
cargo deny check
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgements

- DeepSeek for their powerful language models
- Rust community for excellent tooling and libraries
