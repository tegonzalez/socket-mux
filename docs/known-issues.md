---
doc-graph: "When there is an intention to amend this document, first Adhere to [dc-doc-known-issues]; otherwise, do not modify."
---

# socket-mux Known Issues

## Purpose

Register the admitted, finite residuals in socket-mux TX and RX behavior that each name one owning durable specification and would change a product or verification outcome once that owner absorbs them.

## Orientation

The audience is a reviewer or implementer of socket-mux who needs to know which current residuals to watch, and an author of the owning document who decides what it must say next. This register records what each residual is and which document owns it. It states no required behavior, sequences no work and declares no readiness; a missing rule is added to the owner named in the entry. Prospects are in the [roadmap](roadmap.md).

## Terminology

| Term            | Meaning                                                                                                                                              |
| --------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| Reorder buffer  | The datagrams RX holds out of order in its wheel ([design](design.md#rx-wheel-arena-and-fifo)) until it delivers them or declares a gap lost.        |
| Loss acceptance | The rule by which RX stops waiting for a missing slice and declares it lost: timeout rule A or B, or the arena limit.                                |

## Register

| id         | datetime             | kind              | restatement                                                                                                                                                                                                                                                                            | owner                    | seed     | disposition                                                 | retirement                                                                                                                                        |
| ---------- | -------------------- | ----------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------ | -------- | ----------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| `SM-KI-05` | 2026-10-24T16:35:07Z | accepted-tradeoff | The arena limit declares a missing run lost before the timeout T when a datagram arrives W or more sequence numbers ahead of next, so a reorder deeper than W or a burst that outruns the arena loses datagrams that would have arrived within T ([specification](spec.md#semantics)). | [Specification](spec.md) | unseeded | admitted                                                    | The specification no longer declares a run lost before T under the arena limit.                                                                   |
| `SM-KI-06` | 2026-10-24T16:35:07Z | accepted-tradeoff | When no arena buffer is free, for example while the TCP output is stalled, the drain limit drops each newest datagram, so a slow consumer turns into loss ([specification](spec.md#semantics)).                                                                                        | [Specification](spec.md) | unseeded | admitted                                                    | The specification no longer drops datagrams while the arena is full.                                                                              |

## Boundaries

This register holds only admitted residuals in accepted TX and RX behavior. Desired evolution is in the [roadmap](roadmap.md), current behavior in the [specification](spec.md), and mechanism in the [design](design.md). Implementation holes, task lists and test cases are not admitted here.
