# Arena: Yao's Millionaire Peer Demo

Two peer agents run the same binary in separate Docker containers. One connects (PEER), one listens (LISTEN_PORT). Each receives a random secret wealth (1–100 million) and uses cryptographic commitment schemes to determine who is richer without revealing their exact values. Built with the `claude-agent` Rust crate.

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

   Each agent's wealth is printed at startup (not revealed to the other). Output from both agents appears in the foreground: thinking is shown inline as `[Agent X] thinking: (…)`, and messages as `[Agent X] → …`. When the exchange reaches a conclusion, both agents exit.

3. **Output options**

   By default, thinking and messages are shown. To also see tool names and token counts, set:

   ```bash
   PEER_AGENT_VERBOSE=1
   ```

   To write a full debug log to a file:

   ```bash
   PEER_AGENT_LOG=/path/to/debug.log
   ```

4. **Negotiation protocol**

   The negotiation protocol and wealth-comparison strategy are injected into each agent's prompt at startup. This avoids tool-call latency and coordination issues. Set `NEGOTIATION_PROTOCOL` (e.g. `default`) so both agents use the same protocol from `ref/negotiation-protocols.txt`. The connector also receives a random strategy from `ref/strategies.txt` to propose to the listener.
