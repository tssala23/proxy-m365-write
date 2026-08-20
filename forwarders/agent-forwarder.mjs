import http from 'node:http';

const upstream = process.env.M365_WRITE_UPSTREAM ?? 'http://taj2-int-m365-write.saw-taj2.svc.cluster.local:18791';
const maxBytes = 64 * 1024;

const server = http.createServer(async (request, response) => {
  if (request.method !== 'POST' || request.url !== '/v1.0/me/messages') return response.writeHead(403).end();
  const authorization = request.headers.authorization;
  if (typeof authorization !== 'string') return response.writeHead(401).end();
  const chunks = []; let size = 0;
  for await (const chunk of request) {
    size += chunk.length;
    if (size > maxBytes) return response.writeHead(413).end();
    chunks.push(chunk);
  }
  try {
    const upstreamResponse = await fetch(upstream + request.url, {
      method: 'POST',
      headers: { 'content-type': 'application/json', 'x-m365-write-bearer': authorization.replace(/^Bearer\s+/i, '') },
      body: Buffer.concat(chunks), signal: AbortSignal.timeout(35_000),
    });
    const body = Buffer.from(await upstreamResponse.arrayBuffer());
    response.writeHead(upstreamResponse.status, { 'content-type': upstreamResponse.headers.get('content-type') ?? 'application/json', 'content-length': body.length });
    response.end(body);
  } catch { response.writeHead(502).end(); }
});
server.listen(18081, '127.0.0.1', () => console.log('M365 write agent forwarder listening on 127.0.0.1:18081'));
