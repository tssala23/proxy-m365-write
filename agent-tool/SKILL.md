---
name: microsoft365-draft-approval
description: Propose and, only after separate explicit confirmations, create and send Microsoft 365 Outlook drafts through the governed write proxy.
---

# Microsoft 365 draft approval

Use `/sandbox/bin/m365-draft` for every Microsoft 365 write. It can create a draft and send that identified draft, but cannot edit or delete items.

1. Run `m365-draft propose --subject '...' --body '...'` with optional repeatable `--to` and `--cc` arguments.
2. Show the user the complete subject, body, recipients, proposal ID, and digest emitted by the tool. State that no write has occurred.
3. Stop and ask the user to confirm that exact proposal ID. Never infer confirmation from the original request to compose or draft content.
4. Only after a later user message explicitly confirms that proposal, run `m365-draft approve --id PROPOSAL_ID`.
5. Report the resulting Graph draft ID. Never call `approve` twice for the same proposal.

Creating a draft never authorizes sending it. For sending:

1. Run `m365-draft propose-send --draft-proposal-id CREATE_PROPOSAL_ID` only for a draft created by this tool.
2. Show the send proposal ID, Graph draft ID, source proposal ID, and draft digest. Clearly state that nothing has been sent.
3. Stop and require a later user message that explicitly confirms that exact send proposal ID.
4. Only then run `m365-draft approve-send --id SEND_PROPOSAL_ID` and report success.

Never infer send approval from approval to create the draft, from a request to compose content, or from approval of a different proposal.

Do not use `m365 request`, `curl`, or another program to perform Microsoft 365 writes. Never expose or print `M365_WRITE_INTERVM_BEARER`.
