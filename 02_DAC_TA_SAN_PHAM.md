# ĐẶC TẢ SẢN PHẨM — TASK NOTE AI (Windows)

Phiên bản phạm vi: V1.0. Đối tượng sử dụng: một người dùng trên laptop Windows 10/11. Offline-first. Ngôn ngữ: tiếng Việt. Khung thông báo mặc định: 08:00–18:00, bao gồm 18:00.

## 1. Tuyên bố trải nghiệm

Mở laptop -> đăng nhập Windows -> Sticky Note của Task Note AI tự xuất hiện ở góc phải trên vùng màn hình dùng được. Note hiển thị task hôm nay, việc quá hạn và sắp tới; không chiếm hết màn hình. Khi cần sửa việc hoặc trò chuyện với Trợ lý Task, bấm `Mở rộng` để mở Dashboard đầy đủ. Mỗi đầu giờ trong khung làm việc, nếu còn việc liên quan, Windows có một thông báo tổng hợp gọn. Đóng Note bằng X vẫn nhắc vì app chạy ở khay hệ thống; chỉ lệnh `Thoát` mới dừng hẳn.

**Giới hạn nền tảng:** autostart xảy ra sau khi người dùng đăng nhập Windows, không phải trước màn hình login; khi laptop tắt/ngủ, ứng dụng không thể hiển thị thông báo mới; Windows Focus Assist/Do Not Disturb và quyền thông báo có thể ngăn toast.

## 2. Wireframe Sticky Note

```text
+------------------------------------------------+
| [check] TASK NOTE AI            [pin] [-] [>]  |
| Thứ Tư, 14/10/2026                             |
|------------------------------------------------|
| HÔM NAY                       2/5 hoàn thành    |
| [==========.............]                      |
|------------------------------------------------|
| CẦN XỬ LÝ NGAY                                 |
| [ ] [!] Kiểm tra báo cáo           QUÁ HẠN      |
|------------------------------------------------|
| SẮP ĐẾN                                        |
| [ ] 10:30  Kiểm tra tin nhắn khách hàng         |
| [ ] 14:00  Họp triển khai dự án                 |
| [ ] 16:00  Chốt danh sách công việc             |
|------------------------------------------------|
| [+ Thêm việc]           [Mở trợ lý Task AI]    |
+------------------------------------------------+
```

Thiết kế: rộng 340–380px, cao khoảng 500–560px, nội dung cuộn; nền sáng #F8FAFC, chữ #0F172A, accent #0D9488, nguy cấp màu cam/đỏ kèm chữ; font Segoe UI; tối ưu Windows DPI 125–200%; cho phép chuyển dark mode. Có click nút ghim on-top và lưu trạng thái, drag để đổi vị trí. Nhớ vị trí theo monitor, tự đưa về vùng nhìn thấy khi đổi monitor.

## 3. Wireframe Dashboard và chat

```text
+--------------------------------------------------------------------------+
| TASK NOTE AI          Tìm việc...                           [Cài đặt]    |
|--------------------------------------------------------------------------|
| [Hôm nay]    |  THỨ TƯ 14/10      Hoàn thành 2/5                     |
| [Tất cả]    |------------------------------------------------------|
| [Sắp tới]   |  [ ] 10:30 Tin nhắn khách hàng       [Ưu tiên cao]    |
| [Đã xong]   |  [ ] 14:00 Họp triển khai             [Bình thường]   |
| [Trợ lý]    |  [ ] 16:00 Chốt danh sách             [Bình thường]   |
| [Cài đặt]   |------------------------------------------------------|
|             |  TRỢ LÝ TASK: "Mai 9h gọi khách"                      |
|             |  -> Đề xuất: Gọi khách | ngày mai 09:00             |
|             |  [Xác nhận] [Sửa] [Hủy]                            |
+--------------------------------------------------------------------------+
```

V1 có phần chat thực thi các lệnh task bằng offline parser, không phải một chatbot LLM ảo. Truy vấn phải đọc DB thật; khi thay đổi task cần hiện thẻ xác nhận. Không cần mở Codex sau khi đã cài phần mềm. Thanh bên có thể dùng tab Assistant riêng nếu kích thước cửa sổ hẹp.

## 4. Thời điểm và nội dung nhắc

| Thời điểm | Loại nhắc | Nội dung | Khi nào không gửi |
|---|---|---|---|
| 08:00 | Mở đầu ngày | Tổng task hôm nay, quá hạn, top 3 ưu tiên | Không có việc liên quan hoặc người dùng tắt |
| 09:00–17:00, đúng mỗi đầu giờ | Tổng hợp | Việc đang tồn và sắp tới; tối đa 1 toast/slot | Không có việc liên quan hoặc slot đã gửi |
| 18:00 | Tổng kết | Số hoàn thành/chưa xong, nhiệm vụ quá hạn | Không có task nào hôm nay hoặc người dùng tắt |
| Hạn trừ 15 phút | Trước hạn | Tên việc, giờ hẹn, ưu tiên | Ngoài khung; đã hoàn thành; đã hoãn |
| Đúng hạn | Đến hạn | Tên việc, hành động mở task | Ngoài khung; đã hoàn thành; đã hoãn |

Nếu nhiều task cùng mốc, gộp. Thông báo hệ thống không cố buộc hiển thị trên mọi chế độ Windows; khi bị chặn, trạng thái trong app cần nói rõ.

## 5. Quy tắc công việc

- Trạng thái `Cần làm` / `Đang làm` / `Hoàn thành`. `Quá hạn` là thuộc tính tính toán, không phải lệnh tự đổi trạng thái. Hoàn thành ghi nhận thời gian thực và ngừng các nhắc sau đó.
- Ưu tiên `Cao` / `Bình thường` / `Thấp`; sorting: quá hạn, mức ưu tiên, thời hạn gần nhất, việc không hẹn giờ.
- Ngày hẹn trống: luôn có thể tạo và hoàn thành; không nhắc theo hạn. Công việc cả ngày: hiển thị ở danh sách; không bịa ra giờ nhắc riêng.
- Xóa phải xác nhận hoặc đi qua thùng rác, tránh mất dữ liệu.
- Lịch lặp có mỗi occurrence độc lập; một việc đã hoàn thành hôm nay không tự động hoàn thành lần tuần sau.
- Việc còn dở cuối ngày giữ nguyên hạn thực tế, xuất hiện `Quá hạn` hôm sau; không tự dời ngày hoặc tự xóa.

## 6. Trải nghiệm ở tình huống đặc biệt

- Đăng nhập lúc 10:06: note mở và hiển thị việc hôm nay; không phát bù cả 08:00–10:00; slot kế tiếp 11:00 như lịch.
- Ngủ từ 09:30 đến 12:20: sau wake, đồng bộ DB/thời gian và hiển thị tóm tắt một lần, không xếp hàng ba thông báo đã lỡ.
- Đóng Note bằng X: ẩn UI vào tray, scheduler vẫn chạy; click icon tray hiện Note lại.
- Quit bằng tray: thật sự tắt app; không nhắc nữa tới lần người dùng mở lại hoặc đăng nhập lần tiếp theo (nếu autostart bật).
- Hoàn thành lúc 10:10 cho task 10:30: không còn pre-alert 10:15 và deadline alert 10:30.
- Hoãn qua 18:00: lưu giờ hoãn, không nhắc ngoài giờ; đưa task vào bản tóm tắt lúc hoạt động trở lại.
- Đổi độ phân giải/rút màn hình ngoài: Note được đưa vào màn hình khả dụng, không biến mất ngoài vùng xem.
- Hai lần mở app: chỉ một tiến trình, chỉ một scheduler/tray, dữ liệu vẫn nguyên.
- Chuyển ngày: hôm qua chưa xong không bị xóa; ngày mới chỉ sinh đúng các occurrence lặp cần thiết.

## 7. Màn hình Cài đặt tối thiểu

| Trường | Mặc định |
|---|---|
| Tự chạy khi đăng nhập Windows | ON sau xác nhận lần đầu |
| Hiện Sticky Note khi đăng nhập | ON |
| Ghim Note luôn ở trên | ON, có nút tắt nhanh |
| Nhắc đầu giờ | ON, 08:00–18:00 |
| Nhắc trước hạn | ON, trước 15 phút |
| Nhắc đúng hạn | ON |
| Tổng kết 18:00 | ON |
| Tạm tắt thông báo | OFF |
| Múi giờ | Asia/Ho_Chi_Minh; đổi được |
| Ngôn ngữ / đồng hồ | Tiếng Việt / 24h |
| Dữ liệu & riêng tư | Offline, xuất/nhập JSON |

Hiển thị trạng thái notification OS, service đang chạy, last scheduler check, kế hoạch nhắc sắp tới; có nút gửi thử thông báo. Không lưu trữ API key dạng plaintext.

## 8. Phạm vi không triển khai trước nghiệm thu V1

Không cần web dashboard công khai, website đăng nhập, phân quyền nhóm, kết nối Gmail/Meta/Facebook, Google Calendar, sync di động, báo cáo marketing, biểu đồ nâng cao hoặc LLM trả phí. Kiến trúc nên cho phép mở rộng những tính năng này sau khi V1 ổn định.
