---
doc-graph: "When there is an intention to amend this document, first Adhere to [dc-doc-known-issues]; otherwise, do not modify."
---

# socket-mux Known Issues

## Purpose

Register the admitted, finite residuals in socket-mux TX and RX behavior that each name one owning durable specification and would change a product or verification outcome once that owner absorbs them.

## Orientation

The audience is a reviewer or implementer of socket-mux who needs to know which current residuals to watch, and an author of the owning document who decides what it must say next. This register records what each residual is and which document owns it. It states no required behavior, sequences no work and declares no readiness; a missing rule is added to the owner named in the entry. Proposals are in the [roadmap](roadmap.md).

## Terminology

| Term            | Meaning                                                                                                              |
| --------------- | -------------------------------------------------------------------------------------------------------------------- |
| Reorder buffer  | The datagrams RX holds out of order, keyed by global sequence number, until it delivers them or declares a gap lost. |
| Loss acceptance | The rule by which RX stops waiting for a missing slice and declares it lost, such as a timeout or a memory cap.      |

## Register

| id         | datetime             | kind             | restatement                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | owner                    | seed     | disposition                             | retirement                                                                                                        |
| ---------- | -------------------- | ---------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------ | -------- | --------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| `SM-KI-01` | 2026-10-22T00:16:33Z | deferred-surface | RX's reorder buffer (the RX BTreeMap of pending datagrams) has no specified bound on its memory (RAM) use. RX holds every datagram after a missing one until the missing one's inter-packet delay passes the timeout (design SM-D19), default 10 s, so at a high rate it can hold many; whether a bound on held datagrams declares a gap lost sooner, and what that bound is, is a gap ([OD-22](open-decisions.md)); the loss output proposal `SM-D13` and the bounded reassembly ring proposal [SM-R5](roadmap.md#sm-r5-proposal-slab-pool-and-reassembly-ring) are not approved. Until that is decided, RX's reorder-buffer memory use must be watched and managed. | [Specification](spec.md) | unseeded | admitted; open while OD-22 is undecided | The specification states, without a Gap or PROPOSAL mark, the bound on the memory RX holds for pending datagrams. |

## Boundaries

This register holds only admitted residuals in accepted TX and RX behavior. Desired evolution is in the [roadmap](roadmap.md), current behavior in the [specification](spec.md), and mechanism in the [design](design.md). Implementation holes, task lists and test cases are not admitted here.
