import sys,os,json,time
from pathlib import Path
root=Path(__file__).resolve().parents[2]
assert Path.cwd()==root and os.environ['PYTHONPATH']==str(root)
assert not any(any(word in key.upper() for word in ['TOKEN','PASSWORD','SECRET','API_KEY']) for key in os.environ)
args=sys.argv[1:]
with (root/'calls.jsonl').open('a') as stream:stream.write(json.dumps(args)+'\n')
mode=(root/'mode').read_text() if (root/'mode').exists() else 'normal'
if mode=='slow':time.sleep(30)
if mode=='oversize':print('x'*70000);sys.exit(0)
if mode=='rejected':print('SYNTHETIC_PRIVATE_ERROR',file=sys.stderr);print(json.dumps({'schema_version':1,'error':'operation_rejected'}));sys.exit(1)
def arg(name):return args[args.index(name)+1]
def run(index):return {'id':'run-'+str(index),'title':'á'*150,'topic':'t'*300,'state':'scheduled','slot':'2026-11-13T19:00:00-04:00','video_id':'video_'+str(index),'artifacts':['video.mp4','thumbnail.png'],'unknown_secret':'SYNTHETIC_EXTRA_MUST_NOT_LEAK'}
if args==['profiles']:
 output={'schema_version':1,'profiles':[{'id':'channel','name':'Synthetic channel','channel_id':'UC_SYNTHETIC','account_id':'account','browser_configured':False,'unknown_secret':'SYNTHETIC_EXTRA_MUST_NOT_LEAK'}],'unknown_secret':'SYNTHETIC_EXTRA_MUST_NOT_LEAK'}
elif args[0]=='runs':output={'schema_version':1,'channel':arg('--channel'),'runs':[run(n) for n in range(int(arg('--limit')))]}
elif args[0]=='submit':
 assert '--approved-schedule' in args
 identifier=arg('--operation-id');count=int(arg('--count'))
 output={'schema_version':1,'operation_id':identifier,'state':'accepted','channel':arg('--channel'),'account':arg('--account'),'count':count,'created_at':1,'updated_at':2,'summary':'raw summary is deliberately not forwarded','result':{'scheduled_count':0,'failed_count':0,'runs':[]}}
 (root/'job.json').write_text(json.dumps(output))
 if mode=='lost-response':sys.exit(1)
elif args[0]=='status':
 output=json.loads((root/'job.json').read_text())
 if mode in ['completed','failed','uncertain']:
  output['state']=mode
  output['result']={'scheduled_count':output['count'] if mode=='completed' else 0,'failed_count':output['count'] if mode=='failed' else 0,'runs':[run(n) for n in range(output['count'])] if mode=='completed' else []}
 if mode=='wrong-operation':output['operation_id']='00000000-0000-0000-0000-000000000001'
 if mode=='dishonest-completion':output['state']='completed'
else:raise RuntimeError('unexpected fixed verb')
print(json.dumps(output,ensure_ascii=False))
