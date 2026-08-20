#!/usr/bin/env node
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, readdir, rename, writeFile } from 'node:fs/promises';
import path from 'node:path';

const proposalDir = process.env.M365_DRAFT_PROPOSAL_DIR || '/sandbox/.m365-write/proposals';
const endpoint = process.env.M365_DRAFT_ENDPOINT || 'http://127.0.0.1:18081/v1.0/me/messages';
const placeholderFile = process.env.M365_WRITE_PLACEHOLDER_FILE || '/sandbox/.m365-write/intervm-placeholder';

function fail(message) { console.error(message); process.exit(1); }
function values(args, name) {
  const result = [];
  for (let i = 0; i < args.length; i += 1) if (args[i] === name && args[i + 1]) result.push(args[++i]);
  return result.flatMap(value => value.split(',')).map(value => value.trim()).filter(Boolean);
}
function value(args, name) { const at = args.indexOf(name); return at >= 0 ? args[at + 1] : undefined; }
function proposalPath(id) {
  if (!/^[0-9a-f-]{36}$/i.test(id)) fail('Invalid proposal ID');
  return path.join(proposalDir, `${id}.json`);
}
function digest(draft) { return createHash('sha256').update(JSON.stringify(draft)).digest('hex'); }
function recipients(addresses) { return addresses.map(address => ({ emailAddress: { address } })); }
function summary(proposal) {
  return {
    id: proposal.id, status: proposal.status, digest: proposal.digest,
    subject: proposal.draft.subject,
    contentType: proposal.draft.body.contentType,
    body: proposal.draft.body.content,
    to: proposal.draft.toRecipients?.map(item => item.emailAddress.address) || [],
    cc: proposal.draft.ccRecipients?.map(item => item.emailAddress.address) || [],
    graphDraftId: proposal.graphDraftId,
  };
}
async function save(proposal) {
  await mkdir(proposalDir, { recursive: true, mode: 0o700 });
  const target = proposalPath(proposal.id);
  const temporary = `${target}.${process.pid}.tmp`;
  await writeFile(temporary, `${JSON.stringify(proposal, null, 2)}\n`, { mode: 0o600 });
  await rename(temporary, target);
}
async function load(id) { return JSON.parse(await readFile(proposalPath(id), 'utf8')); }

async function propose(args) {
  const subject = value(args, '--subject');
  const body = value(args, '--body');
  const contentType = value(args, '--content-type') || 'Text';
  if (!subject || body === undefined) fail('Usage: m365-draft propose --subject TEXT --body TEXT [--to EMAIL] [--cc EMAIL]');
  const draft = { subject, body: { contentType, content: body } };
  const to = recipients(values(args, '--to'));
  const cc = recipients(values(args, '--cc'));
  if (to.length) draft.toRecipients = to;
  if (cc.length) draft.ccRecipients = cc;
  const proposal = { version: 1, id: randomUUID(), status: 'proposed', createdAt: new Date().toISOString(), draft, digest: digest(draft) };
  await save(proposal);
  console.log(JSON.stringify(summary(proposal), null, 2));
  console.log(`\nNo Microsoft 365 write occurred. After the user confirms this exact proposal, run: m365-draft approve --id ${proposal.id}`);
}

async function approve(args) {
  const id = value(args, '--id');
  if (!id) fail('Usage: m365-draft approve --id PROPOSAL_ID');
  const proposal = await load(id);
  if (proposal.status !== 'proposed') fail(`Proposal is ${proposal.status}, not proposed`);
  if (digest(proposal.draft) !== proposal.digest) fail('Proposal integrity check failed');
  let token = process.env.M365_WRITE_INTERVM_BEARER;
  if (!token) {
    try { token = (await readFile(placeholderFile, 'utf8')).trim(); } catch { /* handled below */ }
  }
  if (!token) fail('M365_WRITE_INTERVM_BEARER is unavailable');
  const response = await fetch(endpoint, { method: 'POST', headers: { authorization: `Bearer ${token}`, 'content-type': 'application/json' }, body: JSON.stringify(proposal.draft) });
  const text = await response.text();
  if (!response.ok) fail(`Draft creation failed (${response.status}): ${text}`);
  const graph = JSON.parse(text);
  proposal.status = 'executed';
  proposal.executedAt = new Date().toISOString();
  proposal.graphDraftId = graph.id;
  await save(proposal);
  console.log(JSON.stringify(summary(proposal), null, 2));
}

async function main() {
  const [command, ...args] = process.argv.slice(2);
  if (command === 'propose') return propose(args);
  if (command === 'approve') return approve(args);
  if (command === 'show') return console.log(JSON.stringify(summary(await load(value(args, '--id'))), null, 2));
  if (command === 'list') {
    await mkdir(proposalDir, { recursive: true, mode: 0o700 });
    const files = (await readdir(proposalDir)).filter(name => name.endsWith('.json'));
    return console.log(JSON.stringify(await Promise.all(files.map(async name => summary(JSON.parse(await readFile(path.join(proposalDir, name), 'utf8'))))), null, 2));
  }
  fail('Commands: propose, show, list, approve');
}
await main();
