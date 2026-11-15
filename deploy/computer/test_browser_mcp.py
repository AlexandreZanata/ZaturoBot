"""Real stdio plus loopback CDP fixtures; no installed browser, external site or paid call."""
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from unittest.mock import patch
import uuid

sys.path.insert(0, str(Path(__file__).parent))
import browser_mcp as browser
import linux_mcp as base

FAKE_NODE=r'''#!/usr/bin/python3
import json,sys
from pathlib import Path
root=Path(__file__).parent
assert Path(sys.argv[1]).name=='cli.js'
assert '--cdp-endpoint' in sys.argv and '--image-responses' in sys.argv and 'omit' in sys.argv
assert '--snapshot-mode' in sys.argv and 'none' in sys.argv and '--no-webmcp' in sys.argv
for line in sys.stdin:
 request=json.loads(line)
 with (root/'calls.jsonl').open('a') as log:log.write(json.dumps(request)+'\n')
 if request['method']=='notifications/initialized':continue
 if request['method']=='initialize':result={'protocolVersion':'2025-11-25','serverInfo':{'name':'Playwright','version':'1.66.0-alpha-2026-11-09'},'capabilities':{'tools':{}}}
 else:
  params=request['params'];name=params['name'];args=params['arguments'];state=json.loads((root/'state.json').read_text())
  if name=='browser_snapshot':text=state['snapshot']
  elif name=='browser_navigate':
   state['tabs'][0]['url']=args['url'];(root/'state.json').write_text(json.dumps(state));text='Navigation dispatched.'
  elif name in ['browser_click','browser_type','browser_press_key']:
   if name=='browser_type':assert args['submit'] is False
   text='SYNTHETIC_INTERNAL_RESPONSE_NOT_A_COMPLETION_PROOF'
  else:raise RuntimeError('Forbidden tool forwarded')
  result={'isError':False,'content':[{'type':'text','text':text},{'type':'image','mimeType':'image/png','data':'SYNTHETIC_IMAGE_NEVER_FORWARD'}]}
 print(json.dumps({'jsonrpc':'2.0','id':request['id'],'result':result}),flush=True)
'''


class BrowserContracts(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory(prefix='zaturo-browser-fixture-');self.root=Path(self.temp.name)
        self.state={'tabs':[{'id':'SYNTHETIC_PAGE','type':'page','url':'https://shop.example/search'}],'snapshot':'### Page\n- Page URL: https://shop.example/search\n### Snapshot\n- textbox "Search" [ref=e1]\n- button "Find" [ref=e2]\n- textbox "Password" [ref=e3] secret\n'}
        self.save()
        root=self.root
        class Handler(BaseHTTPRequestHandler):
            def log_message(self,*_):pass
            def do_GET(self):
                if self.path!='/json/list':self.send_error(404);return
                state=json.loads((root/'state.json').read_text())
                if state.get('http_error'):self.send_error(403,'synthetic-private-body-never-forward');return
                data=json.dumps(state['tabs']).encode();self.send_response(200);self.send_header('Content-Length',str(len(data)));self.end_headers();self.wfile.write(data)
        self.http=ThreadingHTTPServer(('127.0.0.1',0),Handler);self.thread=threading.Thread(target=self.http.serve_forever,daemon=True);self.thread.start()
        self.endpoint='http://127.0.0.1:'+str(self.http.server_port)
        profile=self.root/'dedicated-profile';profile.mkdir(mode=0o700)
        self.browser=subprocess.Popen(['chrome','-c','import time; time.sleep(120)','--user-data-dir='+str(profile),'--remote-debugging-port='+str(self.http.server_port),'--remote-debugging-address=127.0.0.1'],executable=sys.executable,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
        marker=profile/'.zaturobot-dedicated-browser';marker.write_text(json.dumps({'schema':1,'purpose':'zaturobot-browser','pid':self.browser.pid,'cdp_endpoint':self.endpoint}));marker.chmod(0o600)
        self.node=self.root/'node';self.node.write_text(FAKE_NODE);self.node.chmod(0o700)
        self.packages=self.root/'node_modules';self.packages.mkdir(mode=0o700);(self.packages/'@playwright').mkdir(mode=0o700)
        for name,version in browser.PACKAGES.items():
            directory=self.packages/name;directory.mkdir(parents=True,mode=0o700)
            metadata=directory/'package.json';metadata.write_text(json.dumps({'name':name,'version':version}));metadata.chmod(0o600)
        self.cli=self.packages/'@playwright/mcp/cli.js';self.cli.write_text('// pinned synthetic CLI artifact; never evaluated\n');self.cli.chmod(0o600)
        self.dependency=self.packages/'playwright-core/dependency.js';self.dependency.write_text('// pinned synthetic dependency\n');self.dependency.chmod(0o600)
        self.tree_hash=browser.package_tree_sha256(self.cli)
        self.config_path=self.root/'browser.toml'
        self.config_path.write_text('enabled=true\nallow_input=true\ntimeout_seconds=5\nstate_dir='+json.dumps(str(self.root/'journal'))+'\nnode='+json.dumps(str(self.node))+'\nnode_sha256='+json.dumps(hashlib.sha256(self.node.read_bytes()).hexdigest())+'\nplaywright_cli='+json.dumps(str(self.cli))+'\nplaywright_cli_sha256='+json.dumps(hashlib.sha256(self.cli.read_bytes()).hexdigest())+'\nplaywright_tree_sha256='+json.dumps(self.tree_hash)+'\ncdp_endpoint='+json.dumps(self.endpoint)+'\nprofile_dir='+json.dumps(str(profile))+'\nbrowser_pid='+str(self.browser.pid)+'\nallowed_origins=["https://shop.example"]\n');self.config_path.chmod(0o600)
        self.config=base.Config(self.config_path,backend='browser')
        self.original_dns=socket.getaddrinfo
        self.dns=patch.object(socket,'getaddrinfo',side_effect=lambda host,*a,**kw:self.original_dns(host,*a,**kw) if host=='127.0.0.1' else [(socket.AF_INET,socket.SOCK_STREAM,6,'',('93.184.216.34',443))]);self.dns.start()
    def tearDown(self):
        self.dns.stop();self.browser.terminate();self.browser.wait(timeout=3);self.http.shutdown();self.http.server_close();self.thread.join(3);self.temp.cleanup()
    def save(self):(self.root/'state.json').write_text(json.dumps(self.state))
    def request(self,tool='browser_snapshot',args=None):
        now=int(time.time()*1000)
        return {'operation_id':str(uuid.uuid4()),'lease_token':'synthetic-bound-browser-lease','lease_expires_at':now+20000,'deadline_at':now+20000,'owner_confirmed':True,'target':{'page_id':'SYNTHETIC_PAGE'},'tool':tool,'arguments':args or {}}
    def approved(self,args=None):
        snapshot=browser.execute(self.config,self.request());request=self.request('browser_type',args or {'element':'Search textbox','ref':'e1','text':'poco X7'});request['expected_snapshot_sha256']=snapshot['snapshot_sha256'];request['approved_plan_sha256']=base.plan_digest(request);return request
    def assert_code(self,code,request):
        with self.assertRaises(base.Rejected) as error:browser.execute(self.config,request)
        self.assertEqual(error.exception.code,code)
    def calls(self):return [json.loads(l) for l in (self.root/'calls.jsonl').read_text().splitlines()]

    def test_readonly_navigation_snapshot_and_exact_input_are_real_stdio(self):
        result=browser.execute(self.config,self.request('browser_navigate',{'url':'https://shop.example/?q=poco+X7'}));self.assertEqual(result['status'],'completed')
        self.assertNotIn('SYNTHETIC_IMAGE',json.dumps(result));self.assertNotIn('Password',json.dumps(result));self.assertNotIn('secret',result['data']['text'])
        request=self.approved();reply=browser.execute(self.config,request);self.assertTrue(reply['data']['dispatched']);self.assertFalse(reply['data']['verified_external_effect'])
        self.assertNotIn('SYNTHETIC_INTERNAL',json.dumps(reply));self.assert_code('reconciliation_required',request)
        mutations=[c for c in self.calls() if (c.get('params') or {}).get('name')=='browser_type'];self.assertEqual(len(mutations),1)

    def test_stale_page_snapshot_origin_credentials_and_private_hosts_are_denied(self):
        request=self.approved();self.state['snapshot']+='- button "Different page state" [ref=e4]\n';self.save();self.assert_code('stale_snapshot',request)
        self.state['tabs'][0]['id']='OTHER_PAGE';self.save();self.assert_code('page_changed',self.request())
        self.state['tabs'][0]['id']='SYNTHETIC_PAGE';self.state['tabs'][0]['url']='https://ungranted.example/';self.save();self.assert_code('origin_not_granted',self.request())
        self.state['tabs'][0]['url']='https://shop.example/';self.save()
        self.assert_code('url_credentials_forbidden',self.request('browser_navigate',{'url':'https://shop.example/?access_token=never-disclose'}))
        self.assert_code('public_https_required',self.request('browser_navigate',{'url':'http://127.0.0.1/'}))
        with patch.object(socket,'getaddrinfo',return_value=[(socket.AF_INET,socket.SOCK_STREAM,6,'',('127.0.0.1',443))]):
            self.assert_code('public_host_required',self.request('browser_navigate',{'url':'https://shop.example/'}))

    def test_code_file_cookies_multi_page_and_unapproved_input_cannot_cross(self):
        for tool in ['browser_evaluate','browser_run_code','browser_cookie_list','browser_file_upload','run_shell']:
            self.assert_code('tool_not_allowed',self.request(tool,{'code':'never execute'}))
        request=self.approved({'element':'Password','ref':'e3','text':'never-type-secret'});self.assert_code('stale_element',request)
        self.state['tabs'].append({'id':'SECOND','type':'page','url':'about:blank'});self.save();self.assert_code('single_dedicated_page_required',self.request())
        self.state['tabs'].pop();self.save();request=self.approved();request['arguments']['text']='changed';self.assert_code('exact_plan_required',request)

    def test_profile_marker_hash_and_local_stop_are_required(self):
        marker=self.root/'dedicated-profile/.zaturobot-dedicated-browser';marker.unlink();self.assert_code('dedicated_profile_required',self.request())
        marker.write_text(json.dumps({'schema':1,'purpose':'zaturobot-browser','pid':self.browser.pid,'cdp_endpoint':self.endpoint}));marker.chmod(0o600)
        (self.config.state/'STOP').write_text('stop');self.assert_code('emergency_stop',self.request());(self.config.state/'STOP').unlink()
        self.cli.write_text('// altered after approval');self.assert_code('artifact_hash_mismatch',self.request())

    def test_cdp_transport_errors_do_not_expose_endpoint_bodies(self):
        self.state['http_error']=True;self.save()
        with self.assertRaises(base.Rejected) as error:browser.execute(self.config,self.request())
        result=base.result_error(error.exception)
        self.assertEqual(result['code'],'dedicated_browser_unavailable');self.assertNotIn('synthetic-private',json.dumps(result))

    def test_dependency_mutation_behind_unchanged_cli_and_extra_files_are_refused(self):
        cli_hash=hashlib.sha256(self.cli.read_bytes()).hexdigest()
        self.dependency.write_text('// changed dependency with identical entry point\n')
        self.assertEqual(hashlib.sha256(self.cli.read_bytes()).hexdigest(),cli_hash)
        self.assert_code('package_tree_hash_mismatch',self.request())
        self.assertFalse((self.root/'calls.jsonl').exists())
        self.dependency.write_text('// pinned synthetic dependency\n')
        extra=self.dependency.with_name('unreviewed.js');extra.write_text('// extra module\n');extra.chmod(0o600)
        self.assert_code('package_tree_hash_mismatch',self.request());extra.unlink()
        (self.packages/'unreviewed-package').mkdir(mode=0o700)
        self.assert_code('unexpected_package_entry',self.request())
        self.assertFalse((self.root/'calls.jsonl').exists())

    def test_package_tree_symlinks_versions_limits_and_missing_hash_are_refused(self):
        original=self.dependency.read_bytes();self.dependency.unlink();self.dependency.symlink_to(self.cli)
        self.assert_code('unsafe_package_tree',self.request())
        self.dependency.unlink();self.dependency.write_bytes(original);self.dependency.chmod(0o600)
        metadata=self.packages/'playwright-core/package.json';metadata.write_text(json.dumps({'name':'playwright-core','version':'unreviewed'}))
        self.assert_code('package_version_mismatch',self.request())
        metadata.write_text(json.dumps({'name':'playwright-core','version':browser.PACKAGES['playwright-core']}))
        with patch.object(browser,'MAX_TREE_FILE_BYTES',1):
            self.assert_code('package_tree_too_large',self.request())
        self.config_path.write_text(self.config_path.read_text().replace('playwright_tree_sha256='+json.dumps(self.tree_hash)+'\n',''))
        with self.assertRaises(base.Rejected) as error:base.Config(self.config_path,backend='browser')
        self.assertEqual(error.exception.code,'package_tree_hash_required')
        self.assertFalse((self.root/'calls.jsonl').exists())

    def test_tree_hash_is_canonical_and_excludes_generated_npm_metadata(self):
        metadata=self.packages/'.package-lock.json';metadata.write_text('{"generated":true}');metadata.chmod(0o600)
        bins=self.packages/'.bin';bins.mkdir(mode=0o700);(bins/'playwright-mcp').symlink_to('../@playwright/mcp/cli.js')
        self.assertEqual(browser.package_tree_sha256(self.cli),self.tree_hash)
        metadata.write_text('{"generated":"updated without changing reviewed code"}')
        self.assertEqual(browser.package_tree_sha256(self.cli),self.tree_hash)
        mapping={str(p.relative_to(self.packages)):hashlib.sha256(p.read_bytes()).hexdigest() for name in browser.PACKAGES for p in (self.packages/name).rglob('*') if p.is_file()}
        self.assertEqual(browser.package_tree_sha256(self.cli),base.digest(mapping))
        traversal=self.cli.parent/'..'/'mcp'/'cli.js'
        with self.assertRaises(base.Rejected) as error:browser.package_tree_sha256(traversal)
        self.assertEqual(error.exception.code,'invalid_package_layout')

    def test_final_json_bound_cannot_authorize_refs_removed_from_observation(self):
        label='\\'*350
        self.state['snapshot']='\n'.join('- button "'+label+'" [ref=e'+str(index)+']' for index in range(20));self.save()
        observed=browser.execute(self.config,self.request())
        self.assertTrue(observed['data']['truncated'])
        self.assertLessEqual(len(base.encoded(observed['data'])),base.MAX_TEXT)
        self.assertNotIn('[ref=e19]',observed['data']['text'])
        request=self.request('browser_click',{'element':'Hidden control','ref':'e19'})
        request['expected_snapshot_sha256']=observed['snapshot_sha256'];request['approved_plan_sha256']=base.plan_digest(request)
        self.assert_code('stale_element',request)
        self.assertFalse(any((call.get('params') or {}).get('name')=='browser_click' for call in self.calls()))

    def test_unsafe_browser_switch_value_forms_are_refused(self):
        original=Path(f'/proc/{self.browser.pid}/cmdline').read_bytes()
        for flag in ('--no-sandbox=true','--disable-web-security=1','--remote-allow-origins=*'):
            with self.subTest(flag=flag),patch.object(Path,'read_bytes',return_value=original+flag.encode()+b'\0'):
                with self.assertRaises(base.Rejected) as error:browser.browser_identity(self.config)
                self.assertEqual(error.exception.code,'unsafe_browser_configuration')
        self.assertFalse((self.root/'calls.jsonl').exists())


if __name__=='__main__':unittest.main()
