---
doc-graph: "When there is an intention to amend this document, first Adhere to [dc-doc-role-continuity]; otherwise, do not modify."
---

# Change implementer/coordinator continuity — 2026-10-27

## Role and task

This dated instance reconstitutes the standing Change implementer/coordinator role for “Two-host qualification and Windows-TX-to-Linux-RX throughput.” The principal whose rulings govern this role is Tomas Gonzalez. The current next action is to follow up the P1 multi-lane RX loss regression, complete the missing reverse-direction lane results, and determine whether an administrator can raise the 4 MiB receive-buffer cap; P1 remains in progress. P1 correctness evidence reported at composition is `check-pat`: 1,588,466 blocks, zero bad, plus a matching 1 GiB random-file SHA-256. Measured evidence was produced by static-musl build `355ac60` against the `3fb9443` baseline. The report was supplied by the owner; no repository copy of test data or logs exists.

The project uses the accepted behavior and architecture in the [specification](../docs/spec.md) and [design](../docs/design.md). The [roadmap](../docs/roadmap.md) owns prospective optimization candidates; the [Authority Index](../docs/index.md) routes durable owners; [open decisions](../docs/open-decisions.md) lists none at composition. No graph role or runtime binding is selected by this instance.

## Owner rulings

| Ruling                                                                                                                                                | Current effect                                                                                             |
| ---                                                                                                                                                   | ---                                                                                                        |
| Use the established RX memory-limit name `--rx-memory`; do not rename it to arena terminology.                                                        | Preserve the current public interface in the [specification](../docs/spec.md).                             |
| Treat the implementation in the [specification](../docs/spec.md) and [design](../docs/design.md) as the accepted built design.                        | Do not reopen its behavior or architecture without changed authority.                                      |
| First-release qualification is Linux x86_64 only; Linux aarch64 and current-build macOS retest are deferred; Windows is next.                         | The loopback result does not establish a real-link or Windows qualification.                               |
| No encryption is in scope; links are trusted or protected by an external VPN.                                                                         | Do not add a cryptographic protocol requirement.                                                           |
| The header session is 16-bit and header version stays 1 during pre-alpha.                                                                             | No version bumps before a formal release.                                                                  |
| No release is being cut.                                                                                                                              | Do not release, tag, or publish.                                                                           |
| Test files and logs must not be stored under `/mnt/shared`.                                                                                           | Keep generated artifacts on each host's local disk, under the reported `~/.local/state/socket-mux/` paths. |
| Jumbo MTU waits until the Windows peer is available.                                                                                                  | Do not run or claim end-to-end jumbo qualification earlier.                                                |
| Use subagents to reduce wall time where the host permits and role custody remains clear.                                                              | Preserve independent evaluation and do not delegate mutations outside the assigned boundary.               |
| Ask the owner one question at a time, without identifiers; group by topic and provide full context, examples, 2–4 options, and a recommended default. | Use this format only when a consequential owner decision is actually needed.                               |

## Open decisions and gates

- Windows peer architecture is unconfirmed. The peer is described as a different architecture; the current target proposal is `x86_64-pc-windows-gnu`, requiring the missing Rust target and zig toolchain. If the peer is ARM64 Windows, evaluate `aarch64-pc-windows-gnullvm`. The next action is to confirm architecture before selecting the cross target; the owner decides any target change.
- Whether an administrator can raise `net.core.rmem_max` remains unknown. Host B (`chuck`, `192.168.7.40`) has no root; Host A (`192.168.7.33`) root/admin capability is unknown. Ask the administrator before relying on a larger socket buffer; remeasure after any authorized cap change.
- The P1 multi-lane RX regression is an active defect investigation, not a settled design decision. Investigate it before optimizing; preserve current behavior until a measured correction is authorized and verified.

## Believed action authority

The owner supplied this standing working-authority belief: the role may edit `src/`, tests, and project documentation and may commit locally using author `Tomas Gonzalez <tegonzalez@gmail.com>` with `Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>`. It may SSH to `chuck` and run copied binaries there. It may not push, tag, or release, and it has no sudo authority. Host A's administrative privilege is unknown. These capabilities do not assign phase work: this plan grants no implementation execution authority, and code/test changes, host runs, or implementation commits require a current owner-issued assignment with dependencies. Current concurrent-work boundaries still prevail: do not overwrite another writer's work or touch `README.md`, `src/`, or tests when another assignment owns them. Keep test data and logs off `/mnt/shared`.

## Resumption

Re-run the selected project's loader from `./.agents/init.md` and complete the graph initialization selected by that loader before any governed mutation. Read this continuity instance and the current durable owners it cites; the instance carries the standing task, rulings, gates, sources, and next action without relying on another temporal record. Before any code or governed-document change, bind current target custody, action authority, and verification evidence. Start with P1 regression follow-up, then proceed only through the phase dependencies and owner acceptance captured here. A new qualifying continuity instance supersedes this one only after surviving meaning has reached its current owners; this instance retires when its successor has resumed and its first pass has landed.

## Currency

At composition, the measured baseline was commit `3fb9443`; P1 result build was `355ac60` static musl. `docs/open-decisions.md` contained no product decision entries. Confirm baseline identity with `git rev-parse --short 3fb9443` and check open decisions in the durable owner before relying on these claims. The P1 rates and counters are included above as owner-reported resumption context; remeasure against current host and binary state before using them as current performance evidence.
