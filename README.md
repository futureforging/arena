# Aria POC 2: Claude Agent Knock Knock Joke Demo

Two agents running in separate Docker containers. Agent A asks Agent B for a knock knock joke via TCP. Built with the `claude-agent` Rust crate.

## Setup

1. **API Key (required)**

   Create `anthropic_api_key.txt` in this directory with your Anthropic API key (one line, no trailing newline):

   ```bash
   echo -n "sk-ant-your-key-here" > anthropic_api_key.txt
   ```

   This file is gitignored. Docker Compose mounts it as a secret.

   To use the host environment instead, add `ANTHROPIC_API_KEY: ${ANTHROPIC_API_KEY}` to the `environment` section in `compose.yaml`, then run:

   ```bash
   export ANTHROPIC_API_KEY=sk-ant-your-key-here
   docker compose up
   ```

2. **Run**

   ```bash
   docker compose up
   ```

   Output from both agents appears in the foreground. When the joke is complete, both agents exit.
