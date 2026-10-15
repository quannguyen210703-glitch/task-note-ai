"""Snapshot SQLite through its online backup API. Never copy a live WAL database."""
import argparse, sqlite3, pathlib, hashlib, json, datetime

def digest_rows(db):
    result = {}
    for (name,) in db.execute("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name"):
        columns = [r[1] for r in db.execute(f'PRAGMA table_info("{name}")')]
        rows = list(db.execute(f'SELECT * FROM "{name}" ORDER BY 1'))
        result[name] = {'count': len(rows), 'columns': columns, 'sha256': hashlib.sha256(json.dumps(rows, ensure_ascii=False, default=str).encode()).hexdigest()}
    return result

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('source')
    parser.add_argument('destination')
    args = parser.parse_args()
    source, dest = pathlib.Path(args.source).resolve(), pathlib.Path(args.destination).resolve()
    if dest.exists():
        raise SystemExit('Destination exists; refusing overwrite')
    dest.parent.mkdir(parents=True, exist_ok=True)
    with sqlite3.connect(source.as_uri() + '?mode=ro', uri=True, timeout=10) as src, sqlite3.connect(dest) as out:
        src.backup(out)
        integrity = out.execute('PRAGMA integrity_check').fetchone()[0]
        foreign_keys = list(out.execute('PRAGMA foreign_key_check'))
        assert integrity == 'ok' and not foreign_keys
        evidence = {'at_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(), 'source': str(source), 'backup': str(dest), 'sqlite_runtime': sqlite3.sqlite_version, 'integrity': integrity, 'foreign_keys': foreign_keys, 'tables': digest_rows(out)}
    evidence['file_sha256'] = hashlib.sha256(dest.read_bytes()).hexdigest()
    dest.with_suffix('.manifest.json').write_text(json.dumps(evidence, indent=2), encoding='utf-8')
    print(json.dumps({'backup':str(dest), 'integrity': integrity, 'counts':{k:v['count'] for k,v in evidence['tables'].items()}}, ensure_ascii=False))
