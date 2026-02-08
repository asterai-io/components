# asterai:llm

A unified LLM component that sends a prompt to any supported provider and returns the response.

```wit
prompt: func(prompt: string, model: string) -> string;
```

The `model` parameter uses the format `provider/model` (e.g. `openai/gpt-5-mini`).
API keys are read from environment variables.

## Supported providers

| Provider   | Env var          | Example model                                                 |
|------------|------------------|---------------------------------------------------------------|
| openai     | `OPENAI_KEY`     | `openai/gpt-5-mini`                                           |
| anthropic  | `ANTHROPIC_KEY`  | `anthropic/claude-opus-4-6`                                   |
| mistral    | `MISTRAL_KEY`    | `mistral/mistral-large-latest`                                |
| groq       | `GROQ_KEY`       | `groq/llama-3.1-8b-instant`                                   |
| google     | `GOOGLE_KEY`     | `google/gemini-2.5-flash`                                     |
| venice     | `VENICE_KEY`     | `venice/kimi-k2-5`                                            |
| xai        | `XAI_KEY`        | `xai/grok-4-fast-reasoning`                                   |
| deepseek   | `DEEPSEEK_KEY`   | `deepseek/deepseek-chat`                                      |
| together   | `TOGETHER_KEY`   | `together/meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo`       |
| fireworks  | `FIREWORKS_KEY`  | `fireworks/accounts/fireworks/models/llama-v3p1-70b-instruct` |
| perplexity | `PERPLEXITY_KEY` | `perplexity/sonar-pro`                                        |
| openrouter | `OPENROUTER_KEY` | `openrouter/anthropic/claude-sonnet-4`                        |
