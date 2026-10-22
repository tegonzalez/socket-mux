---
doc-graph: "When there is an intention to amend this document, first Adhere to [dc-doc-goals]; otherwise, do not modify."
---

# socket-mux Goals

## Purpose

Define the durable outcomes of socket-mux, a tool that carries one stream over several datagram lanes, for anyone deciding what to build, review or accept for it.

## Goals

- G1. One incoming local stream is split efficiently into multiple datagram lanes and reconstituted into the original stream at the receiving side.
- G2. One tool handles both the transmitting side and the receiving side.
- G3. The receiving side sequences the stream's slices on reconstitution and determines packet loss and the length of that loss.
- G4. Lost data is not retransmitted at this stage, so the channel is lossy.
- G5. The number of lanes is configurable, with a minimum of three and a configurable default.
- G6. The transmitting and receiving sides can be placed back to back in a verification loop. In one such loop the transmitting side generates a known pseudorandom pattern, and the receiving side's output is validated against that pattern, recovering even under adversarial UDP packet loss.
- G7. The tool is cross-platform.
