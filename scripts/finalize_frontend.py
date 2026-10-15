"""Small exact replacements on established JSX expressions."""
from pathlib import Path
p=Path('src/upgrade.tsx')
s=p.read_text(encoding='utf-8')
s=s.replace('zone="Asia/Ho_Chi_Minh" onOpen={onOpen}', 'zone={zone} onOpen={onOpen}')
s=s.replace('useEffect(()=>setDraft(value),[value]);','useEffect(()=>{setDraft(value);setMessage(value.ai_enabled&&value.ai_consent?"Chưa kiểm tra kết nối":"Offline");},[value]);')
p.write_text(s,encoding='utf-8')
needle='</small>{t?'
replacement='</small><label>Thời lượng ước tính (phút)<input type="number" min={1} max={600} value={step.input.estimate_minutes??""} onChange={e=>setSteps(list=>list.map(s=>s.id===step.id?{...s,input:{...s.input,estimate_minutes:e.target.value?Number(e.target.value):null}}:s))}/></label><button disabled={busy} onClick={()=>void action(()=>invoke("set_step_estimate",{stepId:step.id,minutes:step.input.estimate_minutes}))}>Lưu thời lượng</button>{t?'
if needle in s:
    s=s.replace(needle,replacement)
    p.write_text(s,encoding='utf-8')
