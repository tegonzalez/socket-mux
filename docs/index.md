---
doc-graph: "When there is an intention to amend this document, first Adhere to [dc-doc-index]; otherwise, do not modify."
---

# socket-mux Authority Index

## Purpose

Route readers and authors of socket-mux to the owners of its goals, roadmap, specification, design and known issues without restating them. The declared scope is the whole socket-mux tool: both the TX and RX modes. Start with the [goals](goals.md).

## Authority Basis

| Authority ID  | Priority | Class                 | Surface                         | Scope                                                                                                                                                                               | Dimensions                                                       | Disposition                        | Closure                                     | Expansion                            |
| ------------- | -------- | --------------------- | ------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- | ---------------------------------- | ------------------------------------------- | ------------------------------------ |
| `SM-AUTH-001` | 90       | `dc-doc-goals`        | [Goals](goals.md)               | socket-mux outcomes                                                                                                                                                                 | alignment; outcomes                                              | noncompeting; priority source      | `SM-AUTH-001`                               | new durable outcome                  |
| `SM-AUTH-002` | 60       | `dc-doc-spec`         | [Specification](spec.md)        | Consumer-visible behavior of TX and RX: public surface, command line, datagram header, invariants, semantics, diagnostics, platforms and gaps; proposed behavior is marked PROPOSAL | behavior; public-surface; command-line; terminology              | noncompeting; behavior owner       | `SM-AUTH-001 -> SM-AUTH-002`                | new public-surface entry or behavior |
| `SM-AUTH-003` | 45       | `dc-doc-design`       | [Design](design.md)             | Intended architecture and current design decisions with rationale                                                                                                                   | architecture; mechanisms; rationale                              | noncompeting; mechanism owner      | `SM-AUTH-001 -> SM-AUTH-002 -> SM-AUTH-003` | new mechanism or design decision     |
| `SM-AUTH-004` | 75       | `dc-doc-roadmap`      | [Roadmap](roadmap.md)           | Prospective evolution of TX and RX: every PROPOSAL awaiting a decision, with its prospective requirements, acceptance criteria and readiness; prospective applicability only        | future-evolution; proposals; requirements; acceptance; readiness | noncompeting; future-horizon owner | `SM-AUTH-001 -> SM-AUTH-004`                | new proposal or prospect             |
| `SM-AUTH-005` | 70       | `dc-doc-known-issues` | [Known issues](known-issues.md) | Admitted finite residuals in accepted TX and RX behavior, each against one owning specification                                                                                     | residual id; kind; owner; seed; disposition; retirement          | noncompeting; residual register    | `SM-AUTH-001 -> SM-AUTH-002 -> SM-AUTH-005` | new admitted residual                |

`Priority` is a declared integer `0` through `100` owned by this Authority Basis, and `100` is more governing. The numbers are the first-fill seeds for goals (90), roadmap (75), specification (60) and design (45): the goals are the shortest and most human-facing statement of what must hold, the specification states consumer-visible behavior that realizes them, and the design states mechanism beneath both. The known issues register (70) has no first-fill seed in DocGraph; 70 follows the swe-skills known issues register in this monorepo, and the register records residuals against the specification without competing with it. The roadmap's claims carry prospective applicability, so they do not compete with the current-baseline claims of the specification or the design. Competing claims with matching scope and applicability compare by these numbers; the lower owner is corrected, and equal integers escalate. Class, path, recency and table position do not rank.

## Query Routes

| Question                                       | Kind    | Ordered owners                         | Output                                                                                                      | Missing-input behavior            |
| ---------------------------------------------- | ------- | -------------------------------------- | ----------------------------------------------------------------------------------------------------------- | --------------------------------- |
| What must socket-mux achieve?                  | direct  | goals                                  | Outcomes                                                                                                    | Report the missing goals owner    |
| What does TX or RX accept, emit and guarantee? | derived | goals -> specification                 | Public surface, invariants and gaps                                                                         | Gate on the first missing owner   |
| How are TX and RX built, and why?              | derived | goals -> specification -> design       | Architecture, decisions and rationale                                                                       | Gate on the first missing owner   |
| Which proposals await approval?                | derived | goals -> roadmap                       | Prospects with disposition `proposed` or `deferred`, their prospective requirements and acceptance criteria | Report that no proposal exists    |
| How is the socket-mux CLI invoked?             | derived | goals -> specification                 | Grammar, options, precedence and exit statuses                                                              | Gate on the first missing owner   |
| Which residuals in current behavior are known? | derived | goals -> specification -> known issues | Admitted residuals with their owner, disposition and retirement condition                                   | Report that no known issue exists |

## Durable Boundary

The authority basis consists only of the listed owners. Open questions carry no authority; a question closes when its answer is drafted into the owning document above. Task assignment and work progress have no owner in this index.

Background research (bandwidth techniques, prior art and source excerpts) is in [research](research/); it is reference material and carries no authority.
