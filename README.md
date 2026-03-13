# Arena: Interactive Peer-to-Peer Agent Demo

Two peer agents run the same binary in separate Docker containers. One connects (PEER), one listens (LISTEN_PORT). They exchange messages interactively—e.g. one asks for a knock knock joke, the other delivers it. Built with the `claude-agent` Rust crate.

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

3. **Output options**

   By default, only the messages exchanged between agents are shown (`→` sent, `←` received). To see thinking, tool calls, and agent commentary, set:

   ```bash
   PEER_AGENT_VERBOSE=1
   ```

   To write a full debug log to a file:

   ```bash
   PEER_AGENT_LOG=/path/to/debug.log
   ```
