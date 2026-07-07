# Open specifications

Specifications we publish for the wider ecosystem to read, critique, and implement --
not just documentation of how Ghostlight happens to work. The goal is for the good ideas
here to spread beyond this project. Everything in this directory is offered under
Apache-2.0 OR MIT so anyone may implement it freely.

## What is here

| Spec | What it is |
|---|---|
| [rawx-capability-model.md](rawx-capability-model.md) | The RAWX capability vocabulary (read, action, write, execute): a mechanism-independent, domain-neutral model for governing what an AI agent is allowed to do. `rwx` for agents. |
| [rawx-owasp-agentic-mapping.md](rawx-owasp-agentic-mapping.md) | An honest mapping of RAWX and the governance-overlay pattern onto the OWASP agentic threat taxonomy and the 2026 UW agentic-browser findings, including what a governance layer does NOT mitigate. |

More will follow as parts of the design prove general enough to stand on their own
(candidates: the resource-polarity grant format, the audit record schema, the layered
configuration model).

## Why publish specs at all

The durable asset in agent governance is the vocabulary, not the mechanism. Mechanisms
churn -- automation protocols, browser APIs, the surfaces agents act through. A shared
way to classify and grant agent capabilities outlives all of them, and is more valuable
as a common language than as a private one. If the RAWX vocabulary becomes something
other tools adopt, that is a win for the whole space, and Ghostlight is its reference
implementation.
