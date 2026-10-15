# Task Note AI 2.0 — Windows

Desktop thật: Tauri v2 + Rust + React/TypeScript + SQLite/SQLx. Task, chat, CLI và nhắc dùng chung dữ liệu. **Bản V1 trên máy vẫn giữ nguyên theo lựa chọn của anh; chưa cài đè hoặc migrate DB thật.**

## Bàn giao

Artifact đã build: `artifacts/Task Note AI_2.0.0_x64-setup.exe`, `artifacts/tasknote.exe`, `artifacts/task-note-ai.exe`, `artifacts/TaskNoteAI-2.0.0-source.zip`; SHA256 trong `artifacts/MANIFEST-V2.json`. 31 test Rust, 9 frontend và desktop TEST 33 lệnh IPC/lặp nền đã đạt trong lượt tích hợp Codex; xem TEST_REPORT.md để phân biệt installed/manual còn chưa kiểm chứng. [CLI_QUICKSTART.md](CLI_QUICKSTART.md) có lệnh dùng ngay. [DATA_PRESERVATION_REPORT.md](DATA_PRESERVATION_REPORT.md) có checkpoint/đối soát/rollback; PROJECT_DISCOVERY.md và VERSION_AUDIT.md ghi đường dẫn và dependency.

Không chia sẻ `artifacts/preservation`, staging hoặc test-profile: có thể chứa dữ liệu cá nhân. Gói source V2 loại các thư mục đó, API key, node_modules và target.

## Cài / nâng V1

1. Lưu nội dung đang nhập, thoát V1 sạch từ tray khi anh thực sự muốn nâng. Không cài đè lúc còn ô nhập chưa lưu.
2. Tạo backup mới nhất bằng Online Backup API (`scripts/preserve.py`), không chỉ copy `.sqlite` khi có WAL. Verify integrity/foreign keys, giữ binary/source cũ.
3. Chạy installer V2 dưới đúng Windows user, giữ thư mục cài. Identifier vẫn `vn.tasknoteai.desktop`; DB vẫn `%APPDATA%\vn.tasknoteai.desktop\task-note.sqlite`.
4. Core tự tạo/verify `pre-v2-<UUID>.sqlite` trước migration. Lỗi migration không reset DB hoặc tạo DB rỗng thay thế.
5. Mở Note, chạy `tasknote doctor --json`, đối chiếu task doing/ghi chú/deadline/snooze theo ID trước khi dùng tiếp.

Đóng cửa sổ ẩn vào tray; chọn **Thoát Task Note AI** mới dừng scheduler. Autostart chỉ bật theo lựa chọn onboarding/cài đặt sau đăng nhập. Windows Focus Assist/Do Not Disturb/quyền hệ thống có thể chặn toast. Profile thử không bật autostart/toast.

## Công việc và thông báo

- Trạng thái Chưa làm / Đang làm / Hoàn thành; quá hạn giữ nguyên deadline.
- **Xóa việc** vào Thùng rác, có Hoàn tác/Khôi phục đúng ID, dùng cả todo/doing/done. Lịch lặp chọn scope; không xóa lịch sử đã hoàn thành.
- **Tạm hoãn nhắc** 15/30/60 phút hoặc giờ cụ thể chỉ chặn nhắc, không đổi hạn/trạng thái.
- **Bỏ tạm hoãn** ở task/chi tiết/danh sách Đang tạm hoãn; gọi lặp an toàn, không hồi sinh task xong/xóa hoặc phát lại mốc cũ.
- Pause/resume toàn app độc lập snooze/unsnooze từng task.
- Rust nhắc 08:00 mở đầu, 09:00–17:00 đầu giờ, 18:00 tổng kết; trước/đúng hạn theo toggle. Không phát bù hàng loạt sau wake, không phụ thuộc timer React.
- Đổi giờ cùng chuỗi trong cùng ngày bắt đầu giữ occurrence ID. Đổi ngày bắt đầu/quy tắc cả chuỗi còn giới hạn an toàn; core báo lý do, chọn Lần này hoặc tạo lịch mới.

## Chat và AI

Lệnh đơn giản rõ ràng tự lưu nguyên tử, không cần bật AI tự lập việc:

```text
Hôm nay thêm việc kiểm tra cẩm nang
Mai 9h gọi khách hàng
Hôm nay kiểm tra logo, 16h gọi khách, ngày mai gửi báo cáo
Chiều mai kiểm tra bố cục
```

Thiếu ngày dùng hôm nay theo timezone app; thiếu giờ là cả ngày. “Chiều mai” không tự thành 14:00. Ngày đang xem chỉ dùng khi nói rõ. Batch mơ hồ/past/duplicate cần bổ sung hoặc preview; không tự lưu một phần. Chat sửa/xóa/hoàn thành cần xác nhận. Request ID chống nhân đôi khi retry, undo từ chối xóa mù task đã sửa về sau.

Parser offline là bộ quy tắc, không phải mô hình AI. Chia mục tiêu/suy luận chưa có khóa/mạng sẽ trả **Cần kết nối AI**, giữ nội dung để thử lại. Thảo luận/viết nháp không tự thành task hoặc done.

Vào Cài đặt → Cấu hình AI thật và chọn một trong hai provider:

- **Codex trên máy:** cần Codex đã cài; bấm **Đăng nhập Codex**, hoàn tất luồng ChatGPT trong trình duyệt, bật đồng ý gửi dữ liệu và AI online, lưu cấu hình rồi kiểm tra kết nối. Có thể để trống model để dùng mặc định của Codex. Task Note AI chạy phiên tạm trong thư mục trống, tắt shell/apps/plugins/browser/computer-use/multi-agent, yêu cầu JSON theo schema và không trao quyền ghi task cho Codex.
- **OpenAI REST:** nhập model ID được tài khoản API hỗ trợ và API key, lưu khóa, bật đồng ý gửi dữ liệu và AI online rồi kiểm tra kết nối. API có thể tính phí. Khóa nằm trong Windows Credential Manager; không ở SQLite/localStorage/log/backup.

Hai provider đều chỉ tạo kết quả đề xuất. Mọi task mutation vẫn đi qua validate/preview/xác nhận và SQLite backend chung. Phiên triển khai chưa đăng nhập Codex và không có API key nên kết nối mô hình thật vẫn cần người dùng hoàn tất sau khi cài.

Mục tiêu lưu phạm vi/ràng buộc, kết quả mong muốn, ngày bắt đầu/hạn cuối, ngày cho phép, quyền tự thêm và trạng thái. AI chia 3–5 bước, đề xuất chưa tăng số task; chọn Thêm vào lịch mới lưu. Bước phụ thuộc chờ bước trước xong; bước bị loại/task đã xóa không tự tái tạo. Sửa được thời lượng ước tính. **Chưa có chế độ tự xếp giờ làm vào calendar.**

AI tự lập việc mặc định tắt, bật riêng ở Trợ lý/Cài đặt cộng quyền từng mục tiêu. Mặc định tối đa 3 việc chủ động/ngày toàn bộ mục tiêu, cấu hình được; backend lập trong ngày làm việc và 08:00–18:00. Lập lại kế hoạch chỉ tạo đề xuất, không xóa/nhân đôi lịch đã lưu. Không tự sửa file, điều khiển máy tính, gọi khách hoặc gửi email/Zalo.

## Backup và rollback

Ưu tiên **Sao lưu đầy đủ SQLite** hoặc CLI `backup create`/`backup verify`: giữ settings, mục tiêu, provenance, draft và lịch sử. JSON V1 chỉ dành tương thích task/ghi chú/lịch lặp; nhập thêm ID chưa có, không ghi đè; không phải backup đầy đủ V2.

Không restore snapshot cũ đè task phát sinh sau nâng. Nếu đã ghi mới, backup DB V2 rồi làm migration ngược/merge có đối soát; không mở V1 trực tiếp trên schema V2. Chi tiết trong DATA_PRESERVATION_REPORT.md.

## Build / test

```powershell
. .\scripts\environment.ps1
pnpm install --frozen-lockfile
pnpm typecheck
pnpm lint
pnpm test
cargo test --locked --manifest-path src-tauri/Cargo.toml -p task-note-core
.\scripts\build.ps1
```

Cần Rust MSVC ≥1.94, C++ Build Tools/Windows SDK, Node/pnpm và WebView2. Script ưu tiên toolchain `.tools`. Build CLI trước test/build desktop vì installer đóng gói resource `tasknote.exe`. Python chỉ dùng cho bảo toàn/test, app không cần Python.

`scripts/cli_smoke.py` dùng profile riêng. Desktop test: `--test-profile <path> --test-headless --test-exit-after=30`, identifier TEST, không toast/autostart. Không chạy smoke lên DB thật. Tài liệu V1 tham khảo nằm ở docs/README_V1.md.
