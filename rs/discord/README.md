# asterai:discord

Send messages to Discord channels via the Discord REST API.

## Interface

```wit
package asterai:discord@0.1.0;

interface discord {
  send-message: func(content: string, channel-id: string) -> string;
}
```

Returns the message ID on success, or `"error: ..."` on failure.

## Environment Variables

| Variable        | Required | Description       |
|-----------------|----------|-------------------|
| `DISCORD_TOKEN` | Yes      | Discord bot token |

## Usage

```bash
asterai env add-component my-env asterai:discord
asterai env set-var my-env DISCORD_TOKEN="your-bot-token"

asterai env call my-env asterai:discord discord/send-message \
  "Hello from asterai!" "1234567890"
```
