export type Task = {
  id: string;
  title: string;
  description: string;
  due_at_utc: number | null;
  is_all_day: boolean;
  priority: "high" | "medium" | "low";
  status: "todo" | "doing" | "done";
  tag_or_project: string;
  recurrence_rule: string | null;
  snoozed_until_utc: number | null;
  created_at_utc: number;
  updated_at_utc: number;
  completed_at_utc: number | null;
  archived_at_utc: number | null;
  template_id: string | null;
  occurrence_date: string | null;
  local_date?: string | null;
  task_timezone?: string | null;
  source?: string | null;
  goal_id?: string | null;
  estimate_minutes?: number | null;
  blocked?: boolean;
};
export type TaskInput = Pick<
  Task,
  | "title"
  | "description"
  | "due_at_utc"
  | "is_all_day"
  | "priority"
  | "status"
  | "tag_or_project"
  | "recurrence_rule"
>;
export type Settings = {
  onboarding_done: boolean;
  autostart: boolean;
  show_sticky: boolean;
  pinned: boolean;
  notifications: boolean;
  hourly: boolean;
  pre_alert: boolean;
  due_alert: boolean;
  daily_summary: boolean;
  sound: boolean;
  paused: boolean;
  pause_until_utc: number | null;
  ai_enabled: boolean;
  ai_consent: boolean;
  ai_provider: string;
  ai_model: string;
  auto_plan: boolean;
  ai_daily_task_limit: number;
  ai_daily_call_limit: number;
  work_days: number[];
  pre_minutes: number;
  timezone: string;
  theme: "light" | "dark";
  save_chat: boolean;
};
export type EngineStatus = {
  running: boolean;
  last_check: number | null;
  next_slot: string;
  permission: string;
  last_error: string | null;
  db_path: string;
};
export type Card = {
  token: string;
  label: string;
  input: TaskInput;
  action: string;
};
export type Reply = { text: string; proposals: Card[]; saved?: Task[]; request_id?: string | null; batch_token?: string | null; pending?: boolean };
export type Page =
  | "today"
  | "all"
  | "upcoming"
  | "completed"
  | "assistant"
  | "goals"
  | "snoozed"
  | "settings"
  | "trash";
export type Goal = {id:string;title:string;description:string;desired_outcome:string;start_date:string;deadline:string|null;allowed_days:number[];auto_add:boolean;status:"active"|"paused"|"done";missing_information:string};
export type GoalStep = {id:string;goal_id:string;position:number;input:{title:string;description:string;date:string;time:string|null;priority:string;estimate_minutes:number|null;depends_on:number|null};task_id:string|null;dismissed:boolean};
