# Substrate

**A coding agent swarm runtime built on Firecracker microVMs, written in Rust.**

Substrate orchestrates multiple LLM-driven coding agents in parallel, giving each agent its own isolated sandbox and git branch. The LLM backend is pluggable: use a hosted API or a self-hosted model, selected per user.

> **Status: early development.** Substrate is currently in the design and scaffolding phase. The architecture described below is the target; see the [Roadmap](#roadmap) for what is implemented today.

---

## Motivation

Autonomous coding agents execute model-generated code, which is inherently untrusted. Running that code directly on a developer machine or a shared host is a security and reliability risk. Container isolation shares the host kernel, so a kernel-level escape compromises the host.

Substrate takes a stricter approach:

- **Hardware-level isolation.** Each agent runs in its own [Firecracker](https://github.com/firecracker-microvm/firecracker) microVM with a dedicated guest kernel.
- **Control plane separation.** The agent loop and credentials live on the host; the sandbox only executes tool calls.
- **Backend independence.** The same orchestration logic runs against hosted or self-hosted models.

A secondary goal of the project is educational: to build the surrounding infrastructure (VM lifecycle, host-guest communication, networking, snapshotting) in a way that is readable and well documented.

## Architecture

```
User / CLI
   │
Orchestrator (control plane, host)
   ├── LLM provider layer   OpenAI-compatible; hosted or self-hosted
   ├── Agent loop           LLM → tool call → sandbox → result → LLM
   ├── Swarm supervisor     task splitting, parallel workers, verification, merge
   └── Sandbox backend      trait-based; local (dev) or Firecracker
          └── Guest agent   small daemon inside the VM, reached over vsock
```

### Design principles

1. **The agent loop runs outside the sandbox.** Sandboxes execute commands and file operations only. API keys and orchestration state never enter the guest.
2. **The sandbox is an abstraction.** All higher layers depend on a `Sandbox` trait, not on a specific backend.
3. **One agent, one sandbox, one branch.** Work is isolated at the VM level and integrated through git.
4. **Bounded execution.** Turn limits, token budgets, wall-clock timeouts, and concurrency caps are enforced by the orchestrator.
5. **Untrusted by default.** Model output and repository contents are treated as untrusted input.

### Components

| Crate | Responsibility |
|---|---|
| `core` | Shared types and the `Sandbox` trait |
| `llm` | Provider layer for OpenAI-compatible chat completions with tool calling |
| `agent` | Single-agent loop and tool definitions |
| `orchestrator` | Supervisor, parallel workers, verification, budgets |
| `sandbox-local` | Development backend using a temp directory and child processes (**no isolation**) |
| `sandbox-firecracker` | Firecracker backend using the VMM REST API and vsock |
| `guest-agent` | In-VM daemon that executes commands and file operations |
| `cli` | Command-line entry point |

### Sandbox interface

```rust
#[async_trait]
pub trait Sandbox: Send + Sync {
    async fn exec(&self, cmd: &str, timeout: Duration) -> Result<ExecOutput>;
    async fn read_file(&self, path: &str) -> Result<Vec<u8>>;
    async fn write_file(&self, path: &str, data: &[u8]) -> Result<()>;
    async fn destroy(&self) -> Result<()>;
}
```

### LLM providers

Substrate targets the OpenAI-compatible chat completions API with tool calling as its single provider interface. A provider is configured by base URL, optional API key, and model name, which covers hosted services as well as self-hosted servers such as vLLM, Ollama, and llama.cpp.

Tool-calling reliability varies across models. Smaller self-hosted models may require retries or may not be suitable for agentic coding tasks.

### Swarm model

1. A supervisor decomposes a task into independent subtasks.
2. Each subtask runs in its own sandbox on a dedicated branch (`agent/<id>`).
3. A verification step runs tests per branch.
4. Passing branches are merged; conflicts are reported rather than resolved silently.

## Security model

- Credentials are held by the orchestrator and are never copied into a sandbox.
- Sandbox networking is default-deny, with egress controls planned.
- The Firecracker backend relies on KVM hardware virtualization for isolation.
- **`LocalSandbox` provides no isolation.** It exists for development only and must not be used with untrusted code or real credentials.

Substrate has not undergone a security review. Do not rely on it to contain adversarial workloads.

## Requirements

- Rust (stable toolchain)
- Git
- For the Firecracker backend: a Linux host with KVM (`/dev/kvm`) available, either on bare metal or through nested virtualization
- An LLM endpoint: a hosted API key or a locally served OpenAI-compatible model

The local development backend runs on any platform supported by the Rust toolchain.

## Roadmap

- [ ] Workspace scaffold, `Sandbox` trait, `LocalSandbox`
- [ ] OpenAI-compatible LLM client (hosted and self-hosted)
- [ ] Tool definitions and tool-call parsing
- [ ] Single-agent loop
- [ ] Execution budgets and limits
- [ ] Swarm orchestration (supervisor, parallel workers, verifier)
- [ ] Firecracker backend (VM lifecycle via API socket)
- [ ] Guest agent over vsock
- [ ] Network isolation and egress policy
- [ ] Snapshot and restore for fast sandbox startup

Detailed design notes live in [`DESIGN.md`](./DESIGN.md).

## Repository layout

```
substrate/
├── Cargo.toml
├── DESIGN.md
├── crates/
│   ├── core/
│   ├── llm/
│   ├── agent/
│   ├── orchestrator/
│   ├── sandbox-local/
│   ├── sandbox-firecracker/
│   ├── guest-agent/
│   └── cli/
└── examples/
    └── toy-repo/
```

## Contributing

The project is in an early stage and the architecture may change. Issues and design discussion are welcome. Please open an issue before starting significant work.

## License

To be determined.