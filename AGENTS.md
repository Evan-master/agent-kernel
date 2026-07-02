# Agent Kernel Agent Handbook

This file guides all agentic contributors working anywhere under this repo root.
Follow every rule here plus any future nested `AGENTS.md` files.

## Project Direction

- This repository is building an Agent Kernel prototype, not a Linux wrapper, not a shell agent, and not a POSIX-first compatibility layer.
- The native system model is agent-first: resources, capabilities, intents, actions, observations, checkpoints, rollback, verification, delegation, and event logs.
- Compatibility with legacy systems may be added later as a subordinate subsystem. It must not define the core architecture.
- LLM inference, remote model calls, prompt handling, and high-level planning must stay outside kernel space. The kernel exposes deterministic primitives; the supervisor performs reasoning.
- Prefer a small, inspectable, testable kernel over broad features that blur boundaries.

## Architecture Layers

- `crates/agent-kernel-core/`: no_std-friendly domain model and deterministic kernel primitives. Keep this crate free of heap allocation, host I/O, networking, threads, async runtimes, file system calls, and model/client code.
- `crates/agent-kernel/`: kernel facade and syscall-style interface over `agent-kernel-core`. Keep it no_std-compatible unless a documented architecture change explicitly says otherwise.
- `crates/agent-supervisor/`: host-side simulator and user-space supervisor experiments. This crate may use `std`, printing, CLI arguments, and host adapters.
- `docs/`: architecture notes, plans, design records, and research context.
- Future boot, architecture, or HAL crates must be added as separate crates instead of being mixed into the core model.

Before editing, map the change to one of these layers and reject cross-layer mixing. If the edit is to governance or documentation files instead of runtime code, state which runtime layers the rule is protecting.

## Native Kernel Model Rules

- Do not introduce Unix/POSIX abstractions as the primary model. Terms such as file, process, socket, user, and permission can appear only when bridging or explaining legacy concepts.
- Prefer AgentOS-native names: `Resource`, `Capability`, `Action`, `Observation`, `Checkpoint`, `Rollback`, `Verifier`, `Task`, `Agent`, and `Event`.
- Resource access must flow through explicit capabilities. Do not add convenience methods that bypass authorization.
- Every mutating operation must have an event-log consequence or a documented reason it is intentionally invisible.
- Checkpoint and rollback are first-class kernel concepts. Treat them as core behavior, not as tooling bolted on later.
- Verification is a kernel-visible lifecycle step. A successful action is not equivalent to a verified outcome.

## no_std And Determinism Rules

- Keep `agent-kernel-core` and `agent-kernel` compatible with `#![no_std]`.
- Do not use `Vec`, `String`, `Box`, `HashMap`, `Rc`, `Arc`, filesystem APIs, sockets, threads, timers, randomness, or environment variables in no_std crates unless an allocator/runtime decision has been designed and documented first.
- Prefer fixed-capacity stores, typed IDs, slices, arrays, and explicit error returns.
- Avoid hidden global mutable state. If state is needed, make it owned by a kernel struct or passed explicitly.
- Keep APIs deterministic and easy to replay from an event log.
- Use explicit error enums. Do not panic for normal authorization, capacity, lookup, or validation failures.

## Supervisor Boundary Rules

- `agent-supervisor` may simulate agent behavior, host bridges, LLM calls, and external adapters, but it must not become the kernel.
- The supervisor asks for capabilities and calls syscall-style methods; it does not mutate kernel internals directly.
- Host integration code belongs behind adapters. Do not leak host-specific assumptions into kernel crates.
- When adding a new supervisor flow, include the event sequence it is expected to produce.
- Never store API keys, model tokens, credentials, or private endpoint values in tracked files.

## Testing And TDD

- New runtime behavior must start with a failing test. Watch the test fail for the intended reason before implementing.
- Prefer tests that exercise real kernel behavior through public APIs. Avoid testing private implementation details.
- `agent-kernel-core` tests should validate deterministic state transitions, authorization, capacity failures, event ordering, and rollback/checkpoint semantics.
- `agent-kernel` tests should validate syscall facade behavior and kernel boundary contracts.
- `agent-supervisor` validation may include integration-style command output checks, but core behavior belongs in library tests.
- Run focused tests while developing, then run the full workspace before delivery:

```bash
cargo test --workspace
cargo run -p agent-supervisor
```

## Build, Formatting, And Tooling

- Use stable Rust unless a documented kernel requirement needs nightly.
- Keep the workspace buildable with ordinary Cargo commands.
- Before committing, run:

```bash
cargo fmt --check
cargo test --workspace
cargo run -p agent-supervisor
```

- If adding a bare-metal boot target later, document its toolchain, target triple, emulator command, and expected boot output in the same diff.
- Do not add dependencies casually. For kernel crates, justify every dependency against no_std compatibility, determinism, auditability, and long-term kernel ownership.

## File Size And Responsibility Rules

- Enforce one file, one primary responsibility.
- Default size limits:
  - no_std core modules: soft limit 220 lines, hard limit 400 lines.
  - kernel facade modules: soft limit 180 lines, hard limit 320 lines.
  - supervisor modules: soft limit 250 lines, hard limit 450 lines.
  - tests: soft limit 250 lines, hard limit 500 lines.
  - docs and governance files may be longer when they are intentionally comprehensive.
- If a file exceeds the soft limit, prefer splitting by cohesive responsibility before adding more logic.
- If a file exceeds the hard limit, split it unless it is generated code or the user explicitly requests consolidation.

## Documentation And Comment Rules

- Every new runtime module must include a short module-level doc comment describing:
  - purpose,
  - owning layer,
  - main responsibilities,
  - key dependencies,
  - important constraints or editing pitfalls.
- Use comments only for non-obvious logic: authorization invariants, replay semantics, fixed-capacity tradeoffs, checkpoint/rollback behavior, boot constraints, and host/kernel boundary decisions.
- Do not narrate obvious assignments or standard Rust syntax.
- If a directory gains non-trivial architecture context, add or update a local `README.md`.
- Keep design notes in `docs/`. Do not scatter architecture decisions across issue comments or untracked scratch files.

## Security And Authority Rules

- This project intentionally explores high-authority agent control, so authority boundaries must be explicit rather than implicit.
- Do not add "temporary" bypasses for capability checks.
- Do not log or print secrets. Redact credentials in simulator or host bridge output.
- Treat destructive operations, external network writes, host filesystem writes, and credential access as separate capability classes.
- Prefer least authority for tests and examples even when the final system may support powerful agents.
- If a user request asks for a shortcut that weakens kernel authority boundaries, follow the user request only after stating the deviation and risk.

## Git, Reviews, And PR Readiness

- Keep diffs tight and scoped to the task.
- Do not mix architecture rewrites, dependency upgrades, generated artifacts, and feature work in one commit unless the user explicitly asks for it.
- Commit messages should describe the kernel primitive or boundary being added, for example `feat: add capability authorization core`.
- Before publishing, inspect `git status -sb` and stage only intended files.
- Private repository setup should use GitHub CLI with an explicit private repo:

```bash
gh repo create agent-kernel --private --source=. --remote=origin --push
```

## Delivery Requirements

After editing, report:

- architecture placement summary,
- kernel boundary checks performed,
- tests or validation performed,
- compatibility impact,
- technical debt or follow-up note,
- documentation impact.

If a user request conflicts with this handbook, follow the user request and explicitly call out the deviation and risk.
