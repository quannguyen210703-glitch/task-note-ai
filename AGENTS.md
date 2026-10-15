# AGENTS.md — HƯỚNG DẪN CODEX TRONG REPO TASK NOTE AI

## Objective
Triển khai app desktop Windows Task Note AI theo `01_PROMPT_CHINH_THUC_CHO_CODEX.md`, `02_DAC_TA_SAN_PHAM.md`, `03_KIEM_THU_NGHIEM_THU.md`. Đây là dự án ứng dụng thực; không chấp nhận chỉ tạo mockup.

## Working rules
- Bảo toàn dữ liệu và code hiện hữu; kiểm tra repo/git trước thay đổi.
- Stack mặc định Tauri v2 + Rust + React + TypeScript + SQLite (`sqlx` trong backend).
- Tác vụ hẹn giờ và DB source-of-truth ở Rust, không phụ thuộc React window timer.
- Mọi task CRUD, chat mutation và notification phải dùng cùng nguồn dữ liệu.
- Không tạo API key giả, không hardcode secret, không bật cloud/telemetry không có đồng ý.
- Không tạo thông báo ngoài 08:00–18:00 theo mặc định; 18:00 là slot tổng kết cuối.
- Kiểm thử unique dedupe và sleep/wake bằng fake clock.
- Không đưa chức năng giả vào UI; bất kỳ nút nào hiển thị phải hoạt động hoặc bị disabled với lý do rõ ràng.
- Nếu dùng plugin/APIs Tauri, xác minh đúng API Tauri v2 theo phiên bản thực cài, cấp permissions tối thiểu.
- Luôn báo cáo kết quả lệnh/test thật, không suy đoán rằng app đã build hoặc toast đã chạy.

## Required deliverables
- Source code + migrations + tests.
- README tiếng Việt; `TEST_REPORT.md`; `IMPLEMENTATION_LOG.md`.
- Windows setup `.exe` nếu môi trường Windows đủ điều kiện; nếu không thì ghi blocker chính xác.
- Tổng kết final có danh sách chức năng hoàn thành/chưa xong, lệnh test và vị trí artifact thực.
