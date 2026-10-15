import { afterEach, describe, expect, it, vi } from "vitest";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { TaskEditor, ProposalView, TaskRow } from "./components";
import { AISettings } from "./upgrade";
import { filterTasks, parseLocal, localInput, blank } from "./utils";
import type { Task } from "./types";
import { invoke } from "@tauri-apps/api/core";
vi.mock("@tauri-apps/api/core",()=>({invoke:vi.fn().mockResolvedValue(null)}));
afterEach(cleanup);
const zone = "Asia/Ho_Chi_Minh";
const now = parseLocal("2026-10-14T08:00", zone)!;
const task: Task = {
  ...blank,
  id: "1",
  title: "Báo cáo",
  due_at_utc: parseLocal("2026-10-14T10:30", zone),
  created_at_utc: now,
  updated_at_utc: now,
  completed_at_utc: null,
  archived_at_utc: null,
  template_id: null,
  occurrence_date: null,
  snoozed_until_utc: null,
};
describe("Dữ liệu giao diện", () => {
  it("cho phép chọn Codex trên máy mà không yêu cầu API key", async () => {
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === "ai_key") return false;
      if (command === "codex_status")
        return { installed: true, signed_in: true, detail: "Đã đăng nhập Codex trên máy." };
      return null;
    });
    const settings = {
      onboarding_done: true, autostart: false, show_sticky: true, pinned: true,
      notifications: true, hourly: true, pre_alert: true, due_alert: true,
      daily_summary: true, sound: true, paused: false, pause_until_utc: null,
      ai_enabled: false, ai_consent: false, ai_provider: "openai", ai_model: "",
      auto_plan: false, ai_daily_task_limit: 3, ai_daily_call_limit: 3,
      work_days: [0,1,2,3,4], pre_minutes: 15, timezone: zone,
      theme: "light" as const, save_chat: false,
    };
    render(<AISettings value={settings} onSave={vi.fn().mockResolvedValue(undefined)}/>);
    fireEvent.change(screen.getByLabelText("Provider"), { target: { value: "codex" } });
    expect(screen.getByLabelText("Model Codex (không bắt buộc)")).toHaveProperty("value", "");
    expect(screen.queryByLabelText("Nhập/thay khóa API")).toBeNull();
    await waitFor(() => expect(screen.getByRole("status").textContent).toContain("Đã đăng nhập Codex trên máy."));
  });
  it("UTC ↔ giờ Việt Nam không phụ thuộc timezone máy", () => {
    expect(localInput(now, zone)).toBe("2026-10-14T08:00");
    expect(now).toBe(Date.UTC(2026, 9, 14, 1, 0) / 1000);
  });
  it("lọc hôm nay, trạng thái, ưu tiên, tag và giữ việc quá hạn", () => {
    const late = {
      ...task,
      id: "2",
      title: "Việc cũ",
      due_at_utc: now - 86400,
      priority: "high" as const,
      tag_or_project: "A",
    };
    expect(filterTasks([task, late], "today", now, zone)[0].id).toBe("2");
    expect(
      filterTasks([task, late], "all", now, zone, "", "high", "", "A").length,
    ).toBe(1);
    expect(filterTasks([task, late], "all", now, zone, "không có").length).toBe(
      0,
    );
  });
  it("thêm nhanh chỉ lưu khi gửi form và hiện lỗi backend", async () => {
    const save = vi.fn().mockRejectedValue(new Error("SQLite không ghi được"));
    render(
      <TaskEditor task={null} zone={zone} onSave={save} onClose={() => {}} />,
    );
    fireEvent.change(screen.getByLabelText("Tiêu đề"), {
      target: { value: "Gọi khách" },
    });
    expect(save).not.toHaveBeenCalled();
    fireEvent.click(screen.getByText("Xác nhận lưu"));
    await waitFor(() => expect(save).toHaveBeenCalledTimes(1));
    expect(await screen.findByRole("alert")).toHaveProperty(
      "textContent",
      expect.stringContaining("SQLite"),
    );
  });
  it("chat cần xác nhận; hủy không mutate", async () => {
    const confirm = vi.fn().mockResolvedValue(undefined);
    render(
      <ProposalView
        reply={{
          text: "Chưa lưu",
          proposals: [
            {
              token: "token",
              label: "Thêm",
              input: { ...blank, title: "Gọi khách" },
              action: "create",
            },
          ],
        }}
        zone={zone}
        onConfirm={confirm}
      />,
    );
    expect(confirm).not.toHaveBeenCalled();
    fireEvent.click(screen.getByText("Hủy"));
    expect(confirm).not.toHaveBeenCalled();
  });
  it("chat xác nhận đúng token một lần", async () => {
    const confirm = vi.fn().mockResolvedValue(undefined);
    render(
      <ProposalView
        reply={{
          text: "Chưa lưu",
          proposals: [
            {
              token: "token",
              label: "Thêm",
              input: { ...blank, title: "Gọi khách" },
              action: "create",
            },
          ],
        }}
        zone={zone}
        onConfirm={confirm}
      />,
    );
    fireEvent.click(screen.getByText("Xác nhận"));
    await waitFor(() => expect(confirm).toHaveBeenCalledWith("token"));
    expect(confirm).toHaveBeenCalledTimes(1);
  });
  it("bỏ hoãn ở task đang doing gọi đúng ID; lỗi không giả thành công",async()=>{
    const unsnooze=vi.fn().mockRejectedValue(new Error("Công việc đã thay đổi"));
    render(<TaskRow task={{...task,status:"doing",snoozed_until_utc:now+1800}} now={now} zone={zone} onOpen={()=>{}} onToggle={()=>{}} onUnsnooze={unsnooze}/>);
    fireEvent.click(screen.getByText("Bỏ tạm hoãn"));
    await waitFor(()=>expect(unsnooze).toHaveBeenCalledWith(expect.objectContaining({id:"1",status:"doing"})));
    expect(await screen.findByRole("alert")).toHaveProperty("textContent",expect.stringContaining("thay đổi"));
  });
  it("hoãn hết hạn không còn badge hoặc nút bỏ hoãn; vẫn giữ quá hạn gốc",()=>{
    render(<TaskRow task={{...task,due_at_utc:now-3600,snoozed_until_utc:now-1}} now={now} zone={zone} onOpen={()=>{}} onToggle={()=>{}} onUnsnooze={async()=>{}}/>);
    expect(screen.queryByText("Bỏ tạm hoãn")).toBeNull();expect(screen.getByText("Quá hạn")).toBeTruthy();
  });
  it("xóa mềm trực tiếp cả task đã hoàn thành, không qua confirm lần hai",async()=>{
    const remove=vi.fn().mockResolvedValue(undefined);
    render(<TaskRow task={{...task,status:"done"}} now={now} zone={zone} onOpen={()=>{}} onToggle={()=>{}} onDelete={remove}/>);
    fireEvent.click(screen.getByLabelText("Xóa việc Báo cáo"));await waitFor(()=>expect(remove).toHaveBeenCalledTimes(1));
  });
});
