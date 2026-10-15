# BỘ KIỂM THỬ VÀ NGHIỆM THU TASK NOTE AI V1.0

Codex phải tạo automated unit/integration tests cho business logic và chạy manual test trên bản Windows installer ở những bước khả thi. Bảng phải được chuyển vào `TEST_REPORT.md` với cột Test ID, thực hiện, kết quả PASS/FAIL/BLOCKED, bằng chứng, ghi chú.

| ID | Tình huống | Kết quả bắt buộc |
|---|---|---|
| T01 | Cài bản setup.exe và mở app | Sticky Note thật xuất hiện, UI tiếng Việt không tràn |
| T02 | Tạo task, tắt và mở lại | Dữ liệu còn, không có task bị nhân đôi |
| T03 | Bật tự chạy, đăng xuất/đăng nhập Windows | Sticky Note tự mở sau đăng nhập, một tray icon |
| T04 | Đóng cửa sổ bằng X | Cửa sổ ẩn, app chạy tray, scheduler không dừng |
| T05 | Chọn Thoát từ tray | App thoát hoàn toàn, không còn scheduler |
| T06 | Mở hai lần liên tiếp | Chỉ một instance, không hai engine thông báo |
| T07 | Đồng hồ giả lập 07:59, 08:00, 08:01 | Chỉ 08:00 có một bản mở đầu ngày nếu có việc |
| T08 | Đồng hồ giả lập 09:00–17:00 | Mỗi slot có tối đa một tổng hợp khi có việc |
| T09 | Đồng hồ giả lập 18:00 và 18:01 | 18:00 là tổng kết cuối; 18:01 không có thông báo mới |
| T10 | Task hẹn 10:30 | Có pre-alert 10:15 và due-alert 10:30 nếu chưa hoàn thành |
| T11 | Hoàn thành task 10:30 lúc 10:10 | Không gửi các nhắc 10:15/10:30 |
| T12 | Hoãn task sang 11:30 | Không nhắc tại giờ hẹn cũ; nhắc theo giờ hiệu lực mới |
| T13 | Hoãn sang 19:00 | Không gửi toast ngoài khung 08:00–18:00 |
| T14 | Sleep từ 09:30 tới 12:20 | Không phát bù các slot cũ; reconcile một lần |
| T15 | Restart app hai lần trong cùng slot | Không nhắc lặp do có notification_log |
| T16 | Chuyển qua ngày mới với việc chưa xong | Việc hiện ở Quá hạn, không tự động sửa deadline |
| T17 | Task lặp mỗi thứ Hai 09:00 | Mỗi tuần một occurrence; hoàn thành kỳ này không ảnh hưởng kỳ sau |
| T18 | Tắt notification hoặc Windows từ chối quyền | App vẫn chạy; chỉ rõ lý do không gửi toast |
| T19 | Chat: 'Mai 9h gọi khách hàng' | Có bản preview chính xác; chưa ghi DB trước xác nhận |
| T20 | Chat: 'Đã xong báo cáo' với hai task tên gần giống | Hỏi chọn task, không hoàn thành nhầm |
| T21 | Click xác nhận chat | DB cập nhật, Sticky Note và Dashboard đổi đồng bộ |
| T22 | Xuất backup rồi nhập trên DB mới | Task khôi phục, schema được validate, không mất dữ liệu |
| T23 | Rút màn hình ngoài / Windows scale 150% | Note vẫn nhìn thấy và có thể thao tác |
| T24 | Chạy trong thời gian dài với dashboard ẩn | Scheduler backend vẫn nhận mốc, không phụ thuộc JS setInterval của UI |
| T25 | Gửi thông báo thử từ bản app đã cài | Windows toast hoạt động với quyền đã được cấp; ghi rõ nếu bị Focus Assist chặn |
| T26 | Không có task hôm nay | Empty state sạch, không spam notification đầu giờ |
| T27 | Chỉnh giờ hệ thống hoặc múi giờ | Không phát lặp/mất dữ liệu, hiển thị deadline đúng timezone cấu hình |
| T28 | Lưu task chứa dấu tiếng Việt và ký tự đặc biệt | Lưu/đọc lại đúng, không SQL injection hoặc vỡ layout |

## Tối thiểu automation và kỹ thuật

- `typecheck`, `lint`, `unit tests` và `cargo test` phải chạy được.
- Scheduler test với clock có thể inject/fake; không chờ giờ thật.
- Integration test dữ liệu SQLite: CRUD, migration, unique index, recurrence, backup/restore.
- Test logic frontend: quick add, xác nhận chat, lọc task, trạng thái rỗng và lỗi.
- Xác thực thủ công native toast trên **ứng dụng đã cài**; không chỉ dùng browser/dev server. Nếu chưa có máy Windows test, đánh dấu BLOCKED thay vì PASS.
- Build NSIS `.exe` thật; phải cung cấp đường dẫn thực tế và chứng cứ. Không được thay bằng bản zip HTML/web.

## Tiêu chí chốt nghiệm thu

Tất cả chức năng core chạy độc lập offline, giao diện tương tác thật, dữ liệu bền vững, autostart+tray đáng tin cậy, thông báo được dedupe và không hoạt động ngoài khung, chat offline dùng được, bản build có thể cài, và README đủ để người không chuyên tự sử dụng. Mọi lỗi cản nghiệm thu phải được xử lý trước khi đánh dấu hoàn tất.
