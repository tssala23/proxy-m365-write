import fs from 'node:fs';
import http from 'node:http';
import https from 'node:https';

const listenPort = Number(process.env.LISTEN_PORT ?? '18791');
const upstreamServerName = process.env.UPSTREAM_SERVER_NAME ?? 'default--proxy-m365-write--m365-write.openshell.localhost';
const ca = fs.readFileSync(process.env.OPEN_SHELL_CA_FILE ?? `${process.env.HOME}/.local/state/openshell/tls/ca.crt`);
const allowedPath = value => value === '/v1.0/me/messages' ||
  /^\/v1\.0\/me\/messages\/[A-Za-z0-9_.=%-]{1,512}\/send$/.test(value ?? '');
const server = http.createServer((request, response) => {
  if (request.method !== 'POST' || !allowedPath(request.url)) return response.writeHead(403).end();
  if (!request.headers['x-m365-write-bearer']) return response.writeHead(401).end();
  const upstream = https.request({
    hostname: '127.0.0.1', port: 17670, path: request.url, method: 'POST', servername: upstreamServerName, ca,
    headers: { host: upstreamServerName, 'content-type': 'application/json', 'x-m365-write-bearer': request.headers['x-m365-write-bearer'] }, timeout: 30_000,
  }, upstreamResponse => { response.writeHead(upstreamResponse.statusCode ?? 502, upstreamResponse.headers); upstreamResponse.pipe(response); });
  upstream.on('timeout', () => upstream.destroy(new Error('upstream timeout')));
  upstream.on('error', () => { if (!response.headersSent) response.writeHead(502); response.end(); });
  request.pipe(upstream);
});
server.requestTimeout = 35_000;
server.headersTimeout = 10_000;
server.listen(listenPort, '0.0.0.0', () => console.log(`M365 write integration forwarder listening on ${listenPort}`));
