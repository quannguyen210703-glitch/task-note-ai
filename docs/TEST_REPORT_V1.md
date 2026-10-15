# Báo cáo kiểm thử Task Note AI V1.0

Ngày: 14/10/2026 (Asia/Saigon). Bộ cài đã build/cài thành công; chưa chốt toàn bộ nghiệm thu vì các ca manual bên dưới chưa thực hiện.

## Môi trường và kết quả lệnh

Windows 10 22H2 19045 x64; Node 24.19.0; pnpm 11.25.0; Rust stable MSVC 1.99.0; MSVC 14.44; SDK 10.0.26100. Tauri 2.12.3, SQLx 0.8.6 (xem lockfile).

- `pnpm typecheck`: PASS.
- `pnpm lint`: PASS; cảnh báo cleanup effect ở lần đầu đã sửa.
- `pnpm test`: PASS 5/5; log `artifacts/frontend-test.log`.
- `pnpm build`: PASS, Vite production assets tạo thật.
- `cargo test --manifest-path src-tauri/Cargo.toml --workspace`: PASS 13 tests (11 acceptance + XML escape + đường dẫn autostart), 0 failed; log `artifacts/cargo-test.log`. Các lỗi toolchain ban đầu đã xử lý, không gán PASS cho các lần chạy lỗi.
- `pnpm tauri build`: PASS release + NSIS, log `artifacts/build-final.log`. Bộ cài cuối cài thành công, exit 0.
- Bộ cài: `artifacts/Task Note AI_1.0.0_x64-setup.exe`, SHA256 `D447B3C80FAFDB8B5BCD49527028EBE8A224C24F186C51F036D897B332D2FE23`.
- Native reference probe: WinRT Show thành công, Setting=0 (Enabled), History=1 cho App ID/shortcut đã cài; sẽ xác minh thêm từ chính installed executable.
- Windows Computer Use: đọc được accessibility UI, capture bị timeout và click không có geometry. Không gán PASS cho visual/interaction chưa thực hiện.

## Các ca nghiệm thu

PASS (automation) chỉ xác nhận logic được test, không thay thế nghiệm thu native/manual ghi ở cột thực hiện.

| ID | Thực hiện | Kết quả | Bằng chứng / ghi chú |
|---|---|---|---|
| T01 | Cài NSIS, UI thật | PASS install/UIA; BLOCKED visual | NSIS exit 0; native installed process và UI tiếng Việt/empty state đọc qua accessibility. Capture timeout; chưa xác nhận tràn/DPI bằng hình ảnh. |
| T02 | SQLite đóng/mở lại + app restart | PASS automation / BLOCKED native | SQLite file reopen giữ task, không trùng; chờ restart installed build |
| T03 | Autostart đăng xuất/đăng nhập | BLOCKED | Cần phiên đăng nhập Windows; không tự đăng xuất người dùng |
| T04 | X ẩn cửa sổ, scheduler tiếp tục | BLOCKED | Chưa thao tác manual; Computer Use đã dừng |
| T05 | Thoát từ tray | BLOCKED | Chưa thao tác manual; Computer Use đã dừng |
| T06 | Mở hai instance | PASS installed bản đầu | Mở .exe lần hai vẫn một process PID 18120; phần single-instance không đổi trong bản cuối. |
| T07 | Fake clock 07:59/08:00/08:01 | PASS (automation) | Fake clock / integration tests PASS; artifacts/cargo-test.log |
| T08 | Fake clock các slot 09–17 | PASS (automation) | Fake clock / integration tests PASS; artifacts/cargo-test.log |
| T09 | Fake clock 18:00/18:01 | PASS (automation) | Fake clock / integration tests PASS; artifacts/cargo-test.log |
| T10 | Pre 10:15 / due 10:30 | PASS (automation) | Fake clock / integration tests PASS; artifacts/cargo-test.log |
| T11 | Hoàn thành trước pre-alert | PASS (automation) | Fake clock / integration tests PASS; artifacts/cargo-test.log |
| T12 | Snooze đổi giờ hiệu lực | PASS (automation) | Fake clock / integration tests PASS; artifacts/cargo-test.log |
| T13 | Không gửi sau 18:00 | PASS (automation) | Fake clock / integration tests PASS; artifacts/cargo-test.log |
| T14 | Fake-clock wake + sleep vật lý | PASS automation / BLOCKED native | Fake-clock 09:30→12:20 cho đúng 1 reconcile; chưa sleep vật lý |
| T15 | Dedupe qua DB reopen | PASS (automation) | Fake clock / integration tests PASS; artifacts/cargo-test.log |
| T16 | Giữ deadline quá hạn qua ngày | PASS (automation) | Fake clock / integration tests PASS; artifacts/cargo-test.log |
| T17 | Weekly occurrence độc lập, unique | PASS (automation) | Fake clock / integration tests PASS; artifacts/cargo-test.log |
| T18 | Tắt notifications / OS từ chối | PASS automation / BLOCKED native | Off/pause không tạo toast trong fake-clock; chưa thay quyền Windows thật |
| T19 | Chat Mai 9h preview | PASS (automation) | SQLite/parser integration PASS; artifacts/cargo-test.log; frontend không mutate trước confirm PASS |
| T20 | Tên mơ hồ hỏi chọn | PASS (automation) | SQLite/parser integration PASS; artifacts/cargo-test.log |
| T21 | Confirm DB + UI đồng bộ | BLOCKED | Frontend token-confirm 1 lần PASS; backend PASS; native UI chưa thao tác |
| T22 | Export/import DB mới | PASS (automation) | SQLite/parser integration PASS; artifacts/cargo-test.log |
| T23 | DPI/đổi màn hình | BLOCKED | Có work-area clamp; chưa rút màn hình / đổi scale vật lý |
| T24 | Dashboard ẩn nhiều giờ | BLOCKED | Engine Rust độc lập; chưa soak test nhiều giờ |
| T25 | Native toast trên app đã cài | PASS reference API; BLOCKED final-app/UI | C++/WinRT với App ID/shortcut đã cài: Show submitted, Setting=0, History=1. Chưa chạy --test-notification từ .exe cuối hoặc xác nhận toast hiển thị bằng UI. |
| T26 | Empty-state, no-task không spam | PASS (automation) | Logic frontend PASS, scheduler fake-clock PASS |
| T27 | Clock lùi / timezone | PASS (automation) | Fake clock / integration tests PASS; artifacts/cargo-test.log; test UTC↔Việt Nam frontend PASS |
| T28 | Việt/SQL special text | PASS (automation) | SQLite Unicode/special text PASS; artifacts/cargo-test.log; React hiển thị escaped mặc định |

## Giới hạn cần nghiệm thu thật

Không thay đổi giờ hệ thống, scale Windows, đăng xuất hay sleep máy người dùng chỉ để tạo kết quả PASS. Các ca đó cần một phiên kiểm thử riêng có điều kiện phù hợp. Clock injection xác nhận quy tắc scheduler; không chứng minh mọi trường hợp Windows Focus Assist/hardware. Kết quả installed build sẽ được ghi bổ sung khi thực hiện.

## Dừng kiểm thử GUI và dọn fixture

Người dùng dừng Windows Computer Use bằng phím Escape. Không tiếp tục thao tác GUI. Phép thử hạn thực hai phút khi UI ẩn bị hủy trước khi mở ứng dụng, nên T24 không PASS. Fixture UUID 1ae04144-91b1-476d-800a-e473c924d967 được dọn riêng và cài đặt gốc được khôi phục; không xóa task khác. Bằng chứng cleanup: `artifacts/native-scheduler-evidence.json` có dispatches rỗng, không dùng làm chứng cứ gửi.

Các ca còn cần manual: đóng X/tray/quit, CRUD/chat đồng bộ hai cửa sổ từ tương tác, đăng xuất/đăng nhập, sleep vật lý, DPI/monitor, soak nhiều giờ, OS deny/Focus Assist và toast từ installed executable cuối. Không tuyên bố V1 đã đạt toàn bộ nghiệm thu.
