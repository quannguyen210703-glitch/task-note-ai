import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  ArrowUpRight,
  Bell,
  CheckCheck,
  ChevronDown,
  ChevronUp,
  ClipboardList,
  LayoutDashboard,
  MessageSquare,
  Minus,
  Pin,
  Plus,
  Search,
  Settings as SettingsIcon,
  Trash2,
} from "lucide-react";
import type {
  Task,
  TaskInput,
  Settings,
  EngineStatus,
  Page,
  Reply,
} from "./types";
import { TaskEditor, TaskRow, ProposalView } from "./components";
import { GoalsView, AISettings, QuickChat } from "./upgrade";
import {
  filterTasks,
  todayTask,
  isOverdue,
  effective,
  formatTime,
  zoneName,
} from "./utils";

const pages: { id: Page; label: string; icon: typeof Bell }[] = [
  { id: "today", label: "Hôm nay", icon: ClipboardList },
  { id: "all", label: "Tất cả công việc", icon: LayoutDashboard },
  { id: "upcoming", label: "7 ngày tới", icon: Bell },
  { id: "completed", label: "Đã hoàn thành", icon: CheckCheck },
  { id: "assistant", label: "Trợ lý Task", icon: MessageSquare },
  { id: "goals", label: "Mục tiêu", icon: ClipboardList },
  { id: "snoozed", label: "Đang tạm hoãn", icon: Bell },
  { id: "settings", label: "Cài đặt", icon: SettingsIcon },
  { id: "trash", label: "Thùng rác", icon: Trash2 },
];
export default function App() {
  const sticky =
    new URLSearchParams(location.search).get("window") === "sticky";
  const [tasks, setTasks] = useState<Task[]>([]);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [engine, setEngine] = useState<EngineStatus | null>(null);
  const [page, setPage] = useState<Page>("today");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(true);
  const [editing, setEditing] = useState<Task | null | undefined>(undefined);
  const [collapsed, setCollapsed] = useState(false);
  const [now, setNow] = useState(Math.floor(Date.now() / 1000));
  const [search, setSearch] = useState("");
  const [priority, setPriority] = useState("");
  const [status, setStatus] = useState("");
  const [tag, setTag] = useState("");
  const [filterDate, setFilterDate] = useState("");
  const [deleted,setDeleted]=useState<string|null>(null);
  const generation = useRef(0);
  const reload = useCallback(async () => {
    const request = ++generation.current;
    try {
      const [t, s, e] = await Promise.all([
        invoke<Task[]>("list_tasks"),
        invoke<Settings>("get_settings"),
        invoke<EngineStatus>("engine_status"),
      ]);
      if (request === generation.current) {
        setTasks(t);
        setSettings((previous) =>
          JSON.stringify(previous) === JSON.stringify(s) ? previous : s,
        );
        setEngine(e);
        setError("");
      }
    } catch (e) {
      if (request === generation.current) setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);
  useEffect(() => {
    let alive = true;
    const cleanups: (() => void)[] = [];
    void reload();
    if (new URLSearchParams(location.search).get('window') !== 'sticky') {
      void invoke<string|null>('initial_tab').then(tab=>{
        if(!alive)return;
        if(tab==='add'){setEditing(null);setPage('today');}
        else if(tab && pages.some(p=>p.id===tab))setPage(tab as Page);
      }).catch(e=>setError(String(e)));
    }
    for (const event of ["tasks-changed", "settings-changed", "engine-changed"])
      void listen(event, () => {
        if (alive) void reload();
      }).then((fn) => {
        if (alive) cleanups.push(fn);
        else fn();
      });
    void listen<string>("navigate", (event) => {
      if (event.payload === "add") {
        setEditing(null);
        setPage("today");
      } else if (pages.some((p) => p.id === event.payload))
        setPage(event.payload as Page);
    }).then((fn) => {
      if (alive) cleanups.push(fn);
      else fn();
    });
    const tick = setInterval(
      () => setNow(Math.floor(Date.now() / 1000)),
      20000,
    );
    return () => {
      alive = false;
      cleanups.forEach((fn) => fn());
      clearInterval(tick);
    };
  }, [reload]);
  async function attempt(fn: () => Promise<unknown>) {
    try {
      await fn();
    } catch (e) {
      setError(String(e));
    }
  }
  async function saveSettings(next: Settings) {
    await invoke("save_settings", { settings: next });
    await reload();
  }
  async function toggle(t: Task) {
    await attempt(() =>
      invoke("update_task", {
        id: t.id,
        expected: t.updated_at_utc,
        scope: "one",
        input: { ...t, status: t.status === "done" ? "todo" : "done" },
      }),
    );
  }
  const zone = settings?.timezone ?? "Asia/Ho_Chi_Minh";
  const visible = filterTasks(
    tasks,
    page,
    now,
    zone,
    search,
    priority,
    status,
    tag,
    filterDate,
  );
  const todays = tasks.filter((t) => todayTask(t, now, zone));
  const complete = todays.filter((t) => t.status === "done").length;
  const urgent = filterTasks(tasks, "today", now, zone).filter(
    (t) =>
      t.status !== "done" && (isOverdue(t, now, zone) || t.priority === "high"),
  ).filter(t=>!t.blocked&&!(t.snoozed_until_utc!==null&&t.snoozed_until_utc>now));
  const upcoming = filterTasks(tasks, "today", now, zone)
    .filter(
      (t) =>
        t.status !== "done" &&
        !t.blocked &&
        !(t.snoozed_until_utc!==null&&t.snoozed_until_utc>now) &&
        !urgent.some((u) => u.id === t.id) &&
        effective(t) !== null,
    )
    .slice(0, 3);
  const other = todays.filter(
    (t) =>
      !urgent.some((u) => u.id === t.id) &&
      !upcoming.some((u) => u.id === t.id),
  );
  const row = (t: Task) => (
    <TaskRow
      key={t.id}
      task={t}
      now={now}
      zone={zone}
      onToggle={(t) => void toggle(t)}
      onOpen={setEditing}
      onUnsnooze={async t=>{await invoke("unsnooze_task",{id:t.id});await reload();}}
      onSnooze={async t=>{await invoke("snooze_task",{id:t.id,expected:t.updated_at_utc,until:Math.floor(Date.now()/1000)+1800});await reload();}}
      onDelete={async t=>{if(t.template_id){setEditing(t);return;}await invoke("archive_task",{id:t.id,expected:t.updated_at_utc,scope:"one"});setDeleted(t.id);await reload();}}
      onRestore={
        page === "trash"
          ? (t) => void attempt(() => invoke("restore_task", { id: t.id }))
          : undefined
      }
    />
  );
  const engineLabel = engine?.last_error
    ? "Engine cần kiểm tra"
    : !engine?.running
      ? "Đang khởi động"
      : settings?.paused && (settings.pause_until_utc===null||settings.pause_until_utc>now)
        ? "Đang tạm tắt"
        : !settings?.notifications
          ? "Đã tắt thông báo"
          : "Engine đang chạy";
  const date = new Intl.DateTimeFormat("vi-VN", {
    timeZone: zoneName(zone),
    weekday: "long",
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
  }).format(new Date(now * 1000));
  return (
    <main
      className={`${sticky ? "sticky" : "dashboard"} ${settings?.theme === "dark" ? "dark" : ""}`}
    >
      {sticky ? (
        <>
          <header className="sticky-head">
            <div
              className="drag-zone"
              onMouseDown={() =>
                void attempt(() => getCurrentWindow().startDragging())
              }
            >
              <div className="brand">
                <CheckCheck size={20} />
                <b>TASK NOTE AI</b>
              </div>
              <small>{date}</small>
            </div>
            <div className="window-actions">
              <button
                aria-label="Ghim luôn ở trên"
                aria-pressed={settings?.pinned}
                onClick={() =>
                  settings &&
                  void attempt(() =>
                    saveSettings({ ...settings, pinned: !settings.pinned }),
                  )
                }
              >
                <Pin size={16} />
              </button>
              <button
                aria-label={collapsed ? "Mở Note" : "Thu gọn Note"}
                onClick={() =>
                  void attempt(async () => {
                    await invoke("collapse_note", { collapsed: !collapsed });
                    setCollapsed(!collapsed);
                  })
                }
              >
                {collapsed ? (
                  <ChevronDown size={17} />
                ) : (
                  <ChevronUp size={17} />
                )}
              </button>
              <button
                aria-label="Ẩn vào khay hệ thống"
                onClick={() => void attempt(() => invoke("hide_window"))}
              >
                <Minus size={17} />
              </button>
              <button
                aria-label="Mở bảng quản lý"
                onClick={() =>
                  void attempt(() => invoke("open_dashboard", { tab: "today" }))
                }
              >
                <ArrowUpRight size={17} />
              </button>
            </div>
          </header>
          {!collapsed && (
            <>
              <div className="progress-block">
                <div>
                  <b>HÔM NAY</b>
                  <span>
                    {complete}/{todays.length} hoàn thành
                  </span>
                </div>
                <progress value={complete} max={Math.max(todays.length, 1)} />
              </div>
              <div className="sticky-list">
                <QuickChat onSaved={reload} onOpen={setEditing} zone={zone}/>
                {urgent.length > 0 && (
                  <section>
                    <h3>
                      Cần xử lý ngay <span>{urgent.length}</span>
                    </h3>
                    {urgent.map(row)}
                  </section>
                )}
                {upcoming.length > 0 && (
                  <section>
                    <h3>Việc nên làm tiếp · Sắp đến hạn</h3>
                    {upcoming.map(row)}
                  </section>
                )}
                {other.length > 0 && (
                  <section>
                    <h3>Công việc khác</h3>
                    {other.map(row)}
                  </section>
                )}
                {todays.length === 0 && !loading && <Empty />}
              </div>
              <footer className="sticky-footer">
                <button className="primary" onClick={() => setEditing(null)}>
                  <Plus size={16} /> Thêm việc
                </button>
                <button
                  onClick={() =>
                    void attempt(() =>
                      invoke("open_dashboard", { tab: "assistant" }),
                    )
                  }
                >
                  <MessageSquare size={16} /> Trợ lý Task
                </button>
              </footer>
            </>
          )}
          <div className={`engine-line ${engine?.last_error ? "warning" : ""}`}>
            <i />
            {engineLabel}
          </div>
        </>
      ) : (
        <>
          <aside>
            <div className="brand">
              <CheckCheck />
              <b>TASK NOTE AI</b>
            </div>
            <p className="subtitle">Không bỏ lỡ việc quan trọng.</p>
            <nav>
              {pages.map((p) => (
                <button
                  key={p.id}
                  className={page === p.id ? "selected" : ""}
                  onClick={() => setPage(p.id)}
                >
                  <p.icon size={19} />
                  {p.label}
                </button>
              ))}
            </nav>
            <div className="sidebar-bottom">
              <div className="engine-line">
                <i />
                {engineLabel}
              </div>
              <small>Offline · Dữ liệu trên máy</small>
            </div>
          </aside>
          <div className="dashboard-main">
            <header className="dashboard-header">
              <div>
                <small>{date}</small>
                <h1>{pages.find((p) => p.id === page)?.label}</h1>
              </div>
              <button className="primary" onClick={() => setEditing(null)}>
                <Plus size={18} /> Thêm công việc
              </button>
            </header>
            {page === "assistant" ? (
              <Assistant zone={zone} onOpen={setEditing} settings={settings} onSettings={saveSettings} onGoals={()=>setPage("goals")} />
            ) : page === "goals" ? (
              <GoalsView tasks={tasks} zone={zone}/>
            ) : page === "settings" && settings ? (
              <SettingsView
                value={settings}
                engine={engine}
                onSave={saveSettings}
              />
            ) : (
              <>
                <div className="stats">
                  <div>
                    <span>Hôm nay & quá hạn</span>
                    <strong>
                      {todays.filter((t) => t.status !== "done").length}
                    </strong>
                  </div>
                  <div>
                    <span>Đã hoàn thành hôm nay</span>
                    <strong>
                      {
                        tasks.filter(
                          (t) =>
                            t.completed_at_utc !== null &&
                            new Intl.DateTimeFormat("sv-SE", {
                              timeZone: zoneName(zone),
                            }).format(new Date(t.completed_at_utc * 1000)) ===
                              new Intl.DateTimeFormat("sv-SE", {
                                timeZone: zoneName(zone),
                              }).format(new Date(now * 1000)),
                        ).length
                      }
                    </strong>
                  </div>
                  <div>
                    <span>Quá hạn</span>
                    <strong className="late-number">
                      {tasks.filter((t) => isOverdue(t, now, zone)).length}
                    </strong>
                  </div>
                </div>
                <div className="filters">
                  <label className="search">
                    <Search size={17} />
                    <input
                      aria-label="Tìm công việc"
                      placeholder="Tìm tên hoặc ghi chú…"
                      value={search}
                      onChange={(e) => setSearch(e.target.value)}
                    />
                  </label>
                  <input
                    aria-label="Lọc ngày hẹn"
                    type="date"
                    value={filterDate}
                    onChange={(e) => setFilterDate(e.target.value)}
                  />
                  <select
                    aria-label="Lọc ưu tiên"
                    value={priority}
                    onChange={(e) => setPriority(e.target.value)}
                  >
                    <option value="">Mọi ưu tiên</option>
                    <option value="high">Cao</option>
                    <option value="medium">Bình thường</option>
                    <option value="low">Thấp</option>
                  </select>
                  <select
                    aria-label="Lọc trạng thái"
                    value={status}
                    onChange={(e) => setStatus(e.target.value)}
                  >
                    <option value="">Mọi trạng thái</option>
                    <option value="todo">Cần làm</option>
                    <option value="doing">Đang làm</option>
                    <option value="done">Hoàn thành</option>
                  </select>
                  <select
                    aria-label="Lọc tag"
                    value={tag}
                    onChange={(e) => setTag(e.target.value)}
                  >
                    <option value="">Mọi dự án</option>
                    {[
                      ...new Set(
                        tasks.map((t) => t.tag_or_project).filter(Boolean),
                      ),
                    ].map((t) => (
                      <option key={t}>{t}</option>
                    ))}
                  </select>
                </div>
                <div className="task-panel">
                  {visible.length
                    ? visible.map(row)
                    : !loading && (
                        <Empty
                          filtered={
                            !!(
                              search ||
                              priority ||
                              status ||
                              tag ||
                              filterDate
                            )
                          }
                        />
                      )}
                </div>
              </>
            )}
          </div>
        </>
      )}
      {loading && <p className="loading">Đang đọc dữ liệu…</p>}
      {error && (
        <div className="app-error" role="alert">
          {error}
          <button onClick={() => void reload()}>Thử lại</button>
        </div>
      )}
      {deleted&&<div className="undo-bar" role="status">Đã xóa việc vào Thùng rác. <button onClick={()=>void attempt(async()=>{await invoke("restore_task",{id:deleted});setDeleted(null);await reload();})}>Hoàn tác</button><button aria-label="Đóng thông báo xóa" onClick={()=>setDeleted(null)}>×</button></div>}
      {settings && !settings.onboarding_done && (
        <Onboarding value={settings} onSave={saveSettings} />
      )}
      {editing !== undefined && (
        <TaskEditor
          task={editing}
          zone={zone}
          onClose={() => setEditing(undefined)}
          onSave={async (input: TaskInput, scope: string) => {
            if (editing)
              await invoke("update_task", {
                id: editing.id,
                input,
                expected: editing.updated_at_utc,
                scope,
              });
            else await invoke("create_task", { input });
            setEditing(undefined);
            await reload();
          }}
          onDelete={
            editing
              ? async (scope) => {
                  await invoke("archive_task", {
                    id: editing.id,
                    scope,
                    expected: editing.updated_at_utc,
                  });
                  setEditing(undefined);
                  await reload();
                }
              : undefined
          }
          onSnooze={
            editing
              ? async (until) => {
                  await invoke("snooze_task", {
                    id: editing.id,
                    until,
                    expected: editing.updated_at_utc,
                  });
                  setEditing(undefined);
                  await reload();
                }
              : undefined
          }
          onUnsnooze={editing?async()=>{await invoke("unsnooze_task",{id:editing.id});setEditing(undefined);await reload();}:undefined}
        />
      )}
    </main>
  );
}
function Empty({ filtered = false }: { filtered?: boolean }) {
  return (
    <div className="empty">
      <ClipboardList size={32} />
      <h3>
        {filtered ? "Không tìm thấy công việc" : "Một khoảng trống để bắt đầu"}
      </h3>
      <p>
        {filtered
          ? "Thử thay đổi bộ lọc."
          : "Thêm việc đầu tiên; ghi chú và lịch nhắc sẽ được lưu trên máy."}
      </p>
    </div>
  );
}
function Assistant({ zone,onOpen,settings,onSettings,onGoals }: { zone: string;onOpen:(t:Task)=>void;settings:Settings|null;onSettings:(s:Settings)=>Promise<void>;onGoals:()=>void }) {
  const [text, setText] = useState("");
  const [messages, setMessages] = useState<{ role: string; reply: Reply }[]>(
    [],
  );
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const request=useRef<{id:string;text:string}|null>(null);
  const [pending,setPending]=useState<string|null>(null);
  const [viewDate,setViewDate]=useState("");
  const [draftLoaded,setDraftLoaded]=useState(false);
  useEffect(()=>{let alive=true;void invoke<string|null>("load_draft",{key:"chat-input"}).then(raw=>{if(alive&&raw){const d=JSON.parse(raw);setText(d.text??"");request.current=d.request??null;setPending(d.pending??null);}}).catch(()=>{}).finally(()=>{if(alive)setDraftLoaded(true);});return()=>{alive=false;};},[]);
  useEffect(()=>{if(draftLoaded)void invoke("save_draft",{key:"chat-input",json:JSON.stringify({text,request:request.current,pending})}).catch(e=>setError(`Chưa lưu nháp: ${String(e)}`));},[text,pending,draftLoaded]);
  const end = useRef<HTMLDivElement>(null);
  useEffect(() => {
    void invoke<[string, string][]>("chat_history")
      .then((history) =>
        setMessages(
          history.map(([role, content]) => ({
            role,
            reply: { text: content, proposals: [] },
          })),
        ),
      )
      .catch((e) => setError(String(e)));
  }, []);
  useEffect(() => {
    end.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);
  async function send(value: string) {
    if (!value.trim() || busy) return;
    setBusy(true);
    setError("");
    setMessages((m) => [
      ...m,
      { role: "user", reply: { text: value, proposals: [] } },
    ]);
    try {
      if(request.current?.text!==value)request.current={id:pending??crypto.randomUUID(),text:value};
      await invoke("save_draft",{key:"chat-input",json:JSON.stringify({text:value,request:request.current,pending})});
      const reply = await invoke<Reply>("chat", { text: value,requestId:request.current.id,viewDate:viewDate||null });
      setMessages((m) => [...m, { role: "assistant", reply }]);
      setPending(reply.pending?request.current.id:null);setText("");request.current=null;
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <section className="assistant">
      <div className="assistant-intro">
        <h2>
          <MessageSquare size={22} /> Trợ lý Task{" "}
          <span className="badge">Offline</span>
        </h2>
        <p>
          Thêm việc bằng chat: lệnh rõ ràng tự lưu vào SQLite. Sửa/xóa bằng chat cần xem trước. Parser offline xử lý lệnh đơn giản; suy luận và chia mục tiêu cần AI đã cấu hình.
        </p>
        {settings&&<label className="toggle"><input type="checkbox" checked={settings.auto_plan} onChange={e=>{void onSettings({...settings,auto_plan:e.target.checked}).catch(e=>setError(String(e)));}}/>AI tự lập việc (tối đa {settings.ai_daily_task_limit} việc/ngày theo mục tiêu đã giao)</label>}
        <div className="actions"><button onClick={onGoals}>Xem mục tiêu</button><button disabled={busy} onClick={()=>{void invoke<{message:string;steps:unknown[]}>("replan").then(p=>setMessages(m=>[...m,{role:"assistant",reply:{text:`${p.message} Có ${p.steps.length} bước còn lại. Mở Mục tiêu để chọn thêm.`,proposals:[]}}])).catch(e=>setError(String(e)));}}>Lập kế hoạch hôm nay</button></div>
        <label>Ngày đang xem (chỉ dùng khi anh nói rõ)<input type="date" value={viewDate} onChange={e=>setViewDate(e.target.value)}/></label>
        <div className="examples">
          {[
            "Hôm nay anh có việc gì?",
            "Mai 9h gọi khách hàng",
            "Còn bao nhiêu việc chưa làm?",
          ].map((t) => (
            <button disabled={busy} key={t} onClick={() => void send(t)}>
              {t}
            </button>
          ))}
        </div>
      </div>
      <div className="chat-messages">
        {messages.map((m, i) =>
          m.role === "user" ? (
            <p className="user-message" key={i}>
              {m.reply.text}
            </p>
          ) : (
            <ProposalView
              key={i}
              reply={m.reply}
              zone={zone}
              onOpen={onOpen}
              onConfirm={async (token) => {
                await invoke("confirm_chat", { token });
              }}
            />
          ),
        )}
        {busy && <p>Đang đọc công việc…</p>}
        {error && (
          <p role="alert" className="error">
            {error}
          </p>
        )}
        {error&&<button disabled={busy} onClick={()=>void send(text)}>Thử lại yêu cầu (giữ request ID)</button>}
        <div ref={end} />
      </div>
      <form
        className="chat-form"
        onSubmit={(e) => {
          e.preventDefault();
          void send(text);
        }}
      >
        <input
          aria-label="Nhập lệnh cho trợ lý"
          maxLength={2000}
          placeholder="Ví dụ: Thêm việc kiểm tra báo cáo lúc 14h hôm nay"
          value={text}
          onChange={(e) => setText(e.target.value)}
        />
        <button className="primary" disabled={busy || !text.trim()}>
          Gửi
        </button>
      </form>
      <button
        className="clear-chat"
        onClick={async () => {
          try {
            await invoke("clear_chat");
            setMessages([]);
          } catch (e) {
            setError(String(e));
          }
        }}
      >
        Xóa lịch sử và đề xuất
      </button>
    </section>
  );
}
function Onboarding({
  value,
  onSave,
}: {
  value: Settings;
  onSave: (s: Settings) => Promise<void>;
}) {
  const [autostart, setAutostart] = useState(true);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  return (
    <div className="modal-backdrop">
      <section className="modal onboarding" role="dialog" aria-modal="true">
        <CheckCheck size={32} />
        <h2>Chào bạn, đây là Task Note AI</h2>
        <p>
          Ghi việc trên Desktop, nhắc trong 08:00–18:00. Dữ liệu lưu riêng trên
          máy; không cần tài khoản.
        </p>
        <label className="toggle">
          <input
            type="checkbox"
            checked={autostart}
            onChange={(e) => setAutostart(e.target.checked)}
          />
          Tôi đồng ý tự mở Sticky Note sau khi đăng nhập Windows.
        </label>
        <p className="hint">
          Đóng X sẽ ẩn vào khay hệ thống. Chọn “Thoát Task Note AI” ở khay để
          dừng hẳn.
        </p>
        <button
          disabled={busy}
          onClick={async () => {
            try {
              setMessage(await invoke<string>("test_notification"));
            } catch (e) {
              setMessage(String(e));
            }
          }}
        >
          Kiểm tra quyền · Gửi thông báo thử
        </button>
        {message && <p role="status">{message}</p>}
        <button
          className="primary"
          disabled={busy}
          onClick={async () => {
            setBusy(true);
            try {
              await onSave({ ...value, autostart, onboarding_done: true });
            } catch (e) {
              setMessage(String(e));
            } finally {
              setBusy(false);
            }
          }}
        >
          Bắt đầu sử dụng
        </button>
      </section>
    </div>
  );
}
function SettingsView({
  value,
  engine,
  onSave,
}: {
  value: Settings;
  engine: EngineStatus | null;
  onSave: (s: Settings) => Promise<void>;
}) {
  const [draft, setDraft] = useState(value);
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);
  const [backup, setBackup] = useState<{
    json: string;
    new_tasks: number;
    duplicate_tasks: number;
    templates: number;
  } | null>(null);
  useEffect(() => setDraft(value), [value]);
  async function action(fn: () => Promise<unknown>) {
    setBusy(true);
    setMessage("");
    try {
      const result = await fn();
      setMessage(
        typeof result === "string" ? result : "Đã thực hiện thành công.",
      );
    } catch (e) {
      setMessage(String(e));
    } finally {
      setBusy(false);
    }
  }
  const toggles: [keyof Settings, string][] = [
    ["autostart", "Tự chạy sau khi đăng nhập Windows"],
    ["show_sticky", "Hiện Sticky Note khi khởi động"],
    ["pinned", "Ghim Note luôn ở trên"],
    ["notifications", "Bật thông báo Windows"],
    ["hourly", "Đầu giờ 08:00–17:00"],
    ["pre_alert", "Nhắc trước hạn"],
    ["due_alert", "Nhắc đúng hạn"],
    ["daily_summary", "Tổng kết 18:00"],
    ["sound", "Âm thanh thông báo"],
    ["paused", "Tạm tắt thông báo"],
    ["save_chat", "Lưu hội thoại trên máy"],
  ];
  return (
    <div className="settings-view">
      <section className="settings-card">
        <h2>Trạng thái nhắc việc</h2>
        <dl>
          <dt>Engine</dt>
          <dd>{engine?.running ? "Đang chạy trong Rust" : "Đang khởi động"}</dd>
          <dt>Quyền Windows</dt>
          <dd>{engine?.permission || "Đang kiểm tra"}</dd>
          <dt>Kiểm tra gần nhất</dt>
          <dd>{formatTime(engine?.last_check ?? null, draft.timezone)}</dd>
          <dt>Đầu giờ tiếp theo</dt>
          <dd>
            {engine?.next_slot || "Đang tính"}
            {draft.paused || !draft.notifications
              ? " · Hiện tắt thông báo"
              : ""}
          </dd>
          <dt>Dữ liệu</dt>
          <dd className="db-path">{engine?.db_path}</dd>
        </dl>
        {engine?.last_error && <p className="error">{engine.last_error}</p>}
        <p className="hint">
          Windows Focus Assist / Do Not Disturb có thể ẩn toast. Nếu không thấy,
          mở Windows Settings → System → Notifications & actions, bật thông báo
          Task Note AI và kiểm tra Trung tâm thông báo.
        </p>
        <button
          disabled={busy}
          onClick={() => void action(() => invoke("test_notification"))}
        >
          Gửi thông báo thử
        </button>
      </section>
      <form
        className="settings-card"
        onSubmit={(e) => {
          e.preventDefault();
          void action(() => onSave(draft));
        }}
      >
        <h2>Tùy chọn</h2>
        {toggles.map(([key, label]) => (
          <label className="toggle" key={key}>
            <input
              type="checkbox"
              checked={Boolean(draft[key])}
              onChange={(e) => setDraft({ ...draft, [key]: e.target.checked,...(key==="paused"?{pause_until_utc:null}:{}) })}
            />
            {label}
          </label>
        ))}
        <label>
          Nhắc trước hạn (phút)
          <input
            type="number"
            min={1}
            max={120}
            value={draft.pre_minutes}
            onChange={(e) =>
              setDraft({ ...draft, pre_minutes: Number(e.target.value) })
            }
          />
        </label>
        <label>
          Múi giờ
          <select
            value={draft.timezone}
            onChange={(e) => setDraft({ ...draft, timezone: e.target.value })}
          >
            {[
              ...new Set([
                "Asia/Ho_Chi_Minh",
                "system",
                "UTC",
                "Asia/Tokyo",
                "America/New_York",
                "Europe/London",
                draft.timezone,
              ]),
            ].map((z) => (
              <option key={z} value={z}>
                {z === "system" ? "Theo hệ thống Windows" : z}
              </option>
            ))}
          </select>
        </label>
        <label>
          Giao diện
          <select
            value={draft.theme}
            onChange={(e) =>
              setDraft({ ...draft, theme: e.target.value as Settings["theme"] })
            }
          >
            <option value="light">Sáng</option>
            <option value="dark">Tối</option>
          </select>
        </label>
        <div className="actions">
          <button disabled={busy} className="primary">
            Lưu cài đặt
          </button>
          <button
            type="button"
            disabled={busy}
            onClick={() => void action(() => invoke("reset_position"))}
          >
            Đưa Note về góc màn hình
          </button>
        </div>
      </form>
      <section className="settings-card">
        <h2>Sao lưu và khôi phục</h2>
        <button disabled={busy} onClick={()=>void action(()=>invoke("save_sqlite_backup"))}>Sao lưu đầy đủ SQLite</button>
        <p>
          JSON phiên bản 1 chứa công việc, ghi chú và lịch lặp. Nhập chỉ thêm
          mục chưa có, không ghi đè công việc hiện tại.
        </p>
        <button
          disabled={busy}
          onClick={() =>
            void action(async () => {
              return invoke<string>("save_backup_file");
            })
          }
        >
          Xuất sao lưu JSON
        </button>
        <label>
          Chọn bản sao lưu
          <input
            type="file"
            accept=".json,application/json"
            disabled={busy}
            onChange={(e) => {
              const file = e.target.files?.[0];
              if (file)
                void action(async () => {
                  setBackup(null);
                  if (file.size > 20000000)
                    throw new Error("Tệp vượt quá 20 MB.");
                  const json = await file.text();
                  const preview = await invoke<{
                    new_tasks: number;
                    duplicate_tasks: number;
                    templates: number;
                  }>("preview_import", { json });
                  setBackup({ json, ...preview });
                  return "Đã kiểm tra dữ liệu; chưa nhập.";
                });
            }}
          />
        </label>
        {backup && (
          <div className="proposal">
            <b>
              Xem trước: {backup.new_tasks} việc mới · {backup.duplicate_tasks}{" "}
              việc trùng bỏ qua · {backup.templates} chuỗi
            </b>
            <div className="actions">
              <button
                className="primary"
                disabled={busy}
                onClick={() =>
                  void action(async () => {
                    await invoke("import_backup", { json: backup.json });
                    setBackup(null);
                  })
                }
              >
                Xác nhận nhập
              </button>
              <button disabled={busy} onClick={() => setBackup(null)}>
                Hủy
              </button>
            </div>
          </div>
        )}
      </section>
      <AISettings value={value} onSave={onSave}/>
      {message && (
        <p className="settings-result" role="status">
          {message}
        </p>
      )}
    </div>
  );
}
