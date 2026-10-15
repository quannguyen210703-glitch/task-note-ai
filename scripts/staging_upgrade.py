"""Migration/retry/lock evidence using copies of a verified V1 snapshot only."""
import pathlib,tempfile,shutil,sqlite3,subprocess,json,sys
exe=pathlib.Path(sys.argv[1]).resolve();source=pathlib.Path(sys.argv[2]).resolve()
base=pathlib.Path(tempfile.mkdtemp(prefix='staging-migration-',dir=pathlib.Path('artifacts').resolve()))
def run(root,args,code=0):
    p=subprocess.run([str(exe),'--data-dir',str(root),'--json',*args],capture_output=True,encoding='utf-8',timeout=35)
    data=json.loads(p.stdout)
    assert p.returncode==code,(args,p.returncode,data,p.stderr)
    return data
def copy_profile(name):
    root=base/name;root.mkdir();shutil.copy2(source,root/'task-note.sqlite');return root
normal=copy_profile('normal')
run(normal,['backup','create','--output',str(normal/'backups')])
p=subprocess.run([sys.executable,'scripts/compare_upgrade.py',str(source),str(normal/'task-note.sqlite'),'artifacts/v2-migration-comparison.json'],capture_output=True,encoding='utf-8',timeout=20)
assert p.returncode==0,p.stdout+p.stderr
status=run(normal,['doctor'])['data'];assert status['sqlite_version']=='3.51.3' and status['schema_version']==2
failure=copy_profile('failure-retry')
with sqlite3.connect(failure/'task-note.sqlite') as c:c.execute('CREATE TABLE goals(id TEXT PRIMARY KEY)')
error=run(failure,['backup','create','--output',str(failure/'backup-fail')],4)
with sqlite3.connect(failure/'task-note.sqlite') as c:
    names={r[0] for r in c.execute("SELECT name FROM sqlite_master WHERE type='table'")}
    assert 'mutation_requests' not in names and 'task_events' not in names,'Partial migration was not rolled back'
    assert c.execute('PRAGMA integrity_check').fetchone()[0]=='ok'
    c.execute('DROP TABLE goals') # Fixture collision only, not a real data table.
run(failure,['backup','create','--output',str(failure/'backup-retry')])
comparison=base/'failure-retry-comparison.json'
p=subprocess.run([sys.executable,'scripts/compare_upgrade.py',str(source),str(failure/'task-note.sqlite'),str(comparison)],capture_output=True,encoding='utf-8',timeout=20)
assert p.returncode==0,p.stdout+p.stderr
with sqlite3.connect(normal/'task-note.sqlite') as c:
    c.execute('BEGIN IMMEDIATE')
    lock_error=run(normal,['task','add','--title','Lock fixture only','--request-id','lock-retry'],4)
    c.rollback()
run(normal,['task','add','--title','Lock fixture only','--request-id','lock-retry'])
run(normal,['task','add','--title','Lock fixture only','--request-id','lock-retry'])
with sqlite3.connect(normal/'task-note.sqlite') as c:
    assert c.execute("SELECT count(*) FROM tasks WHERE title='Lock fixture only'").fetchone()[0]==1
report={'ok':True,'source_snapshot':str(source),'profiles':str(base),'runtime_sqlite':status['sqlite_version'],'schema':status['schema_version'],'migration_all_legacy_fields_equal':True,'mid_migration_failure_exit':error['error']['code'],'transaction_rollback':True,'retry_preserves_rows':True,'db_lock_exit':lock_error['error']['code'],'bounded_retry_once':True,'limits':['Actual disk exhaustion and admin-profile change were not induced']}
pathlib.Path('artifacts/v2-staging-tests.json').write_text(json.dumps(report,indent=2,ensure_ascii=False),encoding='utf-8')
print(json.dumps({'ok':True,'sqlite':status['sqlite_version'],'schema':status['schema_version'],'migration_equal':True,'rollback_and_retry':True,'db_lock':True}))
