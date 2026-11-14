"""Synthetic real-pipe MCP contracts. Never attach to a desktop or invoke an installed agent."""
import base64
import importlib.util
import json
import os
from pathlib import Path
import signal
import struct
import subprocess
import sys
import tempfile
import time
import unittest
import uuid
import zlib

spec = importlib.util.spec_from_file_location("linux_mcp", Path(__file__).with_name("linux_mcp.py"))
bridge = importlib.util.module_from_spec(spec); spec.loader.exec_module(bridge)

FAKE = r'''#!/usr/bin/python3
import base64,json,os,sys,time,subprocess
from pathlib import Path
root=Path(__file__).parent
assert sys.argv[1:]==['mcp']
snapshot='process-'+str(os.getpid())
for line in sys.stdin:
 request=json.loads(line)
 with (root/'calls.jsonl').open('a') as out:out.write(json.dumps({'method':request['method'],'params':request.get('params'),'shell':os.environ.get('COMPUTER_USE_LINUX_ENABLE_SHELL'),'provider':os.environ.get('OPENCODE_GO_API_KEY'),'capture_backend':os.environ.get('COMPUTER_USE_LINUX_SCREENSHOT_BACKEND')})+'\n')
 if request['method']=='notifications/initialized':continue
 if request['method']=='initialize':result={'protocolVersion':'2025-11-25','serverInfo':{'name':'computer-use-linux','version':'0.7.53'},'capabilities':{'tools':{}}}
 else:
  params=request['params'];name=params['name'];args=params['arguments'];state=json.loads((root/'state.json').read_text())
  if name=='doctor':data={'readiness':{'can_query_windows':True,'can_build_accessibility_tree':True,'can_capture_screenshots':False,'recommended_next_step':'synthetic-private-path-never-return'}}
  elif name=='list_windows':data={'windows':[state['window']]}
  elif name=='get_app_state':
   if state.get('oversize'):
    print('x'*(4*1024*1024+100),flush=True);continue
   if state.get('hang'):
    child=subprocess.Popen([sys.executable,'-c','import time; time.sleep(120)'],start_new_session=True)
    (root/'escaped.pid').write_text(str(child.pid));time.sleep(120)
   data={'window_context':state['window'],'snapshot_id':snapshot,'tree_scoped':True,'accessibility_tree':state['nodes'],'accessibility_tree_truncated':False,'accessibility_error':None,'window_error':None}
  elif name in ['set_value','click','perform_action','scroll','type_text','press_key','activate_window']:
   assert args.get('window_id')==7 and args.get('pid')==123 and args.get('app_id')=='org.example.Editor'
   assert args.get('screenshot_after') is False
   if name not in ['type_text','press_key','activate_window']:assert args['snapshot_id']==snapshot
   data={'ok':True,'message':'SYNTHETIC_SECRET_RESPONSE_MUST_NOT_PASS_THROUGH'}
  else:raise RuntimeError('Unregistered tool reached fake MCP')
  result={'structuredContent':data,'content':[],'isError':False}
  if name=='get_app_state' and args.get('include_screenshot'):
   result['content']=[{'type':'image','mimeType':'image/png','data':state['png']}]
 response={'jsonrpc':'2.0','id':request['id'],'result':result}
 print(json.dumps(response),flush=True)
'''


def png():
    def chunk(tag, data):return struct.pack(">I", len(data))+tag+data+struct.pack(">I", zlib.crc32(tag+data)&0xffffffff)
    return b"\x89PNG\r\n\x1a\n"+chunk(b"IHDR",struct.pack(">IIBBBBB",2,2,8,6,0,0,0))+chunk(b"IDAT",zlib.compress((b"\0"+b"\xff\xdd\0\xff"*2)*2))+chunk(b"IEND",b"")


class LinuxContracts(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory(prefix="zaturo-linux-mcp-fixture-");self.root=Path(self.temp.name)
        binary=self.root/"computer-use-linux";binary.write_text(FAKE);binary.chmod(0o700)
        self.config_path=self.root/"computer.toml"
        self.config_path.write_text('enabled=true\nallow_input=true\nallow_capture=true\ntimeout_seconds=5\nbinary='+json.dumps(str(binary))+'\nbinary_sha256='+json.dumps(bridge.hashlib.sha256(binary.read_bytes()).hexdigest())+'\nstate_dir='+json.dumps(str(self.root/'journal'))+'\nallowed_apps=["org.example.Editor"]\n');self.config_path.chmod(0o600)
        self.state={'window':{'window_id':7,'pid':123,'app_id':'org.example.Editor','wm_class':'Editor','title':'Synthetic notes','focused':True,'hidden':False},'nodes':[{'index':3,'role':'entry','name':'Search','text':{'content':'Synthetic field'},'states':['editable','focused'],'supports_editable_text':True,'actions':['activate']}],'png':base64.b64encode(png()).decode()}
        self.save();self.config=bridge.Config(self.config_path)

    def tearDown(self):self.temp.cleanup()
    def save(self):(self.root/'state.json').write_text(json.dumps(self.state))
    def request(self,tool='get_app_state',arguments=None):
        now=int(time.time()*1000)
        return {'operation_id':str(uuid.uuid4()),'lease_token':'synthetic-bound-lease-token','lease_expires_at':now+20000,'deadline_at':now+20000,'owner_confirmed':True,'target':{'window_id':7,'pid':123,'app_id':'org.example.Editor'},'tool':tool,'arguments':arguments or {}}
    def approved(self,tool='set_value',arguments=None):
        observation=bridge.execute(self.config,self.request())
        request=self.request(tool,arguments or {'element_index':3,'value':'poco X7'})
        request['expected_snapshot_sha256']=observation['snapshot_sha256'];request['approved_plan_sha256']=bridge.plan_digest(request)
        return request
    def calls(self):return [json.loads(l) for l in (self.root/'calls.jsonl').read_text().splitlines()]
    def assert_code(self,code,request):
        with self.assertRaises(bridge.Rejected) as error:bridge.execute(self.config,request)
        self.assertEqual(error.exception.code,code)

    def test_actual_stdio_fresh_snapshot_exact_input_and_uncertain_replay_fence(self):
        os.environ['COMPUTER_USE_LINUX_ENABLE_SHELL']='1';os.environ['OPENCODE_GO_API_KEY']='synthetic-provider-secret'
        try:
            request=self.approved();result=bridge.execute(self.config,request)
            self.assertTrue(result['data']['dispatched']);self.assertFalse(result['data']['verified_external_effect'])
            self.assertNotIn('SYNTHETIC_SECRET_RESPONSE',json.dumps(result))
            self.assertTrue(all(c['shell'] is None and c['provider'] is None for c in self.calls()))
            self.assert_code('reconciliation_required',request)
            mutations=[c for c in self.calls() if (c.get('params') or {}).get('name')=='set_value'];self.assertEqual(len(mutations),1)
            intent=json.loads(next((self.root/'journal').glob('*.json')).read_text());self.assertEqual(set(intent),{'state','fingerprint'})
        finally:os.environ.pop('COMPUTER_USE_LINUX_ENABLE_SHELL',None);os.environ.pop('OPENCODE_GO_API_KEY',None)

    def test_stale_snapshot_window_rebinding_password_and_shell_are_refused(self):
        request=self.approved();self.state['nodes'][0]['name']='Different target';self.save();self.assert_code('stale_snapshot',request)
        self.state['window']['pid']=456;self.save();self.assert_code('target_changed',request)
        self.state['window']['pid']=123;self.state['nodes'][0]['role']='password text';self.save()
        request=self.approved();self.assert_code('protected_element',request)
        for tool in ['run_shell','setup_accessibility','setup_window_targeting','screenshot']:
            self.assert_code('tool_not_allowed',self.request(tool,{'command':'never execute'}))
        self.assertFalse(any((c.get('params') or {}).get('name')=='set_value' for c in self.calls()))

    def test_exact_plan_grant_emergency_stop_and_lease_are_enforced(self):
        request=self.approved();request['arguments']['value']='changed without approval';self.assert_code('exact_plan_required',request)
        request=self.approved();request['owner_confirmed']=False;self.assert_code('input_grant_required',request)
        (self.config.state/'STOP').write_text('Owner emergency stop');self.assert_code('emergency_stop',self.request())
        (self.config.state/'STOP').unlink();request=self.request();request['lease_expires_at']=0;self.assert_code('expired_lease',request)

    def test_private_capture_is_raw_png_only_and_not_an_ordinary_action(self):
        request=self.request('screenshot');request['approved_plan_sha256']=bridge.plan_digest(request)
        file=self.root/'capture.json';file.write_text(json.dumps(request));file.chmod(0o600)
        result=subprocess.run([sys.executable,str(Path(bridge.__file__)),'--config',str(self.config_path),'capture','--request-json',str(file)],capture_output=True,timeout=10)
        self.assertEqual(result.returncode,0,result.stderr);self.assertEqual(result.stdout,png());self.assertNotIn(b'base64',result.stdout)
        self.assertTrue(all(call['capture_backend']=='x11' for call in self.calls()))
        self.assertFalse(list(self.root.rglob('*.png')))
        request['operation_id']=str(uuid.uuid4());request['owner_confirmed']=False;file.write_text(json.dumps(request))
        result=subprocess.run([sys.executable,str(Path(bridge.__file__)),'--config',str(self.config_path),'capture','--request-json',str(file)],capture_output=True,timeout=10)
        self.assertNotEqual(result.returncode,0);self.assertEqual(result.stdout,b'');self.assertIn(b'capture_grant_required',result.stderr)

    def test_guardian_reaps_escaped_backend_descendant_after_outer_sigkill(self):
        self.state['hang']=True;self.save();request=self.request();file=self.root/'request.json';file.write_text(json.dumps(request));file.chmod(0o600)
        child=subprocess.Popen([sys.executable,str(Path(bridge.__file__)),'--config',str(self.config_path),'action','--request-json',str(file)],stdout=subprocess.PIPE,stderr=subprocess.PIPE,start_new_session=True)
        try:
            limit=time.monotonic()+4
            while not (self.root/'escaped.pid').exists() and time.monotonic()<limit:time.sleep(.02)
            self.assertTrue((self.root/'escaped.pid').exists());escaped=int((self.root/'escaped.pid').read_text())
            os.killpg(child.pid,signal.SIGKILL);child.communicate(timeout=3)
            limit=time.monotonic()+3
            while Path(f'/proc/{escaped}').exists() and time.monotonic()<limit:time.sleep(.03)
            self.assertFalse(Path(f'/proc/{escaped}').exists(),'escaped fake MCP child survived outer cancellation')
        finally:
            if child.poll() is None:os.killpg(child.pid,signal.SIGKILL);child.communicate(timeout=3)

    def test_transport_output_and_timeout_bounds_fail_without_echoing_server_data(self):
        self.state['oversize']=True;self.save();self.assert_code('response_too_large',self.request())
        self.state.pop('oversize');self.state['hang']=True;self.save();self.config.timeout=1
        self.assert_code('mcp_timeout',self.request())
        escaped=int((self.root/'escaped.pid').read_text());limit=time.monotonic()+2
        while Path(f'/proc/{escaped}').exists() and time.monotonic()<limit:time.sleep(.03)
        self.assertFalse(Path(f'/proc/{escaped}').exists())

    def test_semantic_pagination_keeps_snapshot_fence_and_bounds_output(self):
        self.state['nodes']=[{**self.state['nodes'][0],'index':i,'name':'Synthetic field '+str(i)} for i in range(40)];self.save()
        first=bridge.execute(self.config,self.request());self.assertEqual(first['data']['total_nodes'],40);self.assertEqual(first['data']['next_offset'],16)
        request=self.request('get_tree_page',{'offset':16});request['expected_snapshot_sha256']=first['snapshot_sha256'];second=bridge.execute(self.config,request)
        self.assertEqual(second['snapshot_sha256'],first['snapshot_sha256']);self.assertEqual(second['data']['nodes'][0]['index'],16)
        self.assertLessEqual(len(bridge.encoded(second['data'])),bridge.MAX_TEXT)

    def test_desktop_browser_semantics_cannot_bypass_dedicated_browser_policy(self):
        for app,code in [('org.mozilla.firefox','use_dedicated_browser_backend'),('com.openai.codex','protected_agent_or_runner'),('dev.zaturo.bot','protected_agent_or_runner'),('ai.opencode.desktop','protected_agent_or_runner'),('com.visualstudio.code','protected_agent_or_runner')]:
            self.config.apps=[app];self.state['window']['app_id']=app;self.state['window']['wm_class']=app;self.save()
            request=self.request();request['target']['app_id']=app
            self.assert_code(code,request)
            request['tool']='click';request['arguments']={'element_index':3};request['expected_snapshot_sha256']='0'*64;request['approved_plan_sha256']=bridge.plan_digest(request)
            self.assert_code(code,request)
            # Metadata discovery and owner-only encrypted-capture grants remain separate.
            self.assertEqual(bridge.doctor(self.config)['data']['windows'][0]['app_id'],app)

    def test_private_capture_route_cannot_inherit_a_file_writing_fallback(self):
        original=os.environ.get('COMPUTER_USE_LINUX_SCREENSHOT_BACKEND')
        os.environ['COMPUTER_USE_LINUX_SCREENSHOT_BACKEND']='gnome-screenshot'
        try:
            request=self.request('screenshot');request['approved_plan_sha256']=bridge.plan_digest(request)
            image=bridge.capture_png(self.config,request)
            self.assertEqual(bytes(image),png())
            self.assertTrue(all(call['capture_backend']=='x11' for call in self.calls()))
        finally:
            if original is None:os.environ.pop('COMPUTER_USE_LINUX_SCREENSHOT_BACKEND',None)
            else:os.environ['COMPUTER_USE_LINUX_SCREENSHOT_BACKEND']=original


if __name__ == '__main__':unittest.main()
