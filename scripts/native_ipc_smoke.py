"""Launch only the isolated TEST build/profile; never stop the installed V1."""
import pathlib,subprocess,sys,tempfile,json,time,os,datetime
gui,cli=map(lambda v:pathlib.Path(v).resolve(),sys.argv[1:])
profile=pathlib.Path(tempfile.mkdtemp(prefix='test-profile-native-',dir=pathlib.Path('artifacts').resolve()))
p=subprocess.Popen([str(gui),'--test-profile',str(profile),'--test-headless','--test-exit-after=120'],creationflags=subprocess.CREATE_NO_WINDOW)
checks=[]
def run(args):
    result=subprocess.run([str(cli),'--data-dir',str(profile),'--json',*args],capture_output=True,encoding='utf-8',timeout=20)
    data=json.loads(result.stdout)
    assert result.returncode==0 and data['ok'],(args,result.returncode,data,result.stderr)
    checks.append({'args':args[:3],'ok':True})
    return data['data']
try:
    for _ in range(600):
        if (profile/'gui-session.json').is_file() and (profile/'engine-status.json').is_file():break
        if p.poll() is not None:raise RuntimeError(f'Test GUI exited during startup: {p.returncode}; '+((profile/'startup-error.txt').read_text(encoding='utf-8') if (profile/'startup-error.txt').exists() else 'No startup diagnostic'))
        time.sleep(.1)
    assert (profile/'gui-session.json').is_file(),'TEST core did not start'
    status=run(['status']);assert status['core_running'] is True
    task=run(['task','add','--title','Native IPC thử nghiệm','--date','today'])['tasks'][0]
    tid=task['id']
    run(['task','update',tid,'--status','doing']);run(['task','snooze',tid,'--for','30m']);run(['task','unsnooze',tid]);assert run(['task','show',tid])['status']=='doing'
    run(['notifications','pause','--for','60m']);run(['task','snooze',tid,'--for','15m']);run(['notifications','resume']);assert run(['task','show',tid])['snoozed_until_utc'] is not None
    run(['task','complete',tid]);run(['task','delete',tid]);run(['task','restore',tid]);run(['task','reopen',tid]);run(['ai','chat','Hôm nay thêm việc thử đồng bộ'])
    assert len(run(['task','list','--today']))==2
    recurring=run(['ai','chat','Nhắc anh họp kiểm thử mỗi thứ Hai lúc 9h'])['saved'][0]
    second=(datetime.date.fromisoformat(recurring['occurrence_date'])+datetime.timedelta(days=7)).isoformat()
    materialized=False
    for _ in range(18):
        future=run(['task','list','--date',second])
        matching=[t for t in future if t['template_id']==recurring['template_id']]
        if matching:
            assert len(matching)==1
            materialized=True
            break
        time.sleep(5)
    assert materialized,'Background Rust engine did not materialize the next occurrence while windows were hidden'
    session=json.loads((profile/'gui-session.json').read_text())
    assert session['pid']==p.pid
    report={'ok':True,'native_process':p.pid,'profile':str(profile),'gui':str(gui),'cli':str(cli),'checks':checks,'status':status,'background_recurrence_while_hidden':materialized,'notifications_disabled':True,'autostart_disabled':True,'limitations':['IPC/core verified against actual release process; visual UI/event consumption not observed','Native toast and physical sleep/wake not tested']}
    pathlib.Path('artifacts/v2-native-ipc-smoke.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf-8')
    print(json.dumps({'ok':True,'commands':len(checks),'pid':p.pid,'profile':str(profile)}))
finally:
    try:p.wait(timeout=50)
    except subprocess.TimeoutExpired:p.terminate();p.wait(timeout=10)
