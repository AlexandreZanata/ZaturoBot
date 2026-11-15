// Fixed private renderer. No model tool, evaluation API, screenshot path, trace or video file.
import dns from 'node:dns/promises';
import net from 'node:net';
import {pathToFileURL} from 'node:url';

export function publicAddress(address) {
  if (net.isIP(address) === 4) {
    const [a, b, c] = address.split('.').map(Number);
    return !(a === 0 || a === 10 || a === 127 || a >= 224 ||
      (a === 100 && b >= 64 && b <= 127) || (a === 169 && b === 254) ||
      (a === 172 && b >= 16 && b <= 31) || (a === 192 && (b === 168 || b === 0)) ||
      (a === 198 && (b === 18 || b === 19 || (b === 51 && c === 100))) ||
      (a === 203 && b === 0 && c === 113));
  }
  if (net.isIP(address) === 6) {
    const value = new URL(`http://[${address}]/`).hostname.slice(1, -1).toLowerCase();
    const first = parseInt(value.split(':')[0], 16);
    const second = parseInt(value.split(':')[1] || '0', 16);
    return first >= 0x2000 && first <= 0x3fff && !(first === 0x2001 && (second < 0x200 || second === 0xdb8)) && first !== 0x2002;
  }
  return false;
}

export async function checkedUrl(raw, lookup = dns.lookup, websocket = false) {
  if (typeof raw !== 'string' || Buffer.byteLength(raw) > 2048 || /[\s\\\x00-\x1f\x7f]/.test(raw)) throw Error('url');
  const url = new URL(raw);
  if (!(websocket ? ['ws:', 'wss:'] : ['http:', 'https:']).includes(url.protocol) || url.username || url.password ||
      !url.hostname || /(^|\.)(localhost|local|internal)$/i.test(url.hostname.replace(/\.$/, '')) || !['', '80', '443'].includes(url.port)) throw Error('url');
  const host = url.hostname.replace(/^\[|\]$/g, '');
  const records = net.isIP(host) ? [{address: host}] : await lookup(host, {all: true, verbatim: true});
  if (!records.length || records.some(record => !publicAddress(record.address))) throw Error('address');
  return url.href;
}

export async function render(input, chromium, validate = checkedUrl) {
  const url = await validate(input.url);
  if (!Number.isSafeInteger(input.expires_at) || input.expires_at <= Date.now()) throw Error('expired');
  let context;
  let png;
  try {
    const env = Object.fromEntries(Object.entries(process.env).filter(([name]) =>
      ['HOME', 'USER', 'LOGNAME', 'PATH', 'LANG', 'DISPLAY', 'WAYLAND_DISPLAY', 'XAUTHORITY', 'XDG_RUNTIME_DIR', 'DBUS_SESSION_BUS_ADDRESS'].includes(name) || name.startsWith('LC_')));
    context = await chromium.launchPersistentContext(input.profile, {
      executablePath: input.chrome, headless: true, chromiumSandbox: true,
      viewport: {width: 1280, height: 720}, ignoreHTTPSErrors: false,
      acceptDownloads: false, serviceWorkers: 'block', timeout: 7000, env,
      args: ['--disable-breakpad', '--disable-crash-reporter'],
    });
    await context.route('**/*', async route => {
      try { await validate(route.request().url()); await route.continue(); }
      catch { await route.abort('blockedbyclient'); }
    });
    await context.routeWebSocket('**/*', async route => {
      try { await validate(route.url(), dns.lookup, true); route.connectToServer(); }
      catch { route.close(); }
    });
    const pages = context.pages();
    const page = pages.find(candidate => candidate.url() === url) ?? pages[0] ?? await context.newPage();
    for (const other of pages) if (other !== page) await other.close();
    page.on('dialog', dialog => { void dialog.dismiss(); });
    await page.goto(url, {waitUntil: 'domcontentloaded', timeout: 10000});
    // Give bounded asynchronous page resources a chance to paint; never wait indefinitely.
    await page.waitForLoadState('load', {timeout: 1500}).catch(() => {});
    await validate(page.url());
    const selected = page.url();
    png = await page.screenshot({type: 'png', fullPage: false, timeout: 5000});
    if (page.url() !== selected || Date.now() >= input.expires_at || png.length > 8 * 1024 * 1024) throw Error('changed');
    return png;
  } catch (error) {
    png?.fill(0);
    throw error;
  } finally {
    if (context) { try { await context.close(); } catch (error) { png?.fill(0); throw error; } }
  }
}

async function main() {
  let text = '';
  const input = await new Promise((resolve, reject) => {
    process.stdin.setEncoding('utf8');
    const read = chunk => {
      text += chunk;
      if (Buffer.byteLength(text) > 8192) { reject(Error('bound')); return; }
      if (text.includes('\n')) {
        process.stdin.removeListener('data', read);
        try { resolve(JSON.parse(text.trim())); } catch { reject(Error('json')); }
      }
    };
    process.stdin.on('data', read); process.stdin.on('error', reject);
  });
  let image;
  try {
    const {chromium} = await import(pathToFileURL(input.module).href);
    image = await render(input, chromium);
    const length = Buffer.alloc(4); length.writeUInt32BE(image.length);
    await new Promise((resolve, reject) => process.stdout.write(length, error => error ? reject(error) : resolve()));
    await new Promise((resolve, reject) => process.stdout.write(image, error => error ? reject(error) : resolve()));
    image.fill(0);
    await new Promise(resolve => process.stdin.once('end', resolve));
  } finally { image?.fill(0); }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch(() => { process.stderr.write('private_browser_capture_unavailable\n'); process.exitCode = 1; process.stdin.destroy(); });
}
