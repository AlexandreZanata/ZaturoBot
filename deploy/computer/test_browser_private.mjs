import {test} from 'node:test';
import assert from 'node:assert/strict';
import {checkedUrl, publicAddress, render} from './browser_private.mjs';
const lookup = async () => [{address:'93.184.216.34'}];
test('parsed navigation rejects private targets, credentials and ambiguous authorities', async () => {
  assert.equal(await checkedUrl('https://example.test/a?fixture=one',lookup),'https://example.test/a?fixture=one');
  for (const url of ['file:///etc/passwd','javascript:alert(1)','http://127.0.0.1/','http://2130706433/','https://[::1]/','https://localhost./','https://a.local/','https://169.254.169.254/','https://user:pass@example.test/','https://example.test:9222/','https://example.test/\nsecret']) await assert.rejects(checkedUrl(url,lookup));
  await assert.rejects(checkedUrl('https://example.test',async()=>[{address:'93.184.216.34'},{address:'10.1.2.3'}]));
  for (const ip of ['0.0.0.0','192.168.0.1','100.64.1.2','198.18.1.2','2001::1','2001:0000::1','2001:db8::1','::ffff:127.0.0.1','fc00::1','fe80::1']) assert.equal(publicAddress(ip),false,ip);
  assert.equal(publicAddress('2001:4860:4860::8888'),true);
});
function fixture(change=false) {
  let selected='https://example.test/';let closed=false;let captureOptions;let launchOptions;
  const image=Buffer.alloc(100,42);
  const page={url:()=>selected,on:()=>{},waitForLoadState:async()=>{},goto:async value=>{selected=value;},screenshot:async options=>{captureOptions=options;if(change)selected+='changed';return image;}};
  const context={route:async()=>{},routeWebSocket:async()=>{},pages:()=>[page],close:async()=>{closed=true;}};
  const chromium={launchPersistentContext:async(_,options)=>{launchOptions=options;return context;}};
  return {image,chromium,verify:()=>{assert.equal(closed,true);assert.equal(launchOptions.chromiumSandbox,true);assert.equal(launchOptions.serviceWorkers,'block');assert.equal(launchOptions.acceptDownloads,false);assert.equal('path' in captureOptions,false);assert.equal(captureOptions.fullPage,false);}};
}
test('renderer returns only a bounded buffer and closes its sandboxed context',async()=>{
  const f=fixture();const result=await render({url:'https://example.test/',expires_at:Date.now()+20000},f.chromium,async v=>v);
  assert.equal(result,f.image);f.verify();result.fill(0);
});
test('URL change during capture erases the image and closes the context',async()=>{
  const f=fixture(true);await assert.rejects(render({url:'https://example.test/',expires_at:Date.now()+20000},f.chromium,async v=>v));
  assert.ok(f.image.every(v=>v===0));f.verify();
});
