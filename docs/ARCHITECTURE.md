# Kiến trúc Task Note AI V1

- `src/`: hai giao diện React. Query parameter chọn Sticky Note hoặc Dashboard; Tauri tạo hai WebView riêng. Không có dữ liệu mẫu/DB trong trình duyệt.
- `src-tauri/src/`: lifecycle, IPC có kiểm tra dữ liệu, System Tray, single instance, window state, tự khởi động sau onboarding; một async scheduler Rust tick 20 giây.
- `src-tauri/task-core/`: business logic độc lập UI, SQLx SQLite và migration phiên bản. UTC tính theo giây Unix; timezone mặc định Asia/Ho_Chi_Minh.
- `repository`: prepared queries, transaction, optimistic concurrency qua `updated_at_utc`, import thêm mới không overwrite. SQLite WAL, foreign keys, busy timeout. Không ghi đè DB khi migration/open thất bại.
- Chuỗi lặp lưu template và các occurrence độc lập, UNIQUE(template_id, occurrence_date). Sinh trước 14 ngày, tối đa 31 ngày thiếu phía sau; không tạo lịch vô hạn. Lần đã hoàn thành được giữ. Sửa cả chuỗi áp dụng kỳ hiện tại và kỳ chưa xong phía sau; thay đổi timezone app không tự chuyển giờ của template hiện hữu.
- `scheduler`: clock inject được. Một slot/đầu giờ, 18:00 tổng kết. Check gap >90s hoặc đồng hồ lùi → một reconcile thay vì replay. Gộp tối đa 5 mục, 480 ký tự. Claim các khóa dedupe trong transaction trước khi gửi; một lần gửi lỗi được ghi `failed`, không tự retry gây spam.
- `notifications.rs`: WinRT `ToastNotifier.Setting` kiểm tra quyền Windows thực và `Show` đồng bộ; XML được escape; tắt âm thanh bằng `<audio silent="true"/>`. Windows quản lý Focus Assist/DND. Không bypass quyền. Không bật nút hành động/toast callback chưa được kiểm nghiệm.
- `assistant`: parser tiếng Việt offline, truy vấn SQLite; đề xuất lưu riêng có token 10 phút. Xác nhận transaction kiểm tra version của task, tiêu thụ token; thao tác lỗi không hiện thành công. Lệnh mơ hồ không mutate. Không thực thi shell, truy cập file hoặc gọi mạng.
- Backup JSON phiên bản 1 chứa tasks/templates, không chứa API key. Import kiểm tra schema trước, preview số việc thêm/trùng, rồi mới xác nhận. UI file picker chỉ đọc tệp người dùng chọn.

Quyền WebView chỉ cho phép events/IPC cơ bản và kéo cửa sổ. CSP không cho remote script hoặc frame. Không shell plugin, không telemetry hay cloud adapter hoạt động.
