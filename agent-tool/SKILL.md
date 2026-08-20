---
name: microsoft365-draft-approval
description: Propose and, only after explicit user confirmation, create Microsoft 365 Outlook drafts through the governed write proxy.
---

# Microsoft 365 draft approval

Use `/sandbox/bin/m365-draft` for every Microsoft 365 write. This capability can only create Outlook drafts; it cannot send messages, edit existing items, or delete anything.

1. Run `m365-draft propose --subject '...' --body '...'` with optional repeatable `--to` and `--cc` arguments.
2. Show the user the complete subject, body, recipients, proposal ID, and digest emitted by the tool. State that no write has occurred.
3. Stop and ask the user to confirm that exact proposal ID. Never infer confirmation from the original request to compose or draft content.
4. Only after a later user message explicitly confirms that proposal, run `m365-draft approve --id PROPOSAL_ID`.
5. Report the resulting Graph draft ID. Never call `approve` twice for the same proposal.

Do not use `m365 request`, `curl`, or another program to perform Microsoft 365 writes. Never expose or print `M365_WRITE_INTERVM_BEARER`.
