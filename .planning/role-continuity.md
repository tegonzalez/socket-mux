---
doc-graph: "When there is an intention to amend this document, first Adhere to [dc-doc-role-continuity]; otherwise, do not modify."
---

# Change implementer/coordinator continuity — 2026-10-30

## Role and task

This dated instance reconstitutes the standing Change implementer/coordinator role for "Two-host qualification and Windows-TX-to-Linux-RX throughput", the active Change of plan `.planning/plan.md`. The principal whose rulings govern this role is Tomas Gonzalez. No doc-graph role is specified and this instance selects no runtime binding. It replaces the earlier instance of the same date.

Finished work, not open items:

- The thread-per-lane runtime rewrite (no async runtime), the adapter-MTU datagram-size default and the multi-lane loss fix are landed and documented in the [specification](../docs/spec.md) and [design](../docs/design.md); plan steps `P1` and `P2` are done.
- The principal's 2026-10-30 rulings are drained into their owners: a transfer's success, including a qualification run, is judged by its total error rate with a 99.9 % pass line on a 10 Gbit/s single-switch link; a file transfer is judged by that rate, not by equal checksums; Linux x86_64 is qualified on the code of commit `908a81d`. The [specification Boundaries](../docs/spec.md#boundaries) carry all three, and plan step `P3` is accepted.
- The three gates of the earlier instance are closed: the doc-graph guidance change was authorized and made (see open item 2 for its merge); `decisions.git-tracking: ignored` now also means the decisions register is an untracked local intake register, a meaning the principal ratified; Linux x86_64 qualification is accepted.
- socket-mux conforms to the corrected decisions and roadmap rules: the [Authority Index](../docs/index.md#query-routes) routes an unanswerable question to the decisions intake register; the [roadmap](../docs/roadmap.md) holds only admitted prospects, with `R-34` narrowed to adapter receive tuning and admitted and `R-39` admitted; the former batching, quinn-udp and pinning prospects (`R-32`, `R-33`, `R-43`) are retired from the roadmap and stand as pending entries `OD-02` to `OD-04` of the local decisions register `.planning/decisions.md`, beside `OD-01`.
- `KI-13` is rediagnosed and registered in the [known issues](../docs/known-issues.md#ki-13-adapter-miss-bursts-at-stream-start) as adapter miss bursts at stream start, not adapter resets; the link-sweep harness reports the RX adapter's `rx_missed` delta modulo 65,536 because that r8169 counter is 16 bits wide.

Current next action: wait for the principal's answer to `OD-01` and to the 99.9 % pass-line confirmation (open items 1 and 3). The active Change cannot close while any decisions-register entry is `pending`. Work that does not depend on those answers, open items 2 and 4 to 9, may proceed only within the authority below; most of it waits on the principal, peer access or administrator access.

Sources a successor reads: the [Authority Index](../docs/index.md) and the owners it routes ([goals](../docs/goals.md), [constraints](../docs/constraints.md), [specification](../docs/spec.md), [design](../docs/design.md), [known issues](../docs/known-issues.md), [roadmap](../docs/roadmap.md)) and the installed graph under `.doc-graph/`. The plan and the decisions register are temporal surfaces this role acts on; this instance carries the meaning a successor needs from them.

## Rulings no durable owner carries

| Ruling                                                                                                                                 | Status   | Effect                                                                                                                          | Owner or gate |
| -------------------------------------------------------------------------------------------------------------------------------------- | -------- | ------------------------------------------------------------------------------------------------------------------------------- | ------------- |
| Test data and logs stay off `/mnt/shared`                                                                                              | Ratified | Use `~/.local/state/socket-mux/` on `chuck` and `/tmp/smq/` on `spike`                                                          | Principal     |
| No push, tag or release; no sudo; admin scripts are local, read-only, run as a normal user and print the command for the administrator | Ratified | Leave publication and host administration to the principal                                                                      | Principal     |
| Use subagents for bounded independent work; keep independent evaluation                                                                | Ratified | Delegate code and doc work in separate clones or disjoint paths; integrate before verification                                  | Principal     |
| Ask the owner one question at a time, without identifiers, grouped by topic, with context, 2–4 options and a recommended default       | Ratified | Use only for consequential questions the owners cannot answer; the index's query route carries the context, options and default | Principal     |

## Open items

1. Decisions register entries `OD-01` to `OD-04`, all `pending`. Surface: `.planning/decisions.md` (untracked by design; its entries carry each question verbatim and the owners consulted). Questions: `OD-01` whether RX declares a gap lost as soon as a later datagram arrives on the same lane, rather than waiting the full `--timeout` (changes specification Semantics; risk: misclassifying a same-lane reorder, compare `KI-05`); `OD-02` whether TX and RX batch several datagrams per system call (UDP GSO/GRO with `recvmmsg` on Linux, USO on Windows) instead of one per call (replaces design Thread-Per-Lane Blocking I/O); `OD-03` whether batching, if adopted, uses the quinn-udp crate rather than own code over socket2, libc and windows-sys (changes design Rust Crate Set); `OD-04` whether lane threads are pinned to cores, and if so whether the operator or socket-mux chooses them. Deciding authority: the principal. State: `OD-01` has been put to the principal with the recommended default "keep waiting the full timeout"; `OD-02` to `OD-04` have not been put. Next action: on an answer, record it verbatim in its entry and drain it into the owner that governs its subject (behavior into the specification, mechanism into the design); then put the next entry, one question at a time. No code follows an entry before its answer is drained. Gate: the active Change cannot close while an entry is `pending`.
1. Doc-graph and swe-skills merge. Surface: doc-graph commit `a3f5cd4` on branch `improve/decisions-intake` in the clone `/tmp/dg-improve`, and swe-skills commit `7c0660b` on branch `improve/intake-decisions-register` in the clone `/tmp/swe-intake`; each passed independent evaluation. They carry the principal's ratified rule that a question the authority basis cannot answer becomes a `pending` decisions-register entry, never a roadmap prospect, which socket-mux already states in its [Authority Index](../docs/index.md#query-routes). Neither commit exists in `/mnt/shared/projects/swe-skills` or its `.doc-graph`, and `/tmp` is not durable. Desired resolution: the principal merges doc-graph first, then moves the swe-skills `.doc-graph` pointer to include `a3f5cd4`, then merges the swe-skills branch. Owner: principal; not granted to this role. Next action: ask the principal to merge, or at least to fetch both branches into the canonical repositories, before `/tmp` is cleared.
1. Error-rate pass line. The [specification Boundaries](../docs/spec.md#boundaries) state a target of three to five nines and treat a run below 99.9 % as missing it; the principal was asked to confirm 99.9 % as the pass line and has not answered. Criterion: the principal's confirmation or another line. Next action: on a different answer, update every surface that states 99.9 % (the specification Terminology and Boundaries, the design, `KI-13` in the known issues, the README); on confirmation, nothing changes. Owner: principal.
1. `KI-13` adapter miss bursts at stream start. Defect: about 10 of 162 measured socket-mux runs, and 0 of 90 iperf3 runs, lose 16,908 to 1,630,967 datagrams in one RX-adapter miss burst at stream start, within the first 2 s wherever RX's interval lines locate it; mechanism unknown ([known issues](../docs/known-issues.md#ki-13-adapter-miss-bursts-at-stream-start)). Criterion: tracing tells whether socket-mux's start-of-stream behavior or its higher packet rate triggers the bursts. Next action: an administrator with root on `chuck` traces the receive path at stream start with `perf` or `ftrace` and samples `ethtool -S enp191s0` every second through the first seconds of a run. Owner: principal (root access); feeds plan step `P6`.
1. Windows peer, plan step `P4`, roadmap [`R-42`](../docs/roadmap.md#r-42-wanted-linux-aarch64-windows-1011-and-macos-qualification) and [`R-39`](../docs/roadmap.md#r-39-per-layer-measurement-plan). The Windows code paths (console-event mapping, thread priority, adapter MTU through `GetAdaptersAddresses`) have never been compiled. Next action: when the principal provides a headless Windows x86_64 peer, cross-compile `make win64-x86_64`, run the file and pattern qualification from Windows TX to Linux RX, and sweep with Windows send counters. Owner: principal (peer access).
1. Jumbo MTU, plan step `P5`, roadmap [`R-36`](../docs/roadmap.md#r-36-jumbo-mtu-end-to-end): deferred until the Windows peer and a jumbo-capable switch and adapter path exist. Next action: none until then.
1. Adapter tuning, plan step `P6`, roadmap [`R-34`](../docs/roadmap.md#r-34-linux-rx-adapter-receive-tuning): ring sizing and queue steering on `chuck` need an administrator; `scripts/admin/` prints the commands. Open question carried by `R-34`: the adapter's maximum ring and n-tuple and multi-queue support. Owner: principal (administration).
1. Intermittent test failure found at composition. In about 45 test-suite or single-test runs on `spike`, 3 failed: one was `full_speed_transfer_over_16_lanes_is_lossless` in `tests/bulk_transfer.rs`, where RX reported 3 `output`-cause lost datagrams on loopback; the other failure was not identified. Twenty consecutive runs of that test passed. Criterion: whether this is a test-environment flake or a product defect in the output limit. Next action: reproduce under load with the failing test name captured, then report to the principal before any code change. Owner: this role, within its test authority.
1. Optional or not attempted: augmenting the verification skill (`.cursor/skills/verify-socket-mux/`, another writer's) using `external/plugins/pstack` was requested earlier and not attempted, and its scope is not defined; ask the principal for scope first. An independent re-evaluation of commits `25ecd33..1cd7def` is optional.

Handoffs outside this role: the doc-graph change also requires conformance passes in ent-harness, aesa-demo-instr-layer and unify/aesa-demo; each needs its own authorization and is not granted here. Possible graph follow-ups, not admitted: the dg-rebuild skill may bundle unresolved choices as proposals, and roadmap `R-38`'s `deferred` kernel-bypass prospect may hide an undecided mechanism; both belong to the principal.

## Believed action authority

The principal granted: edit `src/`, tests, `docs/`, `README.md`, `.planning/` and `scripts/`; commit locally as `Tomas Gonzalez <tegonzalez@gmail.com>` with trailer `Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>`; SSH to `chuck` (192.168.7.40) as a normal user and run copied binaries there. Not granted: push, tag, release, sudo, merging into or editing the doc-graph or swe-skills repositories or the installed `.doc-graph`, or other consumers' conformance. Never stage `.agents`, `.doc-graph`, `AGENTS.md`, `external`, `.trash` or `.planning/decisions.md`. The `.cursor/skills/verify-socket-mux/` scripts belong to another writer.

## Claims

Measured on `spike` at revision `ad0febe`.

- Product code is unchanged since the qualified commit: `git diff --quiet 908a81d HEAD -- src tests Cargo.toml Cargo.lock && echo unchanged` prints `unchanged`.
- The suite has 122 tests and usually passes: `cargo test --release` reported 122 passed, 0 failed, in most runs, with the intermittent failure of open item 8.
- No async runtime dependency: `grep -c tokio Cargo.toml Cargo.lock` prints `0` for each file; no retired identifiers: `grep -rnE "SM-[A-Z]+-?[0-9]+" src tests Cargo.toml` prints nothing.
- The decisions register is untracked: `git check-ignore -v .planning/decisions.md` names `.gitignore`.
- Both hosts allow 64 MiB socket buffers until reboot: `cat /proc/sys/net/core/rmem_max` and `ssh chuck cat /proc/sys/net/core/rmem_max` print `67108864`.
- The deployed binaries match: `md5sum /tmp/smq/socket-mux` and `ssh chuck md5sum ~/.local/state/socket-mux/bin/socket-mux` print `ee9af2c27181a95b9570af5050406f34`. Test data lives under `/tmp/smq/` on `spike` and `~/.local/state/socket-mux/` (`runs/`, `sweeps/`) on `chuck`.
- The unmerged graph work is present: `git -C /tmp/dg-improve rev-parse --short improve/decisions-intake` prints `a3f5cd4`, and `git -C /tmp/swe-intake rev-parse --short improve/intake-decisions-register` prints `7c0660b`.

## Currency

No longer in force; do not reimport:

- A roadmap `Proposal` label or a `proposed` prospect holding an undecided choice; the installed graph's roadmap and design policies still allow it until open item 2 lands.
- Judging a file transfer by equal checksums, and the earlier gate asking the principal to accept Linux x86_64 qualification.
- Describing `KI-13` as an adapter outage or reset, and a raw `rx_missed` before-and-after difference as the adapter's miss count.
- The async-runtime (Tokio) I/O model, per-gap `loss gap` and `rx totals` info lines, and `SM-*` document identifiers.

## Resumption and retirement

Re-run `./.agents/init.md` and complete graph initialization before any governed mutation. Read this instance and the durable sources above, re-check the claims against the current revision and report drift. Start with open items 1 to 3. This instance retires when its successor has resumed and the first pass under it has landed; surviving meaning goes to its current owners before a replacement is authored.
