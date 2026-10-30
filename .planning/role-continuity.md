---
doc-graph: "When there is an intention to amend this document, first Adhere to [dc-doc-role-continuity]; otherwise, do not modify."
---

# Change implementer/coordinator continuity — 2026-10-30

## Role and task

This dated instance reconstitutes the standing Change implementer/coordinator role for "Two-host qualification and Windows-TX-to-Linux-RX throughput". The principal whose rulings govern this role is Tomas Gonzalez. No doc-graph role is specified and no runtime binding is selected by this instance.

Finished work, not open items:

- The RX/TX runtime rewrite without an async runtime, the shared wait policy, the off-critical-path loss reporter, signal handling, `--bytes`, `--ipd-ns` and `--interval`, the adapter-MTU datagram-size default with one warning, and the gap-amplification fix are landed and documented in the [specification](../docs/spec.md) and [design](../docs/design.md).
- The multi-lane RX loss regression is resolved: on the two 10GbE hosts the remaining loss equals the RX adapter's receive misses ([design Measured Link Behavior](../docs/design.md#measured-link-behavior), [known issues](../docs/known-issues.md#register)).
- The Linux x86_64 requalification runs are done and recorded in the design's Measured Link Behavior; qualification itself is not yet accepted.
- The README carries the tool's purpose and the measured good/better/best command lines.
- The qualification link sweep (72 runs, build `908a81d`) is recorded in design Measured Link Behavior, with the RX-adapter outage registered as `KI-13` in the known issues; raw data is host-local under `~/.local/state/socket-mux/sweeps/20261030T011330Z/` on `chuck`.

Current next action: ask the principal to accept the Linux x86_64 requalification (gate 3; plan step `P3`). Jumbo MTU stays deferred until the Windows peer exists ([roadmap](../docs/roadmap.md) `R-36`).

Sources a successor reads: the [Authority Index](../docs/index.md) and the owners it routes ([goals](../docs/goals.md), [specification](../docs/spec.md), [design](../docs/design.md), [constraints](../docs/constraints.md), [roadmap](../docs/roadmap.md), [known issues](../docs/known-issues.md)), the README, and the installed graph under `.doc-graph/`.

## Rulings no durable owner carries

| Ruling                                                                                                                                                                                               | Status                                 | Effect                                                                                                                  | Owner or gate                                                                                                                                                                                                                                    |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Open owner questions do not go in the roadmap. Questions that authority cannot answer go to a decisions register, the owner decides there, and decided entries are drained into the authority basis. | Ratified by the principal (2026-10-30) | Do not add `Proposal` or undecided prospects to the roadmap; resolve from authority first, escalate only the remainder. | Pending: the installed doc-graph policies `pol-doc-roadmap`, `pol-doc-decisions` and `pol-doc-design` still say otherwise; the working-tree change to `docs/index.md` that states this flow is uncommitted until the graph change is authorized. |
| Test data and logs stay off `/mnt/shared`.                                                                                                                                                           | Ratified                               | Use `~/.local/state/socket-mux/` on `chuck` and `/tmp/smq/` on `spike`.                                                 | Principal                                                                                                                                                                                                                                        |
| No push, tag or release; no sudo. Admin scripts are local, read-only, run as a normal user and print the command for the administrator.                                                              | Ratified                               | Leave publication and host administration to the principal.                                                             | Principal                                                                                                                                                                                                                                        |
| Use subagents for bounded independent work; keep independent evaluation.                                                                                                                             | Ratified                               | Delegate code and doc work in separate clones or disjoint paths; integrate before verification.                         | Principal                                                                                                                                                                                                                                        |
| Ask the owner one question at a time, without identifiers, grouped by topic, with context, 2–4 options and a recommended default.                                                                    | Ratified                               | Use only for consequential decisions that authority cannot answer.                                                      | Principal                                                                                                                                                                                                                                        |

## Open decisions and gates

1. Doc-graph guidance change. Surface: `.doc-graph` policies `pol-doc-roadmap` (Purpose, `document.roadmap.horizon`, Validation), `pol-doc-decisions` (admit a `pending` open-question entry), `pol-doc-design` (route undecided mechanisms to the decisions register), and consequential repairs in `pol-dg-current-state` and the project-configuration contract. Defect: these policies let the roadmap hold undecided choices, contrary to the ruling above. Decision needed: whether the principal authorizes a governed change in the doc-graph repository (recommended: a separate clone on a new branch, leaving the other writer's uncommitted change on `rebase/next` untouched), with independent evaluation through `op-dg-route-governance-change`, then a swe-skills pointer update. Next action: ask the principal; on authorization, apply via swe-improve. Until then keep `docs/index.md` uncommitted.
1. Meaning of `decisions.git-tracking: ignored`. Today it selects the design-decision identifier form only. Decision needed: whether it also means the decisions register is untracked (recommended), or a separate setting is added. Owner: principal. Next action: ask together with gate 1; the answer enters the doc-graph project-configuration contract through the gate 1 change.
1. Linux x86_64 qualification acceptance. Evidence: design Measured Link Behavior (the two-direction matrix at 1 and 4 lanes on `200f8e4`, and the link sweep on `908a81d`). Criterion: the principal accepts that evidence with `KI-10` (the two 4-lane file runs lost 10 and 6 datagrams, `memory` cause, attributed to `KI-10` by inference because their adapter counters were not recorded, so their MD5 sums differ) and `KI-13` as the stated caveats, and decides whether a file run with `KI-10` loss counts as a pass. Next action: put the acceptance question to the principal. Owner: principal. On acceptance: update specification Boundaries.
1. RX-adapter outage (`KI-13` in [known issues](../docs/known-issues.md#register)). Defect: in 4 of 72 sweep runs the RX host's adapter statistics went backwards or jumped while RX lost thousands to over a million datagrams at once; cause undiagnosed. Criterion: the cause is established from the RX host's kernel log and adapter statistics. Next action: ask an administrator on `chuck` to capture `sudo dmesg | grep enp191s0` and `sudo ethtool -S enp191s0` before and after a reproducing run. Owner: principal (administrator access); feeds plan step `P6`.
1. Candidate improvement, not admitted: RX could declare a gap lost as soon as a later datagram arrives on the same lane (lanes rarely reorder), so loss near the stream end does not wait the full timeout. This changes specification Semantics. Owner: principal. Criterion: whether earlier loss declaration could misclassify lane reordering (compare `KI-05` in [known issues](../docs/known-issues.md#register)). Next action: once gate 1 settles where the decisions register lives, enter the question there and put it to the principal; no code before the decision.
1. The Windows code paths (console-event mapping, thread priority, adapter MTU through `GetAdaptersAddresses`) have not been compiled. Owner: principal (peer access). Criterion: roadmap `R-42`. Next action: cross-compile `make win64-x86_64` and run it on the Windows peer when access exists.

## Believed action authority

The principal granted: edit `src/`, tests, `docs/`, `README.md`, `.planning/` and `scripts/`; commit locally as `Tomas Gonzalez <tegonzalez@gmail.com>` with trailer `Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>`; SSH to `chuck` (192.168.7.40) as a normal user and run copied binaries there. Not granted: push, tag, release, sudo, edits to `.doc-graph` or the swe-skills repository (gate 1). Do not stage the untracked `.agents`, `.doc-graph`, `AGENTS.md` or `external`. `udp-impair.py` and `vsm` under `.cursor/skills/verify-socket-mux/scripts/` belong to another writer.

## Claims

Measured at revision `eac5509`; code is unchanged through `1739a92` (working tree: `docs/index.md` modified, uncommitted).

- All tests pass: `cargo test --release` reports 122 passed, 0 failed.
- No async runtime dependency: `grep -c tokio Cargo.toml Cargo.lock` prints `0` for each file.
- Source comments cite no retired identifiers: `grep -rnE "SM-[A-Z]+-?[0-9]+" src tests Cargo.toml` prints nothing.
- Both hosts allow 64 MiB socket buffers until reboot: `cat /proc/sys/net/core/rmem_max` prints `67108864` on each host (`ssh chuck cat /proc/sys/net/core/rmem_max`).

## Currency

No longer in force; do not reimport:

- The async-runtime (Tokio) I/O model and the per-gap `loss gap` / `rx totals` info lines.
- `SM-*` document identifiers.
- The claim that host socket-buffer limits cannot be raised above 4 MiB; the fixed 1452-byte datagram default (now derived from the adapter MTU, 1452 only as fallback).

## Resumption and retirement

Re-run `./.agents/init.md` and complete graph initialization before any governed mutation. Read this instance and the durable sources above, re-check the claims against the current revision and report drift. Start with gate 3 and the open questions in gates 1 and 2. This instance retires when its successor has resumed and the first pass under it has landed; surviving meaning goes to its current owners before a replacement is authored.
