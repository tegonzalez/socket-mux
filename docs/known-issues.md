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
| Reorder buffer  | The datagrams RX holds out of order in its wheel ([design](design.md#sm-d24-rx-wheel-arena-and-fifo)) until it delivers them or declares a gap lost. |
| Loss acceptance | The rule by which RX stops waiting for a missing slice and declares it lost: timeout rule A or B, or the arena limit.                                |

## Register

| id         | datetime             | kind              | restatement                                                                                                                                                                                                                                                                           | owner                    | seed     | disposition                                       | retirement                                                                                                                                        |
| ---------- | -------------------- | ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------ | -------- | ------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| `SM-KI-01` | 2026-10-22T00:16:33Z | deferred-surface  | RX's held datagrams need a bound on their memory use beyond the timeout T. The arena bounds them: RX holds at most W datagrams, and the arena and drain limits apply when it is full ([specification](spec.md#semantics)).                                                            | [Specification](spec.md) | unseeded | retired; the specification states the arena bound | The specification states, without a Gap or PROPOSAL mark, whether a byte or memory cap bounds the datagrams RX holds, and the cap if one applies. |
| `SM-KI-02` | 2026-10-22T07:28:00Z | accepted-tradeoff | The arena limit declares a missing run lost before the timeout T when a datagram arrives W or more sequence numbers ahead of next ([specification](spec.md#semantics)). A reorder deeper than W, or a burst that outruns the arena, loses datagrams that would have arrived within T. | [Specification](spec.md) | unseeded | admitted                                          | The specification no longer declares a run lost before T under the arena limit.                                                                   |
| `SM-KI-03` | 2026-10-22T07:28:00Z | accepted-tradeoff | When no arena buffer is free, for example while the TCP output is stalled, the drain limit drops each newest datagram ([specification](spec.md#semantics)). The wheel orders datagrams efficiently, but RX has no buffer beyond the arena, so a slow consumer turns into loss.        | [Specification](spec.md) | unseeded | admitted                                          | The specification no longer drops datagrams while the arena is full.                                                                              |
| `SM-KI-04` | 2026-10-22T07:28:00Z | deferred-surface  | Platform batching (recvmmsg, GRO, URO, RIO, io_uring) is deferred, so TX and RX do without it; roadmap prospects `SM-R2`, `SM-R4` and `SM-R8` cover it.                                                                                                                               | [Design](design.md)      | unseeded | admitted; deferred                                | The design states platform batching as a current decision or rules it out.                                                                        |

## Boundaries

This register holds only admitted residuals in accepted TX and RX behavior. Desired evolution is in the [roadmap](roadmap.md), current behavior in the [specification](spec.md), and mechanism in the [design](design.md). Implementation holes, task lists and test cases are not admitted here.
