# Nhật ký triển khai — 14/10/2026

## A — Khởi tạo

Repo ban đầu trống, chỉ có `.git`. Đọc đủ bốn tài liệu ở `C:\Users\HP\Downloads\task thong bao` sau khi người dùng cung cấp đường dẫn; sao chép nguyên bản vào repo. Windows 10 22H2 build 19045 x64; Node 24.19.0; pnpm 11.25.0. Cài Rust 1.99.0 vào `.tools`, không sửa PATH hệ thống.

Tạo Tauri 2/React 19/TypeScript/Vite, hai cửa sổ, capabilities tối thiểu và icon. Microsoft bootstrapper bị treo khi tải; tải trực tiếp installer OPC chính thức, extract nội bộ và chạy installer. Lần quiet không elevated trả 5007; chạy lại qua UAC, hiện đang cài MSVC/SDK. Shell desktop chưa được nghiệm thu trong mục này.

## B — Dữ liệu

SQLx SQLite WAL, migration phiên bản, prepared queries/transactions, CRUD với kiểm tra version, snooze, archive/restore, templates + occurrence unique, backup version 1 validate/preview/merge. Tests repository và persistence đã viết; chờ compiler MSVC để chạy.

## C — Desktop UX

Sticky Note/dashboard/tray, đóng X ẩn cửa sổ, single instance, window-state và work-area/DPI clamp, onboarding tự khởi động có đồng ý, giao diện Việt sáng/tối. Tất cả nút liên kết lệnh Rust. Frontend typecheck PASS, lint chưa có lỗi (một cảnh báo đã sửa), production build PASS. React component/logic tests: 5 PASS. Chưa kiểm thử UI desktop đã cài ở thời điểm ghi mục này.

## D — Nhắc việc

Scheduler Rust tick 20 giây, fake clock tests, 08:00/09:00–17:00/18:00, deadline/pre-alert, all-day, snooze, gộp, dedupe SQLite, gap/resume summary. Phát hiện plugin notification trên Desktop luôn trả Granted và dispatch bất đồng bộ không trả lỗi OS; chuyển đường Windows sang WinRT Setting/Show đồng bộ, XML escape và audio silent. Chưa xác nhận native toast trên installed build.

## E — Chat

Parser Vietnamese/deunicode giữ tiêu đề gốc, bảy ý định, truy vấn DB thật, lựa chọn khi nhiều task phù hợp, token xác nhận 10 phút với stale check/transaction, tùy chọn lưu lịch sử. Adapter cloud tách riêng và OFF, không có khóa API. Frontend confirmation/cancel tests PASS; backend tests chờ toolchain.

## F — QA/đóng gói

Đã test frontend 5/5, typecheck và production Vite build. Cargo test lần đầu thất bại trước khi compile ứng dụng vì `link.exe` chưa có; đang cài MSVC và Windows SDK rồi sẽ chạy lại. TEST_REPORT sẽ cập nhật bằng kết quả thực tế, không gán PASS cho thử nghiệm chưa thực hiện.

## Kết quả sau chạy backend

MSVC và Windows SDK đã cài xong, WebView2 155.0.4283.45 được phát hiện. Cargo workspace test PASS 11 tests, không warning/error trong lần cuối. Có regression dời cả chuỗi không tạo trùng occurrence. `cargo fmt --all --check` PASS. Seed demo xuất JSON riêng, không chạm DB người dùng. Release NSIS đang build; native QA chưa chốt.

Regression cuối: sửa cả chuỗi giữ các occurrence trong thùng rác. Cargo workspace 12 tests PASS (11 acceptance + 1 XML escaping), 0 failed; frontend 5 tests PASS.

## Installed build — lỗi phát hiện và xử lý

NSIS đầu cài thành công (exit 0), executable thật 18 MB, UI tiếng Việt được đọc qua Windows accessibility, DB trống. Công cụ capture window bị timeout và input geometry không có; không ghi PASS visual/manual. Phát hiện WinRT Setting trả 0x80070490 trước lần gửi đầu. C++/WinRT probe với App ID/shortcut đã cài: Show submitted, Setting=0, History=1. Sửa trạng thái NotInitialized và chỉ cho gửi khi Enabled hoặc trạng thái khởi tạo; các trạng thái Disabled vẫn chặn. Lỗi đọc quyền khác không dừng scheduler.

Thêm engine-status.json (không chứa nội dung task), CLI --settings/--assistant/--dashboard/--add và --test-notification để chẩn đoán bản cài. Kiểm tra auto-launch mặc định Dynamic có thể ghi HKLM khi chạy elevated; sửa sang CurrentUser và quote đường dẫn .exe có khoảng trắng. Bộ cài cuối đang build lại và sẽ được thử từ installed executable.

## Bàn giao hiện tại

Bản cuối có CurrentUser autostart/quoted path, sender NotInitialized, diagnostics local và CLI test notification. Rust 13 tests PASS; frontend 5 tests PASS; typecheck/lint/build PASS. NSIS final build/cài exit 0; SHA256 trong TEST_REPORT. Người dùng dừng Computer Use bằng Escape; dừng GUI QA, hủy phép thử deadline thực khi hidden và dọn đúng fixture/khôi phục cài đặt. Các ca manual chưa chạy giữ BLOCKED; chưa chốt nghiệm thu toàn bộ.
# Nâng V2 — 14–15/10/2026

Kết thúc kiểm chứng 15/10: 30 Rust (28 core + 2 desktop lib), 8 frontend, 46 CLI, 29 IPC/probes trên desktop release TEST; lặp nền thực sinh đúng một kỳ tiếp theo khi cửa sổ ẩn. NSIS/GUI/CLI 2.0.0 đã build. Staging snapshot mới 34 task so mọi trường cũ theo ID, integrity/foreign keys đạt; V1 vẫn giữ nguyên binary/data, không cutover. Artifacts và SHA256: `artifacts/MANIFEST-V2.json`. Các giới hạn AI/installed UI/calendar nêu rõ trong TEST_REPORT.md.

- Tìm đúng source/bản cài/data root; Git chưa có commit, giữ thay đổi hiện hữu.
- Online Backup API chụp DB WAL, verify và giữ source/lockfile/binary V1. Người dùng chọn giữ app V1 đang chạy, chỉ nhận bộ cài; không cutover DB thật.
- Migration additive, mutation receipt/events, calendar local date/timezone, provenance, goals/steps/plan runs/drafts và CLI pinned intents.
- CLI clap, fs2 owner lock, file IPC private current user; headless một lệnh không có scheduler. Test bắt lỗi Windows lock code 33, đã sửa.
- UI xóa mềm/hoàn tác/restore và snooze/unsnooze chung core; giữ deadline gốc, badge derived, chặn nhắc khi snooze; pause/resume độc lập.
- Chat tự lưu batch nguyên tử, clarification/past/duplicate preview, request ID/retry/undo guard; OpenAI REST schema + keyring Windows, consent mặc định tắt, model cấu hình được.
- Parent-child goal/steps, phụ thuộc, tombstones và giới hạn durable theo ngày. Chưa có tự xếp giờ làm vào calendar.
- SQLx 0.8.6 → 0.9.0, SQLite bundled 3.51.3 vá WAL-reset; identifier/data root giữ nguyên.
- Kết quả cuối xem TEST_REPORT.md; phân biệt fake-clock, binary IPC, installed UI/toast/sleep và provider thật. Phiên build qua đêm bị ngắt, được tiếp tục từ cache; không gán PASS cho lệnh ngắt.

## Tích hợp Codex trên máy — 15/10/2026

Thêm provider `codex` dùng Codex CLI đã cài và phiên đăng nhập ChatGPT của người dùng, không sao chép token/API key vào Task Note AI. UI có kiểm tra trạng thái, đăng nhập Codex, model tùy chọn và kiểm tra kết nối. Backend chạy `codex exec` ephemeral trong thư mục tạm trống, read-only, bỏ user config/rules và tắt các công cụ có khả năng thao tác máy; output buộc theo JSON schema hiện hữu, sau đó validate lần hai trước preview/mutation. Task CRUD vẫn chỉ qua repository/SQLite và quy tắc xác nhận cũ.

Đã xác nhận `codex-cli 0.162.0-alpha.17.2` có trên máy; trạng thái hiện tại là `Not logged in`, vì vậy không tuyên bố inference thật PASS. TypeScript typecheck/lint/build PASS; frontend 9/9, Rust workspace 31/31 và desktop release TEST 33 IPC PASS. NSIS mới đã build và đóng gói lại cùng source/manifest; V1 đang cài không bị thay đổi.

