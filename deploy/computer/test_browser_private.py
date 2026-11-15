"""Offline private-frame protocol, native grant, profile and URL tests. No real screen/account."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
import uuid
sys.path.insert(0,str(Path(__file__).parent))
import browser_mcp
import browser_private as private
import linux_mcp as base

FAKE_NODE=r'''#!/usr/bin/python3
import struct,sys,zlib
assert sys.argv[1].endswith('browser_private.mjs')
request=__import__('json').loads(sys.stdin.readline())
assert request['url']=='https://example.test/private?fixture=one'
def chunk(kind,data):return struct.pack('!I',len(data))+kind+data+struct.pack('!I',zlib.crc32(kind+data)&0xffffffff)
png=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('!IIBBBBB',1280,720,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(b'\x00'+bytes(1280*720*4)))+chunk(b'IEND',b'')
sys.stdout.buffer.write(struct.pack('!I',len(png))+png);sys.stdout.buffer.flush()
sys.stdin.read()
'''
class PrivateBrowserContracts(unittest.TestCase):
    def test_offline_node_url_and_renderer_contracts(self):
        node=shutil.which('node')
        if not node:self.skipTest('Node24 is required by CI; unavailable locally')
        result=subprocess.run([node,'--test',str(Path(__file__).with_suffix('.mjs'))],capture_output=True,timeout=15)
        self.assertEqual(result.returncode,0,result.stdout.decode()+result.stderr.decode())
    def test_guarded_binary_protocol_has_no_private_url_in_argv_or_journal(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);profile=root/'private-profile';profile.mkdir(mode=0o700)
            def write(path,data,mode=0o600):path.write_text(data);path.chmod(mode);return path
            marker=write(profile/'.zaturobot-private-browser',json.dumps({'schema':1,'purpose':'zaturobot-private-page-capture'}))
            node=write(root/'node',FAKE_NODE,0o700);chrome=write(root/'chrome','# Synthetic pinned browser artifact',0o700)
            packages=root/'node_modules';packages.mkdir(mode=0o700)
            for name,version in browser_mcp.PACKAGES.items():
                package=packages/name;package.mkdir(parents=True,mode=0o700);
                for parent in package.parents:
                    if parent == root:break
                    parent.chmod(0o700)
                write(package/'package.json',json.dumps({'name':name,'version':version}))
            cli=write(packages/'@playwright/mcp/cli.js','// Synthetic pinned artifact')
            write(packages/'playwright-core/index.mjs','// Synthetic reviewed package')
            config_values={'enabled':True,'allow_capture':True,'allow_input':False,'timeout_seconds':5,'state_dir':str(root/'journal'),'node':str(node),'node_sha256':hashlib.sha256(node.read_bytes()).hexdigest(),'playwright_cli':str(cli),'playwright_cli_sha256':hashlib.sha256(cli.read_bytes()).hexdigest(),'playwright_tree_sha256':browser_mcp.package_tree_sha256(cli),'browser_executable':str(chrome),'browser_sha256':hashlib.sha256(chrome.read_bytes()).hexdigest(),'profile_dir':str(profile)}
            path=write(root/'private.toml','\n'.join(k+'='+json.dumps(v) for k,v in config_values.items()))
            config=base.Config(path,backend='private_browser');now=int(time.time()*1000)
            request={'operation_id':str(uuid.uuid4()),'lease_token':'synthetic-native-private-lease','lease_expires_at':now+10000,'deadline_at':now+10000,'owner_confirmed':True,'tool':'private_browser_capture','arguments':{'url':'https://example.test/private?fixture=one'},'target':{}}
            image=private.capture(config,request);self.assertTrue(image.startswith(b'\x89PNG'));image[:]=bytes(len(image))
            for entry in config.state.glob('*.json'):self.assertNotIn('example.test',entry.read_text())
            with self.assertRaises(base.Rejected):private.capture(config,request)
            request['owner_confirmed']=False
            with self.assertRaises(base.Rejected):private.capture(config,request)
            marker.write_text('{}')
            with self.assertRaises(base.Rejected):private.private_profile(config)
