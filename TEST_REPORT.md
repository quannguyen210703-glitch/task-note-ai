# Kiểm thử Task Note AI 2.0

Thực hiện 14–15/10/2026, Asia/Saigon. Bản V1 đang dùng được giữ nguyên theo lựa chọn của người dùng. Không thử mutation lên task thật hoặc chạy installer đè V1.

## Kết quả đã chạy

| Lệnh / phép thử | Kết quả | Bằng chứng |
|---|---|---|
| `pnpm typecheck` | PASS | TypeScript không lỗi trong lượt build cuối |
| `pnpm lint` | PASS | ESLint không lỗi |
| `pnpm test` | PASS 9/9 | Chạy lại 15/10 sau tích hợp Codex; test chọn provider, bỏ yêu cầu API key và đọc trạng thái đăng nhập |
| `pnpm build` | PASS | `artifacts/release-checks/installer-dedupe-final.log`, Vite assets thật |
| `cargo test --manifest-path src-tauri/Cargo.toml --workspace` | PASS 31/31 | 11 acceptance + 18 upgrade/core + 2 desktop; gồm provider Codex không bắt buộc API model |
| `cargo test --locked --offline --manifest-path src-tauri/Cargo.toml -p task-note-ai --lib -j 1` | PASS 2/2 | XML escape và autostart path có space, `artifacts/release-checks/desktop-test.log` |
| Binary CLI, 46 commands/assertions | PASS | `scripts/cli_smoke.py`, `artifacts/v2-cli-smoke.json`, `artifacts/release-checks/cli-dedupe-smoke.log` |
| V1 snapshot → schema 2, so toàn bộ giá trị theo ID | PASS | `artifacts/v2-migration-comparison.json`, 6 bảng dữ liệu cũ |
| Migration lỗi giữa chừng → rollback → retry | PASS | `artifacts/v2-staging-tests.json`, không để bảng V2 dở dang/reset DB |
| DB lock → bounded error → retry đúng 1 task | PASS | `artifacts/v2-staging-tests.json` |
| Runtime SQLite | PASS 3.51.3 | `tasknote doctor --json` trên staging; schema 2 |
| `pnpm tauri build` (release + NSIS) | PASS | Installer mới 9.722.808 byte; SHA256 `e3e7355aef1bd51e6eb6cc96b83bed822c7c51e3d22656691bd7bb145e1b0032` |
| Desktop release TEST, 33 lệnh IPC/probes | PASS | `artifacts/v2-native-ipc-smoke.json`; profile TEST riêng, không toast/autostart/DB thật |
| Rust sinh lần lặp tiếp theo khi windows ẩn | PASS native TEST | `background_recurrence_while_hidden=true`; đúng 1 occurrence tương ứng, không mở Note, không toast/autostart |

Các lệnh bị ngắt qua đêm, sandbox esbuild bị chặn đọc thư mục cha, lỗi test lock Windows và lượt compile lib cũ trong lúc đang sửa không được ghi PASS. Đã chạy lại sau khi sửa. Dependency registry được kiểm tra bằng kết nối được phép, không tự mở cloud AI.

## Phạm vi chứng minh

- Chat offline 1/many task, ngày hôm nay/mai, cả ngày/buổi chiều, ngày đang xem không thay mặc định, ngày sai/giờ sai, thảo luận/quoted/draft không tự tạo.
- Regression do desktop smoke phát hiện: “thử” không phải “thứ”; “thu tiền” không phải lịch tuần; “email” không thành ngày mai; tên “Mai” được giữ; “đối chiếu” không bị coi là “đó” hoặc buổi chiều; “Thêm việc soạn …” là task, còn “Em soạn …” là yêu cầu draft. Lịch lặp rõ “mỗi thứ Hai 9h” được giữ.
- Atomic batch, failure không partial commit; request receipt/retry/reopen giữ ID; past/duplicate preview; clarification tiếp tục batch; undo từ chối task đã chỉnh.
- Xóa mềm/restore cùng ID, ghi chú/checklist/status/deadline giữ nguyên; snooze/unsnooze/global pause độc lập, idempotent, không hồi sinh done/trash.
- Fake clock 08:00/18:00/ngoài giờ, hourly/pre/due, wake không bắn bù; dedupe bền qua restart. V2 thay test V1 pre-alert trong thời gian hoãn bằng yêu cầu chặn tất cả nhắc tới hết hoãn.
- Parent-child goal, dependency blocking, sửa estimate, giới hạn 3 task chủ động, restart/daily dedupe, tombstone không tạo lại task đã xóa.
- CLI JSON một envelope, Unicode/quote/path có space, ID/date lỗi, dry-run/doctor không mutation DB, tất cả CRUD/snooze/delete/restore/notifications/backup, thiếu AI trả code 7 và không giả kết quả.
- Hai CLI headless đồng thời đều thành công, một owner lock; CLI không có scheduler nền. Backup live có WAL chứa giao dịch commit mới nhất trong fixture, không chỉ khớp count.

## Các ca Windows / AI còn BLOCKED hoặc chưa hoàn thiện

| Ca | Trạng thái / lý do |
|---|---|
| Installer upgrade tại chỗ, backup cuối/cutover, đối soát DB thật sau cài | BLOCKED theo lựa chọn **giữ bản đang chạy, chỉ bàn giao bộ cài**. Có đối soát staging, không gọi đó là đã nâng laptop |
| Installed V2 UI click xóa/hoãn/bỏ hoãn, Note/dashboard nhận event trực quan | BLOCKED chưa thao tác native installed UI. UI callback tests và binary IPC chỉ chứng minh phần được test |
| Native toast từ installed V2, hide/quit tray, đăng nhập/autostart, DPI/monitor, sleep/wake vật lý, soak | BLOCKED chưa thực hiện; không đổi giờ/sleep/đăng xuất máy người dùng để tạo PASS |
| Kết nối mô hình thật, chất lượng chia mục tiêu/draft ngoài fixture | BLOCKED: Codex CLI 0.162.0-alpha.17.2 đã được phát hiện nhưng `codex login status` trả `Not logged in`; không có API key. Adapter Codex local và OpenAI REST đã viết; schema/test không thay cho lượt inference sau khi người dùng đăng nhập |
| Thiếu dung lượng đĩa thật, chạy dưới admin profile khác | BLOCKED không gây các tình huống đó trên máy đang dùng. DB lock và migration rollback đã có fixture thực |
| Rollback có writes sau cutover | BLOCKED chưa cutover; có hướng dẫn không restore mù, chưa có automated reverse-merge |
| Tự xếp giờ làm vào calendar, giữ sức chứa theo interval hẹn cụ thể | CHƯA TRIỂN KHAI. Planner tạo bước cả ngày, có estimate/capacity thận trọng; không tự chen/di chuyển hẹn |
| Đổi ngày bắt đầu/quy tắc cả chuỗi qua mutation core mới | GIỚI HẠN AN TOÀN. Đổi nội dung/giờ cùng ngày giữ ID; trường hợp khác báo lý do và chọn lần này/tạo lịch mới |

Không tuyên bố đạt toàn bộ các tiêu chí AI/installed Windows của prompt. `code/build đã có` và `đã nâng trên profile thật` là hai trạng thái khác nhau.

## Những lỗi đã được phát hiện và sửa trong vòng kiểm chứng

Các lượt lỗi không được tính PASS: startup TEST đầu tiên hết thời gian chờ 20 giây; parser hiểu nhầm “thử”/“đối”; metadata calendar cố gắn vào UUID mới khi occurrence đã tồn tại gây FK error. Đã sửa nhận diện theo từ/ngữ cảnh, bỏ ghi metadata cho insert dedupe không tạo hàng, giữ metadata lịch sử khi sửa chuỗi, rồi chạy lại toàn bộ test core và native background recurrence trên binary cuối. Lượt build CLI bị Windows khóa executable khi smoke đang chạy được thực hiện lại sau khi tiến trình kết thúc. Các lượt workspace build bị dừng để giảm tải RAM không được coi là PASS; các lệnh core và desktop lib riêng ở bảng trên đã chạy xong.

Ngày cả ngày và các occurrence mới sinh giữ local_date/timezone khi đổi app timezone; Note, CLI và điều kiện summary dùng cùng ngày. CLI chặn heartbeat V1 stale/partial, có test core và binary code 6. Final installer/CLI/source có size và SHA256 trong `artifacts/MANIFEST-V2.json`. Chưa chạy installer vào profile người dùng.
