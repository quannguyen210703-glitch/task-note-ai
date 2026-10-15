"""Package only verified source and outputs, excluding private profiles/backups."""
from pathlib import Path
import shutil,hashlib,json,zipfile,datetime
root=Path(__file__).resolve().parent.parent
art=root/'artifacts'
release=root/'src-tauri/target/release'
installer=release/'bundle/nsis/Task Note AI_2.0.0_x64-setup.exe'
assert installer.is_file(),'V2 installer is missing; refusing to package a V1 file'
outputs=[installer,release/'tasknote.exe',release/'task-note-ai.exe']
for p in outputs:
    assert p.is_file() and p.stat().st_size>100000,p
    shutil.copy2(p,art/p.name)
source_files=[]
for dirname in ['src','src-tauri/src','src-tauri/task-core/src','src-tauri/task-core/tests','src-tauri/task-core/migrations','src-tauri/task-core/examples','src-tauri/icons','src-tauri/capabilities','docs','scripts']:
    for p in (root/dirname).rglob('*'):
        if p.is_file() and '__pycache__' not in p.parts and p.suffix not in ['.pyc','.sqlite','.log']:source_files.append(p)
for name in ['package.json','pnpm-lock.yaml','pnpm-workspace.yaml','index.html','tsconfig.json','vite.config.ts','eslint.config.js','.gitignore','src-tauri/Cargo.toml','src-tauri/Cargo.lock','src-tauri/build.rs','src-tauri/tauri.conf.json','src-tauri/task-core/Cargo.toml']:
    source_files.append(root/name)
source_files+=list(root.glob('*.md'))
source_files=sorted(set(source_files),key=lambda p:p.relative_to(root).as_posix())
source_hash=hashlib.sha256()
zip_path=art/'TaskNoteAI-2.0.0-source.zip'
with zipfile.ZipFile(zip_path,'w',zipfile.ZIP_DEFLATED) as z:
    for p in source_files:
        rel=p.relative_to(root).as_posix();data=p.read_bytes()
        source_hash.update(rel.encode()+b'\0'+data+b'\0');z.writestr(rel,data)
manifest={'version':'2.0.0','at_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'identifier':'vn.tasknoteai.desktop','installed_upgrade':False,'user_choice':'Keep installed V1 running; deliver installer only','source_sha256':source_hash.hexdigest(),'source_files':len(source_files),'artifacts':[]}
for p in [*(art/p.name for p in outputs),zip_path]:manifest['artifacts'].append({'file':p.name,'size_bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()})
(art/'MANIFEST-V2.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2),encoding='utf-8')
print(json.dumps({'version':'2.0.0','artifacts':manifest['artifacts'],'source_files':len(source_files)},ensure_ascii=False))
