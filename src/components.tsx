import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Check, Clock, Trash2, X } from "lucide-react";
import type { Task, TaskInput, Reply } from "./types";
import {
  blank,
  effective,
  formatTime,
  isOverdue,
  localInput,
  parseLocal,
  priorityLabels,
  statusLabels,
  recurrenceLabel,
} from "./utils";

export function TaskRow({
  task,
  now,
  zone,
  onToggle,
  onOpen,
  onRestore,
  onDelete,onSnooze,onUnsnooze,
}: {
  task: Task;
  now: number;
  zone: string;
  onToggle: (t: Task) => void;
  onOpen: (t: Task) => void;
  onRestore?: (t: Task) => void;
  onDelete?: (t:Task)=>Promise<void>;
  onSnooze?: (t:Task)=>Promise<void>;
  onUnsnooze?: (t:Task)=>Promise<void>;
}) {
  const [busy,setBusy]=useState(false);
  const [error,setError]=useState("");
  const snoozed=task.snoozed_until_utc !== null && task.snoozed_until_utc > now && task.status !== "done" && task.archived_at_utc === null;
  async function action(fn:()=>Promise<void>){setBusy(true);setError("");try{await fn();}catch(e){setError(String(e));}finally{setBusy(false);}}
  const late = isOverdue(task, now, zone);
  const due = effective(task);
  const outside =
    due !== null &&
    !task.is_all_day &&
    (() => {
      const clock = localInput(due, zone).split("T")[1];
      return clock < "08:00" || clock > "18:00";
    })();
  return (
    <article className={`task-row ${task.status === "done" ? "done" : ""}`}>
      {task.archived_at_utc === null ? (
        <button
          className={`check ${task.status === "done" ? "checked" : ""}`}
          aria-label={
            task.status === "done"
              ? `Đánh dấu chưa xong ${task.title}`
              : `Hoàn thành ${task.title}`
          }
          onClick={() => onToggle(task)}
        >
          {task.status === "done" && <Check size={14} />}
        </button>
      ) : (
        <Trash2 size={18} />
      )}
      <button className="task-content" onClick={() => onOpen(task)}>
        <strong>{task.title}</strong>
        <span>
          {task.is_all_day&&task.local_date?task.local_date:formatTime(due, zone, task.is_all_day)}{" "}
          {snoozed && ` · Hoãn đến ${formatTime(task.snoozed_until_utc,zone)}`}
        </span>
        <div className="badges">
          {late && <b className="badge late">Quá hạn</b>}
          <b className={`badge ${task.priority}`}>
            {priorityLabels[task.priority]}
          </b>
          {task.status === "doing" && <b className="badge">Đang làm</b>}
          {task.blocked&&<b className="badge">Chờ bước trước</b>}
          {task.source&&task.source!=="manual"&&<b className="badge">{task.source==="chat"?"Từ chat":"AI theo mục tiêu"}</b>}
          {task.estimate_minutes&&<b className="badge">{task.estimate_minutes}′ · AI ước tính</b>}
          {task.recurrence_rule && (
            <b className="badge">{recurrenceLabel(task.recurrence_rule)}</b>
          )}
          {task.tag_or_project && (
            <b className="badge">{task.tag_or_project}</b>
          )}
          {outside && <b className="badge">Hạn ngoài giờ nhắc</b>}
        </div>
      </button>
      {onRestore && (
        <button className="small" onClick={() => onRestore(task)}>
          Khôi phục
        </button>
      )}
      {task.archived_at_utc===null && <div className="row-actions">
        {snoozed&&onUnsnooze ? <button disabled={busy} onClick={()=>void action(()=>onUnsnooze(task))}>Bỏ tạm hoãn</button> : task.status!=="done"&&onSnooze&&<button disabled={busy} onClick={()=>void action(()=>onSnooze(task))} title="Tạm hoãn nhắc 30 phút; giữ hạn gốc">Hoãn 30′</button>}
        {onDelete&&<button disabled={busy} onClick={()=>void action(()=>onDelete(task))} aria-label={`Xóa việc ${task.title}`}><Trash2 size={14}/></button>}
      </div>}
      {error&&<small role="alert">{error}</small>}
    </article>
  );
}
export function TaskEditor({
  task,
  zone,
  onSave,
  onClose,
  onDelete,
  onSnooze,
  onUnsnooze,
}: {
  task: Task | null;
  zone: string;
  onSave: (input: TaskInput, scope: string) => Promise<void>;
  onClose: () => void;
  onDelete?: (scope: string) => Promise<void>;
  onSnooze?: (until: number) => Promise<void>;
  onUnsnooze?:()=>Promise<void>;
}) {
  const [input, setInput] = useState<TaskInput>(
    task
      ? {
          title: task.title,
          description: task.description,
          due_at_utc: task.due_at_utc,
          is_all_day: task.is_all_day,
          priority: task.priority,
          status: task.status,
          tag_or_project: task.tag_or_project,
          recurrence_rule: task.recurrence_rule,
        }
      : { ...blank },
  );
  const [date, setDate] = useState(task?.is_all_day&&task.local_date?`${task.local_date}T00:00`:localInput(task?.due_at_utc ?? null, zone));
  const [scope, setScope] = useState("one");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [snoozeDate, setSnoozeDate] = useState("");
  const [draftReady,setDraftReady]=useState(false);
  const [origin,setOrigin]=useState<{source:string;original_request:string}|null>(null);
  useEffect(()=>{if(task)void invoke<{source:string;original_request:string}|null>("task_origin",{id:task.id}).then(setOrigin).catch(()=>{});},[task]);
  const draftKey=`editor:${task?.id??"new"}`;
  useEffect(()=>{let alive=true;void invoke<string|null>("load_draft",{key:draftKey}).then(raw=>{if(alive&&raw){const d=JSON.parse(raw) as {input:TaskInput;date:string}|null;if(d){setInput(d.input);setDate(d.date);}}}).catch(()=>{}).finally(()=>{if(alive)setDraftReady(true);});return()=>{alive=false;};},[draftKey]);
  useEffect(()=>{if(draftReady)void invoke("save_draft",{key:draftKey,json:JSON.stringify({input,date})}).catch(()=>{});},[draftKey,draftReady,input,date]);
  const field = <K extends keyof TaskInput>(key: K, value: TaskInput[K]) =>
    setInput({ ...input, [key]: value });
  async function action(fn: () => Promise<void>) {
    setBusy(true);
    setError("");
    try {
      await fn();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  const archived = task?.archived_at_utc !== null && task !== null;
  return (
    <div className="modal-backdrop">
      <section
        className="modal"
        role="dialog"
        aria-modal="true"
        aria-label={task ? "Chi tiết công việc" : "Thêm công việc"}
      >
        <header>
          <h2>{task ? "Chi tiết công việc" : "Thêm công việc"}</h2>
          <button aria-label="Đóng chi tiết" onClick={onClose} disabled={busy}>
            <X size={20} />
          </button>
        </header>
        {task&&<small className="task-id">ID: {task.id}</small>}
        {origin&&<details><summary>Nguồn tạo: {origin.source}</summary><p>{origin.original_request||"Thao tác trực tiếp"}</p></details>}
        <form
          onSubmit={(e) => {
            e.preventDefault();
            void action(async () => {await onSave({ ...input, due_at_utc: parseLocal(date, zone) }, scope);await invoke("save_draft",{key:draftKey,json:"null"});});
          }}
        >
          <label>
            Tiêu đề
            <input
              autoFocus
              required
              maxLength={300}
              value={input.title}
              onChange={(e) => field("title", e.target.value)}
              disabled={archived}
            />
          </label>
          <label>
            Ghi chú
            <textarea
              maxLength={10000}
              value={input.description}
              onChange={(e) => field("description", e.target.value)}
              disabled={archived}
            />
          </label>
          <label>
            Ngày và giờ · {zone}
            <input
              type="datetime-local"
              value={date}
              onChange={(e) => setDate(e.target.value)}
              disabled={archived}
            />
          </label>
          <label className="toggle">
            <input
              type="checkbox"
              checked={input.is_all_day}
              onChange={(e) => field("is_all_day", e.target.checked)}
              disabled={archived}
            />
            Công việc cả ngày (không nhắc theo hạn)
          </label>
          <div className="columns">
            <label>
              Ưu tiên
              <select
                value={input.priority}
                onChange={(e) =>
                  field("priority", e.target.value as TaskInput["priority"])
                }
                disabled={archived}
              >
                {Object.entries(priorityLabels).map(([v, t]) => (
                  <option key={v} value={v}>
                    {t}
                  </option>
                ))}
              </select>
            </label>
            <label>
              Trạng thái
              <select
                value={input.status}
                onChange={(e) =>
                  field("status", e.target.value as TaskInput["status"])
                }
                disabled={archived}
              >
                {Object.entries(statusLabels).map(([v, t]) => (
                  <option key={v} value={v}>
                    {t}
                  </option>
                ))}
              </select>
            </label>
          </div>
          <label>
            Tag / dự án
            <input
              maxLength={100}
              value={input.tag_or_project}
              onChange={(e) => field("tag_or_project", e.target.value)}
              disabled={archived}
            />
          </label>
          {task?.template_id && (
            <label>
              Phạm vi chỉnh sửa
              <select value={scope} onChange={(e) => setScope(e.target.value)}>
                <option value="one">Chỉ lần này</option>
                <option value="series">
                  Lần này và các kỳ chưa xong sau đó
                </option>
              </select>
            </label>
          )}
          <label>
            Lịch lặp
            <select
              value={input.recurrence_rule ?? ""}
              disabled={
                archived ||
                (task !== null && (!task.template_id || scope === "one"))
              }
              onChange={(e) => field("recurrence_rule", e.target.value || null)}
            >
              <option value="">Không lặp</option>
              <option value="daily">Hằng ngày</option>
              <option value="weekdays">Ngày làm việc (T2–T6)</option>
              {[
                "Thứ Hai",
                "Thứ Ba",
                "Thứ Tư",
                "Thứ Năm",
                "Thứ Sáu",
                "Thứ Bảy",
                "Chủ nhật",
              ].map((d, i) => (
                <option key={d} value={`weekly:${i}`}>
                  Mỗi {d}
                </option>
              ))}
            </select>
          </label>
          {input.recurrence_rule && (
            <p className="hint">
              Ngày bắt đầu phải đúng ngày của lịch lặp. Mỗi kỳ có trạng thái
              hoàn thành riêng.
            </p>
          )}
          {error && (
            <p role="alert" className="error">
              {error}
            </p>
          )}
          {!archived && (
            <footer>
              <button type="button" onClick={onClose} disabled={busy}>
                Hủy
              </button>
              <button className="primary" disabled={busy}>
                {busy ? "Đang lưu…" : "Xác nhận lưu"}
              </button>
            </footer>
          )}
        </form>
        {task && !archived && onSnooze && task.status !== "done" && (
          <div className="snooze">
            <h3>
              <Clock size={15} /> Hoãn nhắc
            </h3>
            {task.snoozed_until_utc!==null&&task.snoozed_until_utc>Date.now()/1000&&onUnsnooze&&<button disabled={busy} onClick={()=>void action(onUnsnooze)}>Bỏ tạm hoãn</button>}
            <div className="actions">
              {[15, 30, 60].map((m) => (
                <button
                  disabled={busy}
                  key={m}
                  onClick={() =>
                    void action(() =>
                      onSnooze(Math.floor(Date.now() / 1000) + m * 60),
                    )
                  }
                >
                  {m} phút
                </button>
              ))}
            </div>
            <label>
              Hoãn tới
              <input
                type="datetime-local"
                value={snoozeDate}
                onChange={(e) => setSnoozeDate(e.target.value)}
              />
            </label>
            <button
              disabled={busy || !snoozeDate}
              onClick={() =>
                void action(async () => {
                  const until = parseLocal(snoozeDate, zone);
                  if (until !== null) await onSnooze(until);
                })
              }
            >
              Xác nhận hoãn
            </button>
          </div>
        )}
        {task && !archived && onDelete && (
          <div className="delete-section">
              <button
                className="danger"
                disabled={busy}
                onClick={() => void action(() => onDelete(scope))}
              >
                <Trash2 size={15} /> Xóa việc {scope==="series"?"lần này và các lần sau":"vào Thùng rác"}
              </button>
          </div>
        )}
      </section>
    </div>
  );
}
export function ProposalView({
  reply,
  zone,
  onConfirm,
  onOpen,
}: {
  reply: Reply;
  zone: string;
  onConfirm: (token: string) => Promise<void>;
  onOpen?:(t:Task)=>void;
}) {
  const [resolved, setResolved] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  return (
    <div className="assistant-message">
      <p>{reply.text}</p>
      {reply.saved?.map(task=><div className="proposal" key={task.id}><b>{task.title}</b><span>{formatTime(task.due_at_utc,zone,task.is_all_day)}</span>{!reply.batch_token&&onOpen&&<button onClick={()=>onOpen(task)}>Xem / Chỉnh sửa</button>}</div>)}
      {!resolved&&reply.batch_token&&<button disabled={busy} onClick={async()=>{setBusy(true);try{await invoke("confirm_batch",{token:reply.batch_token});setResolved(true);}catch(e){setError(String(e));}finally{setBusy(false);}}}>Thêm toàn bộ vào lịch</button>}
      {!resolved&&!reply.batch_token&&!!reply.saved?.length&&reply.request_id&&<button disabled={busy} onClick={async()=>{setBusy(true);try{await invoke("undo_batch",{requestId:reply.request_id});setResolved(true);}catch(e){setError(String(e));}finally{setBusy(false);}}}>Hoàn tác {reply.saved.length} việc</button>}
      {!resolved &&
        reply.proposals.map((card) => (
          <div className="proposal" key={card.token}>
            <b>
              {card.label}: {card.input.title}
            </b>
            <span>
              {formatTime(card.input.due_at_utc, zone, card.input.is_all_day)}
            </span>
            <span>
              {recurrenceLabel(card.input.recurrence_rule)} ·{" "}
              {priorityLabels[card.input.priority]}
            </span>
            <div className="actions">
              <button
                className="primary"
                disabled={busy}
                onClick={async () => {
                  setBusy(true);
                  setError("");
                  try {
                    await onConfirm(card.token);
                    setResolved(true);
                  } catch (e) {
                    setError(String(e));
                  } finally {
                    setBusy(false);
                  }
                }}
              >
                Xác nhận
              </button>
              <button disabled={busy} onClick={() => setResolved(true)}>
                Hủy
              </button>
            </div>
          </div>
        ))}
      {resolved && <small>Đề xuất đã xử lý / hủy.</small>}
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
    </div>
  );
}
