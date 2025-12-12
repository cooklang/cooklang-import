# Troubleshooting

## Common Errors

### "OPENAI_API_KEY must be set"

Set your API key:

```sh
export OPENAI_API_KEY="your-key-here"
```

Or use a different provider with its corresponding key.

### "No providers available in fallback configuration"

Ensure at least one provider is:
- Enabled in `config.toml` (`enabled = true`)
- Included in `fallback.order`
- Has a valid API key configured

### Rate Limiting

If you encounter rate limits:
1. Enable fallback to use multiple providers
2. Increase `retry_delay_ms` in config
3. Use a different provider temporarily

### Recipe Extraction Failed

If structured extractors fail:
1. The tool falls back to Chrome fetcher for JS-heavy sites
2. Then tries LLM-based text extraction
3. Check if the site requires authentication or has anti-bot protection

### Timeout Errors

Increase the timeout:

```sh
cooklang-import https://example.com/recipe --timeout 90
```

Or in code:

```rust
RecipeImporter::builder()
    .url("...")
    .timeout(Duration::from_secs(90))
    .build()
    .await?;
```

## Debug Logging

Enable debug output:

```sh
RUST_LOG=debug cooklang-import <url>
```

## Migration Notes

### From Environment Variables Only

If upgrading from a version that only used environment variables:

1. **No action required** - Environment variables continue to work
2. **Optional**: Create `config.toml` for advanced features
3. Keep API keys in environment variables for security
