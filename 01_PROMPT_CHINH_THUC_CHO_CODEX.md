# PROMPT CHÍNH THỨC — XÂY DỰNG TASK NOTE AI CHO WINDOWS

> Dán toàn bộ nội dung tài liệu này vào Codex ở chế độ có quyền làm việc với thư mục dự án. Các tài liệu `02_DAC_TA_SAN_PHAM.md`, `03_KIEM_THU_NGHIEM_THU.md`, `AGENTS.md` đi kèm là tiêu chuẩn bổ sung bắt buộc. Đây là yêu cầu XÂY DỰNG PHẦN MỀM THẬT, không phải yêu cầu tạo ảnh, bản mẫu HTML tĩnh hoặc chỉ viết kế hoạch.

## 0. Vai trò và cách làm việc

Bạn là senior Windows desktop engineer, Tauri/Rust/React engineer, UX designer và QA engineer. Chủ động kiểm tra môi trường, tạo dự án, triển khai từng phần, chạy thử, sửa lỗi, kiểm thử và đóng gói. Chỉ hỏi người dùng khi thiếu thông tin thực sự chặn việc triển khai; dùng mặc định nêu trong tài liệu đối với các lựa chọn khác. Trước khi sửa thư mục đã có code, kiểm tra trạng thái Git và bảo toàn dữ liệu hiện có. Không ghi đè tác phẩm không thuộc dự án.

Yêu cầu quan trọng: không được dừng ở wireframe, pseudocode, mock API, giao diện giả, hoặc một app web chạy trên localhost. Cần có ứng dụng desktop hoạt động độc lập trên Windows 10/11 x64 và bản cài đặt NSIS `.exe` nếu môi trường có thể build Windows. Không tuyên bố đã kiểm thử trên Windows khi chưa thực sự kiểm thử.

## 1. Mục tiêu sản phẩm

Tên ứng dụng: **Task Note AI**. Ngôn ngữ hiển thị mặc định: tiếng Việt. Múi giờ mặc định: `Asia/Ho_Chi_Minh` (UTC+07:00), cho phép theo múi giờ hệ thống hoặc cấu hình trong Cài đặt.

Người dùng cần: đăng nhập Windows -> tự thấy Sticky Note nổi ở góc phải màn hình, hiển thị những việc cần làm hôm nay -> đầu MỖI GIỜ từ 08:00 đến 18:00 ứng dụng nhắc việc chưa hoàn thành -> có thể mở rộng thành bảng quản lý đầy đủ và khung hội thoại Task AI -> thêm, sửa, hoàn thành, hoãn, xem lại công việc mà không cần mở Codex. Khi cửa sổ ẩn đi, engine nhắc việc phải tiếp tục chạy qua system tray; khi thoát hẳn ứng dụng thì ngừng nhắc. Máy tắt hoặc ngủ không thể gửi toast; khi đăng nhập/khôi phục, đối soát để hiển thị tóm tắt một lần thay vì bắn bù nhiều thông báo cũ.

## 2. Công nghệ và kiến trúc được chốt

- Tauri **v2 ổn định**, Rust backend, React + TypeScript + Vite, giao diện CSS/Tailwind (hoặc tương đương), icon Lucide.
- SQLite cục bộ; Rust là tầng dữ liệu chuẩn (source of truth), dùng `sqlx` với migrations có version, transaction, prepared queries và cơ chế chống trùng lặp. Không coi localStorage là database chính.
- Rust backend quản lý application lifecycle, scheduler, repository, thông báo và events. Frontend chỉ hiển thị và gửi lệnh qua Tauri IPC; không phụ thuộc bộ đếm JavaScript trong cửa sổ để nhắc việc.
- Các khả năng tương thích Tauri v2: autostart, notification, single-instance, window-state, tray icon; positioner nếu cần định vị góc màn hình. Dùng API và plugin phiên bản đã kiểm tra thực tế tại thời điểm lập trình, cấu hình permissions tối thiểu (least privilege).
- Đóng gói NSIS `setup.exe` cho Windows 10/11 x64; tạo `.msi` nếu máy build hỗ trợ và không làm chậm nghiệm thu chính. WebView2 được xử lý theo khuyến nghị của Tauri.
- Không yêu cầu máy chủ; toàn bộ quản lý task, hiển thị, nhắc giờ và lệnh chat cơ bản phải dùng được offline. Không tự ý thực hiện telemetry, upload ghi chú hoặc đồng bộ cloud.
- Nếu tính năng LLM trực tuyến có trong phiên bản này, phải là lựa chọn bật thủ công, có sự đồng ý rõ ràng; khóa API lưu bằng kho bí mật hệ điều hành (Windows Credential Manager hoặc giải pháp OS keychain phù hợp), không lưu plaintext trong SQLite, localStorage, log hoặc mã nguồn. Nếu chưa thể bảo vệ khóa đúng chuẩn, hoãn adapter cloud và giữ đầy đủ chat offline.

## 3. Kiến trúc cửa sổ

A. `sticky`: cửa sổ độc lập mặc định ~360 x 520 logical px, resizable với giới hạn hợp lý, bo góc, có nút ghim luôn ở trên (always-on-top, bật mặc định nhưng cho tắt), kéo thả được, mặc định góc phải màn hình làm việc (work area, tránh đè taskbar); nhớ vị trí/kích thước qua các lần mở; khi thay màn hình, độ phân giải, scale DPI phải tự đưa lại vào vùng nhìn thấy. Không chiếm focus không cần thiết và không cố chèn lên ứng dụng toàn màn hình.

B. `dashboard`: cửa sổ quản lý độc lập mặc định ~1000 x 700 logical px, có sidebar, Today, All Tasks, Upcoming/Week, Completed, Assistant, Settings. Khi mở dashboard từ Sticky Note, giữ dữ liệu hai cửa sổ đồng bộ tức thì. Cửa sổ dashboard không tự mở mỗi lần đăng nhập, chỉ sticky mở. Đóng cửa sổ bằng X phải ẩn/đóng UI theo hành vi đã thiết kế nhưng không làm scheduler chết; chỉ `Thoát Task Note AI` từ khay hệ thống mới dừng app.

C. `tray`: menu Mở Note, Mở bảng quản lý, Thêm việc nhanh, Tạm tắt thông báo (đến khi bật lại), Cài đặt, Thoát. Không được sinh tiến trình thứ hai khi double-click icon hoặc app khởi động lại. Khi click toast hỗ trợ thì mở task liên quan; nếu hệ thống không hỗ trợ callback thì mở danh sách từ tray, không làm sai trạng thái.

## 4. Giao diện Sticky Note

Phong cách hiện đại, gọn, dễ đọc cho người dùng Việt: Segoe UI/system font, nền sáng trung tính, chữ xanh đen, accent teal, ưu tiên nhận diện bằng màu + nhãn (không chỉ màu), hỗ trợ dark mode tùy chọn. UI tiếng Việt có dấu đầy đủ, kiểm tra tràn chữ/DPI.

Phần đầu: biểu tượng app; tên `TASK NOTE AI`; ngày và thứ; nút ghim, thu gọn, mở rộng; biểu tượng trạng thái engine. Khối `HÔM NAY`: tổng task, đã hoàn thành, thanh tiến độ; phần `Cần xử lý ngay` gồm việc quá hạn và việc quan trọng; phần `Sắp đến` tối đa ba task; danh sách các task khác có cuộn. Mỗi hàng có checkbox Hoàn thành, tiêu đề, giờ, mức độ ưu tiên; click hàng mở chi tiết. Nút `+ Thêm việc` hoạt động thật. Nút `Mở trợ lý AI` mở tab Assistant trong dashboard. Khi không có task hiển thị empty-state thân thiện chứ không hiện lỗi.

Kích thước tối thiểu phải vẫn sử dụng được, chức năng checkbox và thêm nhanh không bị giấu ngoài vùng nhìn thấy.

## 5. Quản lý công việc

Các trường tối thiểu:
`id`, `title` (bắt buộc), `description`, `due_at_utc` (nullable), `is_all_day`, `priority` (`high|medium|low`), `status` (`todo|doing|done`), `tag_or_project`, `recurrence_rule` (nullable), `snoozed_until_utc` (nullable), `created_at_utc`, `updated_at_utc`, `completed_at_utc` (nullable), `archived_at_utc` (nullable). Lưu lịch sử sự kiện nếu cần. Tách lịch lặp thành template + occurrence hoặc tương đương để mỗi lần lặp có thể hoàn thành độc lập và không sinh trùng sau khi reboot.

CRUD thật: tạo, sửa, đánh dấu đang làm, hoàn thành, đánh dấu chưa hoàn thành, hoãn, lưu ghi chú, ưu tiên, tìm kiếm, lọc theo ngày/trạng thái/ưu tiên/tag, thùng rác hoặc xác nhận trước xóa vĩnh viễn. Có thể tạo việc không đặt giờ; việc này xuất hiện trong danh sách nhưng không có pre-alert. Các việc quá hạn chưa hoàn thành được giữ trong phần `Quá hạn`, **không tự sửa ngày hẹn**. Không xóa task lúc qua ngày. Hỗ trợ lịch lặp hàng ngày, ngày làm việc (thứ Hai đến thứ Sáu) và hàng tuần; chỉnh riêng một occurrence hoặc cả chuỗi phải minh bạch, không sinh hàng loạt occurrence vô hạn.

Cùng một task được phản ánh đồng thời trên sticky, dashboard, chat và scheduler. Mọi thao tác mutate phải ghi DB thành công trước khi cập nhật UI là hoàn tất; xử lý lỗi có thông báo rõ và rollback UI nếu cần. Backup xuất JSON định dạng có version; import có bước xem trước, validate dữ liệu và chống trùng, không ghi đè toàn bộ DB thiếu xác nhận. Có ít nhất cách sao lưu/thay máy rõ ràng.

## 6. Quy tắc nhắc giờ CHUẨN

**Khung hoạt động thông báo mặc định là 08:00–18:00 giờ Việt Nam, bao gồm cả mốc 18:00.** Mọi thông báo lịch chỉ bật khi người dùng bật notification và Windows cấp quyền. Cài đặt có toggle riêng cho `nhắc đầu giờ`, `nhắc trước hạn`, `nhắc đúng hạn`, `tổng kết cuối ngày`, `âm thanh` và `Tạm tắt thông báo`.

### Slots đầu giờ
- 08:00: `Bắt đầu ngày mới` — tổng task hôm nay, chưa hoàn thành, quá hạn, tối đa ba task ưu tiên.
- 09:00, 10:00, ..., 17:00: `Nhắc việc đầu giờ` — chỉ một thông báo TỔNG HỢP cho tất cả task có liên quan, không phải một toast cho mỗi task. Không gửi thông báo vô nghĩa nếu không có việc cần xử lý.
- 18:00: `Tổng kết ngày` — đã xong/chưa xong/quá hạn; chỉ hiện toast nếu trong ngày có task, không spam khi danh sách trống. Không có hourly slot sau 18:00.

### Nhắc theo hạn
- Nếu task có giờ hẹn hợp lệ và chưa hoàn thành: nhắc trước hạn 15 phút và đúng hạn, **chỉ khi thời điểm gửi nằm trong 08:00–18:00**; các mốc nằm ngoài giờ yên lặng không được gửi tự động. Nếu task có hạn ngoài khung, hiển thị rõ trong giao diện và đưa vào bản tóm tắt trong khung tiếp theo thích hợp. Mốc nhắc mặc định cấu hình được (ví dụ 15 phút).
- Task hoàn thành trước giờ nhắc => hủy nhắc còn lại. Hoãn 15/30/60 phút hoặc chọn giờ khác => cập nhật thời điểm nhắc hiệu lực, tránh gửi ở mốc cũ. Nếu hoãn qua 18:00 => không báo sau 18:00, chờ khung làm việc tiếp theo. Thao tác `Hoàn thành`/`Hoãn` nằm trong ứng dụng; nút tương tác trực tiếp trong toast là nâng cao, chỉ bật nếu API Windows đã kiểm nghiệm trên bản cài đặt.
- Nếu có nhiều reminder cùng thời điểm: gộp thông báo, ưu tiên sự kiện đến hạn hoặc mức high, giới hạn độ dài.

### Không trùng, không nổ bù
- Lưu `notification_log`/`reminder_dispatches` trong SQLite với unique key vững: ví dụ `(kind, local_date, hour_slot)` cho hourly và `(task_occurrence_id, due_at_utc, reminder_kind)` cho task reminders. Phải tồn tại qua các lần restart, dùng transaction/idempotency.
- Scheduler chạy ở **Rust backend**, tick hợp lý (ví dụ 15–30 giây), kiểm tra wall-clock và reconcile lúc resume/wake. Không phụ thuộc cửa sổ đang mở và không tự tạo service/Task Scheduler của Windows nếu không thật sự cần.
- Nếu máy ngủ/tắt qua nhiều mốc: KHÔNG phát lại 08:00, 09:00, 10:00... liên tục. Khi đăng nhập hoặc máy thức dậy, hiển thị một lần tóm tắt việc còn tồn và tiếp tục lịch tương lai. Mốc quá hạn cũ thể hiện trong UI chứ không spam toast. Khi app mở muộn 10:06, đừng giả là đã gửi 10:00.
- Tôn trọng Windows Do Not Disturb/Focus Assist, quyền thông báo, chế độ toàn màn hình và giới hạn hệ điều hành; nếu toast bị chặn, hiển thị cảnh báo trong phần Cài đặt/Trạng thái, không hứa gửi thông báo bất chấp hệ thống.
- `Tạm tắt thông báo` chỉ dừng toast, không xóa task hoặc tắt tác vụ đồng bộ nội bộ. Kết thúc tạm tắt không gửi bù hàng loạt.

## 7. Chat Task AI: PHẢI CÓ trong bản giao đầu tiên

Bản offline phải có ô chat sử dụng được bằng bộ hiểu lệnh tiếng Việt dựa trên quy tắc + parser thời gian, không yêu cầu API hoặc Internet. Không gọi đây là mô hình AI tự suy luận nếu chỉ là parser; giao diện có thể dùng tên `Trợ lý Task` và ghi rõ chế độ `Offline`. Cần triển khai ít nhất các ý định:

1. `Hôm nay anh có việc gì?` -> truy vấn task hôm nay và quá hạn.
2. `Thêm việc kiểm tra báo cáo lúc 14h hôm nay` -> đề xuất tạo task, nút Xác nhận/Hủy.
3. `Mai 9h gọi khách hàng` -> parse ngày mai, giờ 09:00; hiển thị kết quả có thể xác nhận.
4. `Dời việc kiểm tra báo cáo sang 16h` -> tìm task, nếu trùng tên phải hỏi chọn.
5. `Đã xong báo cáo` -> đề xuất task phù hợp, xác nhận trước khi đánh dấu done.
6. `Nhắc anh họp nhóm mỗi thứ Hai lúc 9h` -> lịch lặp hàng tuần.
7. `Còn bao nhiêu việc chưa làm?` -> truy vấn số liệu thật từ SQLite.

Quy tắc parser: dùng thời gian/múi giờ thực của app; không tự bịa ngày nếu câu mơ hồ (`thứ Sáu`, `tuần sau`, `chiều` không rõ giờ). Hỏi xác nhận thông tin còn thiếu bằng một câu gọn. Không xóa/sửa task khác khi nhận diện không chắc. Không tạo công việc chỉ vì người dùng gõ trong chat; phải preview rồi xác nhận, đặc biệt thao tác xóa/đổi giờ. Lưu hội thoại cục bộ nếu người dùng chọn, cho xóa lịch sử. Không cho chat thực thi lệnh shell, truy cập file ngoài sandbox hoặc tự điều khiển Windows.

Thiết kế adapter `OptionalCloudAIProvider` tách biệt, mặc định OFF; chỉ tích hợp gọi LLM khi người dùng tự cấu hình khóa và bật chia sẻ nội dung. Nếu không có API thì toàn bộ V1 vẫn chạy đầy đủ: CRUD, notification và bảy ý định offline nêu trên.

## 8. Cài đặt và trải nghiệm người dùng

Cài đặt mặc định: tự chạy khi đăng nhập Windows = ON sau khi người dùng xác nhận ở bước onboarding; mở Sticky Note khi đăng nhập = ON; ghim Note nổi = ON (có nút tắt nhanh); chỉ nhắc trong 08:00–18:00; nhắc trước hạn 15 phút; 08:00 báo đầu ngày; mỗi đầu giờ 09:00–17:00; 18:00 tổng kết; ngôn ngữ tiếng Việt; giao diện light với hỗ trợ dark mode.

Onboarding kiểm tra quyền Windows Notifications, giải thích tự khởi động và bật bằng sự đồng ý của người dùng; có nút `Gửi thông báo thử`. Cài đặt gồm khu vực `Trạng thái nhắc việc` cho biết engine running, Windows permission status, last check, next planned slot và trạng thái tắt tiếng. Nếu quyền bị từ chối, app vẫn hoạt động với nhắc trực quan trong giao diện và chỉ cách bật lại trong Windows Settings.

Cho phép đổi vị trí Note, tắt always-on-top, ẩn Note, mở lại từ tray, khôi phục vị trí nếu màn hình thay đổi, định dạng giờ 24h, chữ dễ đọc với Windows scaling 125/150/200%, và không có thao tác xóa hàng loạt không xác nhận.

## 9. Yêu cầu bảo mật, ổn định, hiệu năng

- Offline-first; không phải đăng nhập; không dùng API key nhúng cứng; không truyền task lên server mặc định; không ghi thông tin nhạy cảm vào log.
- DB dùng migrations versioned và có backup/export; không mất dữ liệu khi app bị tắt/khởi động lại. Xử lý lỗi mở SQLite, migration, dữ liệu hỏng bằng báo lỗi và không ghi đè âm thầm.
- Không nhân đôi process/tray/scheduler; tránh busy loop, tránh timer JS bị throttling; sau nhiều giờ treo trên tray vẫn nhắc đúng.
- Kiểm tra input, giới hạn độ dài, escaped text, parameterized DB queries, CSP/capabilities tối thiểu; không bật shell plugin tùy tiện.
- Mọi dữ liệu thao tác lưu timestamp UTC + timezone hiển thị; lưu định danh occurrence và loại reminder để chống gửi trùng. Xử lý thay đổi giờ Windows, chuyển múi giờ, sleep/resume, reboot và nâng cấp phiên bản.

## 10. Kế hoạch THỰC THI bắt buộc

Làm theo các chặng có nghiệm thu rõ, không chỉ nêu ra:

Chặng A — Khởi tạo: kiểm tra OS/Node/Rust/C++ Build Tools/WebView2; scaffold Tauri v2 + React TS; tổ chức thư mục frontend, backend, test, docs. Nếu đã có repo, kiểm tra trước khi sửa. Chạy được cửa sổ shell thật.

Chặng B — Persistence: thiết kế SQL schema/migrations, repository CRUD, lịch lặp/occurrence, import/export, unit tests. Thêm task rồi tắt/mở app phải còn.

Chặng C — Desktop UX: sticky, dashboard, tray, always-on-top, window state, single-instance, autostart, UI tiếng Việt, các trạng thái rỗng/lỗi/đang tải. Xử lý đóng X không tắt scheduler.

Chặng D — Notification engine: scheduler chạy từ Rust, slots 08:00–18:00, nhắc đúng hạn/trước hạn, snooze, dedupe, wake/restart, permissions, test toast thực trên bản cài Windows.

Chặng E — Task chat offline: parser tiếng Việt cho 7 ý định và preview xác nhận; unit tests; đảm bảo CRUD qua cùng repository.

Chặng F — QA và đóng gói: chạy lint/typecheck/unit/integration; build release; tạo NSIS installer `.exe`; cài lên máy Windows test và kiểm tra autostart, tray, notification, restart, sleep/wake nếu môi trường hỗ trợ. Ghi chính xác những điều đã/chưa được test. Sửa lỗi cho đến khi các tiêu chí bắt buộc đạt.

Được phép làm linh hoạt thứ tự tiểu bước để tiết kiệm công, nhưng phải giao sản phẩm làm việc hoàn chỉnh cho phạm vi V1. Không thêm tính năng ngoại vi (Google Calendar, cloud sync, mobile, reports nâng cao) trước khi core qua QA.

## 11. Kiểm thử và điều kiện hoàn thành

Đọc file `03_KIEM_THU_NGHIEM_THU.md`. Viết automated tests cho scheduler bằng injectable/fake clock và timezone cố định; không cần chờ đến 08:00 thực mới kiểm thử. Tối thiểu test: 07:59 không nhắc; 08:00 một bản đầu ngày; 09:00...17:00 mỗi slot tối đa một; 18:00 một bản tổng kết; 18:01 không nhắc; task 10:30 -> 10:15 pre-alert + 10:30 due; hoàn thành 10:10 -> không pre-alert; snooze -> không gửi ở thời điểm cũ; sleep 08:00–11:00 -> một catch-up summary chứ không ba toast; restart trong cùng giờ -> không lặp; recurring task không sinh trùng; mở hai instance -> chỉ một process; off notifications -> không toast; task lưu lại qua restart; các lệnh chat preview rồi mới ghi DB; dữ liệu backup/restore khớp.

Ngưỡng nghiệm thu: không có lỗi build; typecheck/lint/unit/integration pass hoặc giải thích chính xác những test không thể chạy; các nút trong UI phải hoạt động thực (không có placeholder đánh lừa); chức năng nhắc giờ tin cậy khi dashboard ẩn. Đặc biệt, Windows native toast phải thử ở installed build: tài liệu Tauri nêu rằng desktop notification trên Windows cần ứng dụng đã được cài đặt, chạy chế độ dev không đủ xác nhận.

## 12. Đầu ra Codex phải bàn giao

1. Mã nguồn hoàn chỉnh, hướng dẫn cấu trúc và cách chạy/build.
2. File NSIS installer `.exe` và vị trí thực tế của file nếu đã build trên Windows. Nếu không thể build Windows do môi trường khác hệ điều hành hoặc thiếu toolchain, không tự nhận là đã xuất `.exe`; phải nêu rõ blocker cùng lệnh build trên Windows, kèm bằng chứng các phần khác đã test.
3. README tiếng Việt: cài đặt, lần chạy đầu, bật notification Windows, bật/tắt tự khởi động, mở lại từ tray, chỉnh giờ, sao lưu/khôi phục, gỡ cài đặt và xử lý lỗi phổ biến.
4. Báo cáo QA `TEST_REPORT.md`: từng case pass/fail/blocked, môi trường và bằng chứng hoặc log.
5. Nhật ký tiến độ `IMPLEMENTATION_LOG.md` gồm các chặng hoàn thành và các giới hạn chưa xử lý.
6. Tối thiểu dữ liệu demo có thể nạp qua chức năng seed/dev; bản cài cho người dùng phải khởi động với DB riêng, không trộn dữ liệu demo thực.

Hãy bắt đầu ngay bằng kiểm tra repo và môi trường, sau đó tạo/hoàn thiện mã theo các chặng. Mỗi khi một chặng xong hãy tóm tắt kết quả thực tế, lệnh đã chạy và lỗi còn lại. Không được tuyên bố hoàn thành khi chỉ có UI demo.
