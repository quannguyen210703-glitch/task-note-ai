# Kiến trúc V2

## Quyền sở hữu và đồng bộ

GUI hoặc CLI headless giữ khóa file `core.lock` suốt vòng đời dùng core. GUI giữ lock trước open/migrate DB và có duy nhất một scheduler Rust. CLI khi GUI chạy gửi typed request qua thư mục `cli-ipc` trong user data directory; ACL kế thừa chỉ current user/System/Admin có quyền ghi. Không mở TCP server. Hai CLI headless chờ lock có giới hạn; không tự chạy nền hoặc mở Sticky Note.

Request có UUID transport, request ID ổn định, receive/expiry time, typed action và dry-run. File ghi tạm rồi rename; response cũng atomic. Core kiểm tra schema/kích thước/expiry, commit rồi emit `tasks-changed`/`settings-changed` để hai React windows reload cùng SQLite. Timeout không đồng nghĩa chưa commit: caller retry cùng ID. Intent CLI pin action/expected version lần đầu; mutation receipt và task event được commit cùng transaction.

`Repository.mutation_gate` đồng bộ mutations và bước dispatch trong cùng process; pool một SQLite connection. Trước gửi toast core đọc lại task/settings, lọc task đã xong/xóa/hoãn/đổi hạn rồi claim unique dispatch key. Không xóa notification history khi bỏ hoãn. Một sửa đổi vừa commit trước dispatch được tôn trọng.

## Dữ liệu

Schema V1 giữ nguyên. Migration 2 thêm mutation_requests/service_intents, task_events, task_calendar, task_provenance, goals/goal_steps, plan_runs/ai_calls, chat_requests/batch_proposals và ui_drafts. Task all-day có local date/timezone riêng; không đổi nghĩa due_at_utc. Snooze là điều kiện nhắc, không phải status hoặc deadline mới. Status overdue/badge snooze là derived state.

Goal không được đếm như một task lớn cộng thêm con. Step trỏ task ID đã tạo; soft delete vẫn giữ liên kết, dismissed giữ dấu loại nên planner không tái tạo. Phụ thuộc kiểm tra từ task predecessor thật; suggestions dùng `blocked` tính từ join SQLite. Provenance cho biết original request, nguồn, goal/step và estimate. Planner tạo việc cả ngày, không tự xếp slot giờ.

## Chat / AI

Parser offline giải lệnh chắc chắn. Cloud chỉ gọi khi consent/enabled/model/key đầy đủ, từ Rust qua HTTPS cố định, timeout/response size bounded. Keyring dùng Windows-native. Request chỉ gửi nội dung yêu cầu hoặc mục tiêu liên quan, không gửi toàn DB. JSON schema strict và backend enum/date/length/dependency validation; mô hình không có SQL/shell/filesystem/tools.

Lệnh tạo rõ được lưu atomic sau validation. Past/duplicate/ambiguous cần preview/clarification; task mutation cũ qua chat cần confirm/version check. Batch undo chỉ cho task chưa có mutation sau tạo. Receipt được đọc trước khi gọi lại provider nếu lần trước đã commit nhưng reply thất bại. Ngày phân giải giữ theo receive time; retry sang ngày khác được trình lại dự kiến.

Planner chạy trong engine Rust, chỉ khi B + quyền goal + ngày/giờ cho phép. Durable local_date/run attempts và step linkage chống trùng qua restart/wake. Quota task tính cả task planner đã bị xóa; API attempts bền, retry cách tối thiểu 5 phút. Không có goal thì không bịa việc. Không gọi LLM cho mỗi toast.

## Bảo toàn và thử nghiệm

Online backup/VACUUM INTO giữ committed WAL. Startup V1→V2 tạo backup verified trước migration. Staging so từng trường cũ theo ID, có rollback/retry/lock fixture. Profile test có identifier TEST, thư mục DB riêng, cấm autostart/toast, có exit timer; không thay profile thật. Installed cutover không thực hiện khi người dùng chọn giữ V1.

Các giới hạn và kết quả lệnh thực nằm ở TEST_REPORT.md. Không dùng kiến trúc, mock hoặc fake clock thay chứng cứ native installed UI/notifications/physical sleep/API thật.
