"""Read-only compare every legacy field by primary key, excluding migration metadata."""
import sqlite3,pathlib,json,sys,hashlib
old,new,out=map(pathlib.Path,sys.argv[1:])
def connect(p):return sqlite3.connect(p.resolve().as_uri()+'?mode=ro',uri=True)
evidence={'source':str(old.resolve()),'upgraded_copy':str(new.resolve()),'tables':{},'ok':True}
with connect(old) as a,connect(new) as b:
    for (name,) in a.execute("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name<>'_sqlx_migrations' ORDER BY name"):
        columns=[r[1] for r in a.execute(f'PRAGMA table_info("{name}")')]
        quoted=','.join('"'+c+'"' for c in columns)
        rows_a=list(a.execute(f'SELECT {quoted} FROM "{name}" ORDER BY 1'))
        rows_b=list(b.execute(f'SELECT {quoted} FROM "{name}" ORDER BY 1'))
        equal=rows_a==rows_b
        evidence['tables'][name]={'legacy_columns':columns,'before_count':len(rows_a),'after_count':len(rows_b),'equal_all_values_by_id':equal,'sha256_before':hashlib.sha256(json.dumps(rows_a,ensure_ascii=False,default=str).encode()).hexdigest(),'sha256_after':hashlib.sha256(json.dumps(rows_b,ensure_ascii=False,default=str).encode()).hexdigest()}
        evidence['ok'] &= equal
    evidence['integrity']=b.execute('PRAGMA integrity_check').fetchone()[0]
    evidence['foreign_key_errors']=list(b.execute('PRAGMA foreign_key_check'))
    evidence['ok'] &= evidence['integrity']=='ok' and not evidence['foreign_key_errors']
    evidence['doing_ids_hash']=hashlib.sha256(json.dumps(list(a.execute("SELECT id FROM tasks WHERE status='doing' ORDER BY id"))).encode()).hexdigest()
out.write_text(json.dumps(evidence,indent=2,ensure_ascii=False),encoding='utf-8')
print(json.dumps({'ok':evidence['ok'],'tables':len(evidence['tables']),'output':str(out)}))
sys.exit(0 if evidence['ok'] else 1)
