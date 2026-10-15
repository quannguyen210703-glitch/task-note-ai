"""Exercise actual tasknote.exe on an isolated temporary profile only."""
import pathlib,subprocess,json,sqlite3,sys,tempfile,concurrent.futures,os,msvcrt
exe=pathlib.Path(sys.argv[1]).resolve()
profile=pathlib.Path(tempfile.mkdtemp(prefix='test-profile-cli-',dir=pathlib.Path('artifacts').resolve()))
checks=[]
def run(args,code=0):
    p=subprocess.run([str(exe),'--data-dir',str(profile),'--json',*args],capture_output=True,text=True,encoding='utf-8',timeout=35)
    assert p.returncode==code,(args,p.returncode,p.stdout,p.stderr)
    data=json.loads(p.stdout)
    assert data['schema_version']==1 and data['ok']==(code==0),(args,data)
    checks.append({'command':args[:3],'exit':p.returncode,'valid_json':True})
    return data.get('data')
def snapshot():
    with sqlite3.connect(profile/'task-note.sqlite') as c:
        return list(c.iterdump())
help_result=subprocess.run([str(exe),'--help'],capture_output=True,text=True,encoding='utf-8',timeout=10)
assert help_result.returncode==0 and 'Task Note AI' in help_result.stdout
run(['version'])
t=run(['task','add','--title','Kiểm tra "logo" — tiếng Việt','--date','today','--request-id','smoke-add'])['tasks'][0]
assert run(['task','add','--title','Kiểm tra "logo" — tiếng Việt','--date','today','--request-id','smoke-add'])['tasks'][0]['id']==t['id']
tid=t['id']
run(['task','show',tid]);run(['task','list','--today']);run(['task','list','--date','2026-10-15','--status','doing'])
run(['task','update',tid,'--status','doing','--title','Kiểm tra lại cẩm nang','--request-id','smoke-update'])
run(['task','update',tid,'--status','doing','--title','Kiểm tra lại cẩm nang','--request-id','smoke-update'])
before=snapshot();run(['task','update',tid,'--date','2026-10-15','--time','14:00','--dry-run']);assert snapshot()==before
before=snapshot();run(['doctor']);run(['status']);assert snapshot()==before
run(['task','snooze',tid,'--for','30m']);original=run(['task','show',tid]);assert original['status']=='doing'
run(['notifications','pause','--for','60m']);run(['task','unsnooze',tid]);after=run(['task','show',tid]);assert after['status']=='doing' and after['due_at_utc']==original['due_at_utc'] and after['snoozed_until_utc'] is None
assert run(['status'])['notification_pause'] is True
run(['task','unsnooze',tid]);run(['notifications','resume'])
run(['task','complete',tid]);run(['task','snooze',tid,'--for','30m'],2);run(['task','delete',tid]);assert len(run(['task','trash','list']))==1
run(['task','unsnooze',tid]);run(['task','restore',tid]);run(['task','reopen',tid])
run(['task','snooze',tid,'--until','2099-10-15T09:00:00+07:00']);run(['task','unsnooze',tid])
run(['task','show','ffffffff-ffff-ffff-ffff-ffffffffffff'],3)
run(['task','add','--title','Ngày sai','--date','2026-02-30'],2)
run(['task','add','--title','Giờ sai','--time','25:00'],2)
run(['task','list','--status','paused'],2)
run(['ai','chat','Hôm nay thêm việc kiểm tra logo'])
batch=run(['ai','chat','Hôm nay thêm việc thử đồng bộ, thu tiền, gửi email, gọi chị Mai, soạn checklist, đối chiếu dữ liệu'])
assert len(batch['saved'])==6 and batch['batch_token'] is None
assert len({t['due_at_utc'] for t in batch['saved']})==1
assert any(t['title']=='gọi chị Mai' for t in batch['saved'])
assert next(t for t in batch['saved'] if t['title']=='đối chiếu dữ liệu')['description']==''
assert len(run(['ai','chat','Thêm việc soạn checklist riêng'])['saved'])==1
assert run(['ai','chat','Thêm việc đối chiếu dữ liệu riêng'])['saved'][0]['description']==''
run(['ai','plan','--date','today','--dry-run'])
run(['ai','chat','Chia mục tiêu hoàn thiện tài liệu thành 3 bước'],7)
backup=run(['backup','create','--output',str(profile/'backup with spaces')]);run(['backup','verify',backup['path']])
with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
    results=list(pool.map(lambda i:run(['task','add','--title',f'Đồng thời {i}','--date','tomorrow','--request-id',f'concurrent-{i}']),range(2)))
assert len({r['tasks'][0]['id'] for r in results})==2
(profile/'engine-status.json').write_text('{"last_check":1,"running":true}',encoding='utf-8')
run(['status'],6)
(profile/'engine-status.json').write_text('{',encoding='utf-8')
run(['status'],6)
(profile/'engine-status.json').write_text('{"version":"2.0.0","last_check":1}',encoding='utf-8')
run(['status'])
with open(profile/'core.lock','r+b') as owner:
    owner.seek(0);owner.write(b'0');owner.flush();owner.seek(0)
    msvcrt.locking(owner.fileno(),msvcrt.LK_NBLCK,1)
    (profile/'gui-session.json').write_text('{"fixture":"unresponsive IPC owner"}',encoding='utf-8')
    try:run(['status'],4)
    finally:
        owner.seek(0);msvcrt.locking(owner.fileno(),msvcrt.LK_UNLCK,1)
assert not list((profile/'cli-ipc').glob('*.request'))
report={'ok':True,'profile':str(profile),'executable':str(exe),'checks':checks,'count':len(checks)}
pathlib.Path('artifacts/v2-cli-smoke.json').write_text(json.dumps(report,indent=2,ensure_ascii=False),encoding='utf-8')
print(json.dumps({'ok':True,'commands':len(checks),'profile':str(profile)}))
