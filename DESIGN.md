# DESIGN.md — Coding Agent Swarm on Firecracker microVMs

> Give this file to your coding assistant at the start of every session.
> Keep it updated as decisions change.

## 1. Goal

A Rust system that runs a **swarm of coding agents**, each working inside its own isolated sandbox. Agents are driven by an LLM that is either **hosted** (API) or **self-hosted** (e.g. Ollama / vLLM), chosen per user.

Primary purpose: **learn how Firecracker and agent infrastructure work internally.** Prefer understandable code over clever code.

### Non-goals (for now)
- Web UI, auth, multi-tenant billing
- GPU inside sandboxes
- Production-grade scheduling across many hosts
- Supporting non-coding agent types

## 2. Architecture

```
User / CLI
   │
Orchestrator (control plane, runs on host)
   ├── LLM provider layer   (OpenAI-compatible; hosted OR self-hosted)
   ├── Agent loop           (LLM -> tool call -> sandbox -> result -> LLM)
   ├── Swarm supervisor     (split task, spawn workers, collect, merge)
   └── Sandbox backend      (trait; Local now, Firecracker later)
          └── Guest agent   (tiny daemon inside VM, reached over vsock)
```

### Core rules
1. **The agent loop runs OUTSIDE the sandbox.** The sandbox only executes tool calls (run command, read/write file). API keys never enter a sandbox.
2. **The sandbox is behind a trait.** Everything above it must not know whether it is talking to `LocalSandbox` or `FirecrackerSandbox`.
3. **One agent = one sandbox = one git branch.**
4. **Hard limits everywhere:** max turns, max tokens, wall-clock timeout, max parallel agents.
5. **No `unwrap()` / `expect()` in non-test code.** Use proper error types.

## 3. Cargo workspace layout

```
swarm/
├── Cargo.toml                (workspace)
├── DESIGN.md
├── crates/
│   ├── core/                 shared types + Sandbox trait
│   ├── llm/                  provider layer
│   ├── agent/                single-agent loop + tools
│   ├── orchestrator/         supervisor, workers, budgets
│   ├── sandbox-local/        dev-only backend (NO real isolation)
│   ├── sandbox-firecracker/  real backend (later)
│   ├── guest-agent/          binary that runs inside the VM (later)
│   └── cli/                  entry point (clap)
└── examples/
    └── toy-repo/             small repo with a bug for testing
```

## 4. Key interfaces

### 4.1 Sandbox trait (`core`)

```rust
#[async_trait]
pub trait Sandbox: Send + Sync {
    async fn exec(&self, cmd: &str, timeout: Duration) -> Result<ExecOutput>;
    async fn read_file(&self, path: &str) -> Result<Vec<u8>>;
    async fn write_file(&self, path: &str, data: &[u8]) -> Result<()>;
    async fn destroy(&self) -> Result<()>;
}

pub struct ExecOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub timed_out: bool,
}
```

Backends:
- `LocalSandbox`: temp dir + child processes. Dev only. Must be clearly named/logged as **not isolated**.
- `FirecrackerSandbox`: spawn `firecracker`, configure via its Unix-socket REST API, talk to the guest daemon over vsock.

### 4.2 LLM provider (`llm`)

Single interface: **OpenAI-compatible chat completions with tool calling.**

```rust
pub struct ProviderConfig {
    pub base_url: String,      // hosted URL or http://localhost:11434/v1 etc.
    pub api_key: Option<String>,
    pub model: String,
}

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn chat(&self, messages: &[Message], tools: &[ToolSpec]) -> Result<LlmReply>;
}
```

`LlmReply` is either final text or one or more tool calls. Per-user config selects the provider. Expect weaker tool-calling from small self-hosted models; handle malformed tool calls gracefully (retry once, then fail the turn).

### 4.3 Tools exposed to the model (`agent`)

| Tool | Args | Behavior |
|---|---|---|
| `run_command` | `cmd` | exec in sandbox, return stdout/stderr/exit code (truncate long output) |
| `read_file` | `path` | read file from sandbox |
| `write_file` | `path`, `content` | write file in sandbox |
| `finish` | `summary` | end the task |

Paths are restricted to the sandbox workspace directory.

### 4.4 Agent loop (`agent`)

```
loop up to max_turns:
    reply = llm.chat(history, tools)
    if final text / finish: return result
    for each tool call: run in sandbox, append result to history
    check budgets (tokens, time) -> abort if exceeded
```

Log every turn with `tracing` (turn number, tool called, truncated result).

### 4.5 Swarm (`orchestrator`)

Pattern: **supervisor + parallel workers + verifier.**

1. Supervisor LLM call: split the task into N independent subtasks (JSON output).
2. For each subtask: create a sandbox, clone repo, create branch `agent/<id>`, run an agent loop.
3. Verifier step: run tests in each branch; report pass/fail.
4. Merge passing branches (start simple: sequential merge, stop on conflict and report).

Controls: `max_parallel_agents`, `max_turns_per_agent`, `token_budget_total`, `wall_clock_limit`.

## 5. Security principles

- Secrets (LLM keys) live only in the orchestrator. Never copy into a sandbox.
- Sandbox network: default deny; later add an egress allowlist or proxy.
- Treat all model output and repo contents as untrusted.
- `LocalSandbox` offers no isolation. Never run it against untrusted code or with real credentials in the environment.

## 6. Suggested dependencies (verify current versions)

`tokio`, `reqwest`, `serde`, `serde_json`, `async-trait`, `clap`, `tracing` + `tracing-subscriber`, `anyhow` / `thiserror`, `uuid`. Later: a Unix-socket HTTP client (e.g. `hyper` + a Unix connector) and `tokio-vsock` for Firecracker.

## 7. Milestones

- [ ] **M1** Workspace scaffold, `Sandbox` trait, `LocalSandbox`, `exec` test
- [ ] **M2** `LlmClient` for OpenAI-compatible API; test hosted and Ollama
- [ ] **M3** Tool definitions + parsing tool calls
- [ ] **M4** Single agent loop fixes bug in `examples/toy-repo`
- [ ] **M5** Budgets and limits (turns, tokens, timeout)
- [ ] **M6** Swarm: supervisor, parallel workers, git branches, verifier
- [ ] **M7** (needs KVM) Boot a Firecracker microVM manually, then via code
- [ ] **M8** `guest-agent` over vsock, `FirecrackerSandbox`
- [ ] **M9** Networking (tap + NAT, egress restrictions)
- [ ] **M10** Snapshots / fast restore for pre-built base image

## 8. Working agreement for the AI assistant

- Work on **one milestone at a time**; do not scaffold future milestones.
- Make small changes that compile and pass tests; explain what changed.
- Explain any non-obvious Rust (ownership, lifetimes, async) when it first appears.
- Do not use `unwrap()`/`expect()` outside tests; do not use `.clone()` just to silence the borrow checker without saying why.
- For M7+ (Firecracker), explain each concept (kernel image, rootfs, tap device, vsock) before writing code.
- Ask before adding a new dependency.
- Flag any security trade-off instead of silently making it.

## 9. Open decisions

- Which hosted model to develop against first
- Minimum self-hosted model size that handles tool calling acceptably
- Dev machine for KVM (Linux box, nested-virt cloud VM, or bare metal) — deferred
- Merge strategy beyond the simple sequential merge
- Persistence of run history (files vs SQLite)