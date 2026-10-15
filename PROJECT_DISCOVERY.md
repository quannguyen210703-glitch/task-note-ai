# Khảo sát dự án — 14/10/2026

- Source: `C:\Users\HP\Documents\ChatGPT\thông báo task` (Tauri v2 / Rust / React / SQLx).
- Installed: `C:\Users\HP\AppData\Local\Task Note AI\task-note-ai.exe`, file/product version 1.0.0; process 7044 tại thời điểm khảo sát.
- Identifier giữ nguyên: `vn.tasknoteai.desktop`.
- Data thực: `C:\Users\HP\AppData\Roaming\vn.tasknoteai.desktop\task-note.sqlite`; WAL đang hoạt động. Không dùng DB fixture thay thế.
- Git chưa có commit; toàn bộ source đang untracked. Đã lưu source/lockfile trước thay đổi tại `artifacts/preservation/source-v1.zip`; giữ binary cũ trong cùng thư mục.
- Checkpoint online backup: `artifacts/preservation/initial-v1.sqlite`, manifest kèm theo; integrity/foreign keys đạt, 32 task, 2 template. Không ghi nội dung cá nhân trong báo cáo.
- MSVC, SDK, Rust và pnpm cục bộ đã có trong `scripts/environment.ps1`.
- V1 không có flush ô nhập chưa lưu. Chưa được đóng app/cutover trước khi biết người dùng đã lưu nội dung đang gõ.
- Không có API key được cung cấp; kiểm tra provider thực cần người dùng cấu hình trong Windows Credential Manager.

## Luồng lỗi qua khảo sát source

- Xóa/hoãn chỉ có trong modal chi tiết, xóa thêm một lớp confirm; Note thiếu thao tác trực tiếp.
- Không tồn tại IPC/repository/UI bỏ hoãn.
- `effective_due()` dùng snooze thay cho deadline: quá hạn bị diễn giải lại, badge không xét hết hạn, summary có thể nhắc task đang snooze.
- `archive(series)` xóa cả lịch sử cũ; update tự clear snooze cả khi chỉ sửa tiêu đề.
- Cần kiểm thử backend, IPC và UI riêng; khảo sát source chưa chứng minh click bản cài đã được tái hiện.
