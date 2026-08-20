# proxy-m365-write

Governed Microsoft 365 write path for a two-VM OpenShell/OpenClaw deployment. The first milestone can create Outlook drafts only. It cannot send mail, update or delete an item, add attachments, or access another Graph write endpoint.

## Approval flow

OpenClaw first runs `m365-draft propose`. That command saves a local proposal and prints its complete contents, ID, and SHA-256 digest; it makes no network request. After the user explicitly confirms that exact ID in a later message, OpenClaw may run `m365-draft approve --id ...`. The tool verifies the digest and invokes the loopback write path once.

```text
OpenClaw
  │ propose (local file only)
  └─ user confirms exact proposal ID
       │
       ▼
m365-draft approve ── bearer placeholder ──► agent forwarder (agent VM)
       ── OpenShell substitutes inter-VM capability ──► integration forwarder
       ──► draft-only Rust proxy (integration VM sandbox)
       ── Graph token placeholder ──► OpenShell substitutes OAuth token
       ──► POST graph.microsoft.com/v1.0/me/messages
```

The real Graph access and refresh tokens live only in the integration VM's OpenShell provider. The agent VM has only a separate, static inter-VM capability. Breaking out of the OpenClaw sandbox therefore does not reveal the Microsoft credential. Defense is layered: both forwarders require the exact method/path, the Rust proxy authenticates the inter-VM capability and validates/re-serializes a strict draft schema, and the integration OpenShell policy permits only the proxy binary to contact Graph.

The approval separation in `agent-tool/SKILL.md` is an agent workflow control. The independently enforced proxy boundary is create-draft-only. A future milestone can make approval itself cryptographically or externally enforced rather than relying on the skill/tool boundary.

## Development

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
docker build -t localhost/m365/proxy-m365-write:v1 -f Containerfile .
```

Runtime variables are `INTER_VM_BEARER_SHA256`, `CLIMICROSOFT365_ACCESS_TOKEN`, and optionally `LISTEN_ADDR` and `GRAPH_API_BASE`. The Graph variable must contain an OpenShell placeholder, never a real token. Render `TENANT_ID` and namespace/service names in the example deployment files for the target environment. Never commit OAuth material or the inter-VM bearer.

Microsoft delegated consent needs `Mail.ReadWrite` to create drafts. `Mail.Send` is neither requested nor needed.
