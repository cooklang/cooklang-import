# AI Providers

`cooklang-import` supports multiple AI providers for recipe conversion.

## OpenAI

- **Models**: gpt-6-luna (default). OpenAI removes the GPT-4 family (gpt-4o, gpt-4o-mini, gpt-4.1, gpt-4.1-mini and fine-tunes of them) from its API on 2027-01-14.
- **Environment Variables**: `OPENAI_API_KEY`; optional `OPENAI_MODEL` and `OPENAI_REASONING_EFFORT`
- **Reasoning**: off by default (`none`), which is the fastest and is enough for conversion. Set `reasoning_effort` (or `OPENAI_REASONING_EFFORT`) to `low`, `medium` or `high` to turn it on. `temperature` is only sent while reasoning is off: the model rejects it otherwise.

```toml
[providers.openai]
enabled = true
model = "gpt-6-luna"
temperature = 0.2
max_tokens = 4000
# reasoning_effort = "low"
```

## Anthropic Claude

- **Models**: claude-haiku-4-5 (default). Claude 5 models are not supported yet: they reject the `temperature` this converter sends.
- **Environment Variable**: `ANTHROPIC_API_KEY`

```toml
[providers.anthropic]
enabled = true
model = "claude-haiku-4-5"
temperature = 0.7
max_tokens = 4000
```

## Google Gemini

- **Models**: gemini-2.5-flash (latest), gemini-2.0-flash-lite
- **Environment Variable**: `GOOGLE_API_KEY`

```toml
[providers.google]
enabled = true
model = "gemini-2.5-flash"
temperature = 0.7
max_tokens = 2000
```

## Azure OpenAI

- **Models**: Your deployed models (e.g., gpt-4, gpt-35-turbo)
- **Environment Variable**: `AZURE_OPENAI_API_KEY`
- **Required Config**: `endpoint`, `deployment_name`, `api_version`

```toml
[providers.azure_openai]
enabled = true
endpoint = "https://your-resource.openai.azure.com"
deployment_name = "your-deployment"
api_version = "2024-02-15-preview"
```

## Ollama (Local)

Run AI models locally without API keys.

- **Models**: llama3, llama2, codellama, mixtral, and more
- **Requirements**: [Ollama](https://ollama.ai) installed locally
- **Base URL**: `http://localhost:11434` (default)

### Setup

```bash
# Install Ollama
curl -fsSL https://ollama.ai/install.sh | sh

# Pull a model
ollama pull llama3
```

### Configuration

```toml
[providers.ollama]
enabled = true
model = "llama3"
base_url = "http://localhost:11434"
```

## Provider Fallback

Enable automatic failover between providers:

```toml
[fallback]
enabled = true
order = ["openai", "anthropic", "google"]
retry_attempts = 3
retry_delay_ms = 1000
```

When enabled:
1. Tries the primary provider with exponential backoff retries
2. On failure, switches to the next provider in the list
3. Continues until success or all providers exhausted

## Environment Variable Format

For nested configuration, use double underscores:

```sh
export OPENAI_MODEL="gpt-6-luna"            # OpenAI model, shortest form
export COOKLANG__PROVIDERS__OPENAI__MODEL="gpt-6-luna"
export COOKLANG__FALLBACK__ENABLED=true
```

## Configuration Priority

1. Environment variables (highest)
2. `config.toml` file
3. Default values (lowest)
