import type { Task, TaskInput, Page } from "./types";
export const blank: TaskInput = {
  title: "",
  description: "",
  due_at_utc: null,
  is_all_day: false,
  priority: "medium",
  status: "todo",
  tag_or_project: "",
  recurrence_rule: null,
};
export function zoneName(zone: string) {
  return zone === "system"
    ? Intl.DateTimeFormat().resolvedOptions().timeZone
    : zone;
}
export function dateKey(stamp: number, zone: string) {
  return new Intl.DateTimeFormat("sv-SE", {
    timeZone: zoneName(zone),
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).format(new Date(stamp * 1000));
}
export function localInput(stamp: number | null, zone: string) {
  if (stamp === null) return "";
  const parts = new Intl.DateTimeFormat("sv-SE", {
    timeZone: zoneName(zone),
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hourCycle: "h23",
  }).formatToParts(new Date(stamp * 1000));
  const part = (s: string) => parts.find((p) => p.type === s)?.value ?? "";
  return `${part("year")}-${part("month")}-${part("day")}T${part("hour")}:${part("minute")}`;
}
export function parseLocal(value: string, zone: string): number | null {
  if (!value) return null;
  const [date, time] = value.split("T");
  const [y, m, d] = date.split("-").map(Number);
  const [h, minute] = time.split(":").map(Number);
  let stamp = Date.UTC(y, m - 1, d, h, minute) / 1000;
  for (let i = 0; i < 3; i++) {
    const shown = localInput(stamp, zone);
    const [sd, st] = shown.split("T");
    const [sy, sm, sday] = sd.split("-").map(Number);
    const [sh, sminute] = st.split(":").map(Number);
    const difference =
      (Date.UTC(y, m - 1, d, h, minute) -
        Date.UTC(sy, sm - 1, sday, sh, sminute)) /
      1000;
    stamp += difference;
    if (difference === 0) break;
  }
  if (localInput(stamp, zone) !== value)
    throw new Error("Giờ này không tồn tại trong múi giờ đã chọn.");
  return stamp;
}
export function formatTime(stamp: number | null, zone: string, allDay = false) {
  if (stamp === null) return "Không đặt hạn";
  return new Intl.DateTimeFormat("vi-VN", {
    timeZone: zoneName(zone),
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    ...(allDay ? {} : { hour: "2-digit", minute: "2-digit", hourCycle: "h23" }),
  }).format(new Date(stamp * 1000));
}
export function effective(t: Task) {
  return t.due_at_utc;
}
export function taskDate(t:Task,zone:string){return t.is_all_day&&t.local_date?t.local_date:t.due_at_utc===null?null:dateKey(t.due_at_utc,zone);}
export function isOverdue(t: Task, now: number, zone: string) {
  const due = effective(t);
  return (
    t.status !== "done" &&
    t.archived_at_utc === null &&
    due !== null &&
    (t.is_all_day ? taskDate(t,zone)! < dateKey(now, zone) : due < now)
  );
}
export function todayTask(t: Task, now: number, zone: string) {
  const due = effective(t);
  return (
    t.archived_at_utc === null &&
    ((due === null &&
      (t.status !== "done" ||
        (t.completed_at_utc !== null &&
          dateKey(t.completed_at_utc, zone) === dateKey(now, zone)))) ||
      (due !== null && taskDate(t,zone) === dateKey(now, zone)) ||
      isOverdue(t, now, zone))
  );
}
export function filterTasks(
  tasks: Task[],
  page: Page,
  now: number,
  zone: string,
  search = "",
  priority = "",
  status = "",
  tag = "",
  date = "",
) {
  const today = dateKey(now, zone);
  const week = dateKey(now + 7 * 86400, zone);
  return tasks
    .filter((t) => {
      if (page === "trash") return t.archived_at_utc !== null;
      if (t.archived_at_utc !== null) return false;
      if (page === "snoozed" && !(t.status !== "done" && t.snoozed_until_utc !== null && t.snoozed_until_utc > now)) return false;
      if (page === "completed" && t.status !== "done") return false;
      if (page === "today" && !todayTask(t, now, zone)) return false;
      if (page === "upcoming") {
        const due = effective(t);
        if (
          due === null ||
          t.status === "done" ||
          taskDate(t, zone)! <= today ||
          taskDate(t, zone)! > week
        )
          return false;
      }
      return true;
    })
    .filter(
      (t) =>
        (t.title + " " + t.description)
          .toLocaleLowerCase("vi")
          .includes(search.toLocaleLowerCase("vi")) &&
        (!priority || t.priority === priority) &&
        (!status || t.status === status) &&
        (!tag || t.tag_or_project === tag) &&
        (!date ||
          (effective(t) !== null && taskDate(t, zone) === date)),
    )
    .sort((a, b) => {
      const late =
        Number(isOverdue(b, now, zone)) - Number(isOverdue(a, now, zone));
      if (late) return late;
      const p = { high: 0, medium: 1, low: 2 };
      return (
        p[a.priority] - p[b.priority] ||
        (effective(a) ?? Infinity) - (effective(b) ?? Infinity) ||
        a.title.localeCompare(b.title, "vi")
      );
    });
}
export const priorityLabels = {
  high: "Cao",
  medium: "Bình thường",
  low: "Thấp",
};
export const statusLabels = {
  todo: "Cần làm",
  doing: "Đang làm",
  done: "Hoàn thành",
};
export function recurrenceLabel(rule: string | null) {
  if (!rule) return "Không lặp";
  if (rule === "daily") return "Hằng ngày";
  if (rule === "weekdays") return "Ngày làm việc";
  return `Mỗi ${["thứ Hai", "thứ Ba", "thứ Tư", "thứ Năm", "thứ Sáu", "thứ Bảy", "Chủ nhật"][Number(rule.split(":")[1])]}`;
}
