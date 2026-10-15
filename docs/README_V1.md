# Task Note AI V1.0 — Windows 10/11 x64

Ứng dụng desktop Tauri v2 + Rust + React/TypeScript; SQLite qua SQLx. Trợ lý Task hiểu lệnh tiếng Việt bằng parser offline, không phải mô hình LLM. Không cần Codex, Internet hay tài khoản khi sử dụng.

## Cài và sử dụng

1. Chạy bộ cài `Task Note AI_1.0.0_x64-setup.exe` trong `artifacts` khi bộ cài đã được build. Cài theo tài khoản Windows hiện tại. Bộ cài chưa ký số có thể được SmartScreen cảnh báo; chỉ sử dụng bản từ nguồn bạn tin cậy. Xem `TEST_REPORT.md` để biết trạng thái build/nghiệm thu thực tế.
2. Lần đầu, đọc onboarding và chọn có cho phép tự khởi động sau khi đăng nhập Windows hay không. Bấm **Gửi thông báo thử**, kiểm tra toast/Trung tâm thông báo rồi **Bắt đầu sử dụng**. Windows có thể tải WebView2 nếu máy chưa có.
3. Sticky Note hiện ở góc phải vùng làm việc. Kéo phần tên/ngày để di chuyển. Nút ghim bật/tắt luôn ở trên; mũi tên thu gọn/mở lại; dấu trừ ẩn vào System Tray; mũi tên chéo mở Dashboard.
4. **Thêm việc**: nhập tiêu đề, ghi chú, ngày giờ tùy chọn, ưu tiên, trạng thái, tag và lịch lặp. Bấm **Xác nhận lưu**. Không đặt hạn thì không có nhắc trước/đúng hạn; việc cả ngày không bị gán nhắc theo giờ.
5. Checkbox hoàn thành/bỏ hoàn thành; bấm tên để sửa, đánh dấu Đang làm, hoãn 15/30/60 phút hoặc tới giờ khác. Quá hạn giữ nguyên ngày hẹn. Xóa có xác nhận và vào thùng rác. Có thể khôi phục việc đơn/kỳ của chuỗi đang hoạt động; chuỗi đã dừng không tự bật lại khi khôi phục riêng một kỳ.
6. Lịch lặp **Hằng ngày**, **T2–T6**, **Mỗi thứ…**: chọn ngày bắt đầu khớp quy tắc. Từng kỳ hoàn thành riêng. Khi sửa chọn **Chỉ lần này** hoặc **Lần này và các kỳ chưa xong sau đó**. Tắt lịch lặp cả chuỗi dừng kỳ tương lai, giữ lịch sử hoàn thành. Sinh trước 14 ngày, không nạp demo tự động.

## Dashboard và Trợ lý Task

Dashboard có Hôm nay, Tất cả, 7 ngày tới, Đã hoàn thành, Trợ lý, Cài đặt và Thùng rác. Tìm tên/ghi chú; lọc ngày hẹn, ưu tiên, trạng thái, tag. Các cửa sổ đồng bộ sau khi DB lưu xong; nếu dữ liệu bị sửa ở cửa sổ khác, mở lại chi tiết trước khi lưu.

Các lệnh ví dụ:

- `Hôm nay anh có việc gì?`
- `Thêm việc kiểm tra báo cáo lúc 14h hôm nay`
- `Mai 9h gọi khách hàng`
- `Dời việc kiểm tra báo cáo sang 16h`
- `Đã xong báo cáo`
- `Nhắc anh họp nhóm mỗi thứ Hai lúc 9h`
- `Còn bao nhiêu việc chưa làm?`
- `Xóa việc báo cáo`

Lệnh thay đổi hiển thị đề xuất, chưa ghi công việc. Bấm **Xác nhận** đúng thẻ hoặc **Hủy**. Khi có nhiều tên phù hợp, chọn theo tên và thời hạn; không tự chọn thay bạn. Đề xuất hết hạn sau 10 phút hoặc khi task đã đổi. Với ngày mơ hồ, ghi `dd/mm/yyyy` và giờ 24h rõ ràng. Chat không thực thi shell hoặc điều khiển Windows. Lưu hội thoại là tùy chọn OFF; bật ở Cài đặt; có nút xóa lịch sử/đề xuất.

## Nhắc việc và khay hệ thống

- Mặc định **08:00–18:00** theo Asia/Ho_Chi_Minh: 08:00 bắt đầu ngày; 09:00–17:00 mỗi đầu giờ một bản tổng hợp; 18:00 tổng kết nếu ngày đó có công việc.
- Trước hạn 15 phút (đổi được 1–120 phút) và đúng hạn, chỉ trong khung làm việc. Nhiều nhắc cùng tick được gộp. Việc hoàn thành/hủy/hoãn không nhận các nhắc ở hạn cũ.
- Khi đăng nhập muộn hoặc thức dậy: tối đa một bản tổng hợp trong giờ hoạt động, không phát bù mọi slot. SQLite lưu khóa chống trùng qua lần mở lại. Claim trước gửi theo nguyên tắc at-most-once: nếu Windows trả lỗi, ghi `failed`, không tự gửi lại gây spam.
- Tắt/ẩn cửa sổ vẫn chạy scheduler trong Rust. Bấm biểu tượng khay để mở Note; menu có Dashboard, thêm nhanh, tạm tắt/bật lại, Cài đặt và **Thoát Task Note AI**. Chỉ Thoát dừng engine.
- Windows quản lý Focus Assist / Do Not Disturb và quyền thông báo. Cài đặt đọc `ToastNotifier.Setting` thực; không hứa hiển thị toast khi hệ điều hành chặn. Nút thử chỉ gửi theo yêu cầu trực tiếp của bạn, kể cả ngoài giờ làm việc. Thông báo lịch tuân thủ khung giờ.

## Cài đặt, sao lưu và chuyển máy

Cài đặt cho phép tự chạy/hiện Note khi đăng nhập, ghim Note, bật/tắt từng loại nhắc, tạm tắt, âm thanh, timezone IANA hoặc theo hệ thống, sáng/tối và khôi phục vị trí. Lưu cài đặt mới có hiệu lực. Giờ task lưu UTC; đổi timezone thay đổi cách hiển thị, không sửa deadline UTC. Template lặp giữ timezone lúc tạo.

**Xuất JSON** mở hộp thoại chọn vị trí lưu; sao lưu gồm task, ghi chú, lịch lặp và lịch sử hoàn thành/thùng rác, phiên bản 1. **Nhập** chọn JSON → xem số việc mới/trùng → xác nhận. Không overwrite mục hiện có; task trùng ID hoặc cùng template/ngày được bỏ qua. Backup tối đa 20 MB. JSON không bao gồm cài đặt, hội thoại hay nhật ký thông báo.

DB: `%APPDATA%\vn.tasknoteai.desktop\task-note.sqlite` (đường dẫn chính xác hiện ở Cài đặt). Khi muốn sao chép DB trực tiếp, **Thoát ứng dụng trước**, copy cả thư mục gồm SQLite/WAL/SHM nếu có. Ưu tiên xuất JSON để chuyển máy. Không xóa DB khi lỗi migration.

Gỡ ứng dụng từ Windows Settings → Apps. Tắt tự khởi động ở Cài đặt trước khi gỡ; sao lưu trước. Dữ liệu trong AppData có thể còn lại; chỉ xóa thủ công sau khi chắc chắn không cần.

## Xử lý lỗi

- Không thấy Note: bấm khay → Mở Note; vào Cài đặt → Đưa Note về góc màn hình.
- Không thấy toast: kiểm tra trạng thái quyền, Windows Notifications & actions, Focus Assist và Trung tâm thông báo; thử từ bản đã cài. Tắt hoàn toàn ứng dụng cũng ngừng nhắc.
- Không lưu được task: đọc lỗi; kiểm tra ổ đĩa, quyền AppData và mở lại chi tiết khi có xung đột. Không báo lưu thành công trước khi SQL transaction commit.
- WebView2 thiếu: bộ cài tải runtime chính thức; máy dùng lần đầu có thể cần Internet. Khi runtime đã có, app hoạt động offline.
- SQLite lỗi/hỏng: giữ nguyên các tệp, sao lưu thư mục dữ liệu và dùng bản backup JSON trên DB mới. Không tự ghi đè hoặc nạp demo.

## Phát triển và đóng gói

Yêu cầu Node.js LTS, pnpm 11, Rust stable MSVC ≥1.90, Microsoft C++ Build Tools (MSVC x64 + Windows SDK), WebView2. Công cụ `.tools` chỉ dùng trên máy triển khai và bị Git bỏ qua; không cần để chạy bản cài.

```powershell
pnpm install --frozen-lockfile
. .\scripts\environment.ps1
pnpm tauri dev
pnpm typecheck
pnpm lint
pnpm test
cargo test --manifest-path src-tauri/Cargo.toml --workspace
.\scripts\build.ps1
```

`build.ps1` dừng nếu test/build lỗi, tạo NSIS và chép bộ cài vào `artifacts`, in SHA256. Mã nguồn cấu trúc trong `docs/ARCHITECTURE.md`. Kết quả thật trong `TEST_REPORT.md` và tiến độ trong `IMPLEMENTATION_LOG.md`.

Nạp demo riêng: `cargo run --manifest-path src-tauri/task-core/Cargo.toml --example demo` xuất `artifacts/demo.json`; nhập qua Cài đặt nếu muốn. Bản cài mặc định có DB trống.

API Tauri đã kiểm tra theo thư viện cài trong Cargo.lock/pnpm-lock.yaml; tham khảo [Windows installer](https://v2.tauri.app/distribute/windows-installer/), [Notifications](https://v2.tauri.app/plugin/notification/), [ToastNotifier.Setting](https://learn.microsoft.com/en-us/uwp/api/windows.ui.notifications.toastnotifier.setting).

## Chẩn đoán bản cài

Có thể chạy `task-note-ai.exe --settings`, `--dashboard`, `--assistant` hoặc `--add` để mở đúng khu vực. `--test-notification` yêu cầu một toast thử và ghi `notification-test.json` trong thư mục dữ liệu (kết quả gửi và số lượng history Windows; không ghi nội dung task). `engine-status.json` ghi trạng thái engine gần nhất. WinRT có thể trả `NotInitialized` trước lần gửi đầu; Windows kiểm soát việc hiển thị khi gửi. Autostart chỉ ghi HKCU cho tài khoản hiện tại, với đường dẫn được quote đúng.
