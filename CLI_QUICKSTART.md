# CLI Task Note AI 2.0

PowerShell, sau khi cài V2 vào thư mục mặc định:

```powershell
$tasknote = "$env:LOCALAPPDATA\Task Note AI\tasknote.exe"
& $tasknote --help
& $tasknote version
& $tasknote doctor --json
& $tasknote task list --today --json
& $tasknote task add --title "Kiểm tra tài liệu" --date today
```

CLI cũng được bàn giao ở `artifacts\tasknote.exe`. Không tự sửa PATH. Có thể dùng đường dẫn đầy đủ trong terminal hiện tại. Bản V1 đang chạy không có giao thức CLI mới: CLI từ chối ghi song song; chỉ sử dụng profile thật sau khi đã nâng lên V2.

Lấy `id` từ `task list --json`, rồi thay ID thật vào biến dưới đây. Ví dụ không tự chạy lên dữ liệu của anh:

```powershell
$taskId = '<ID lấy từ danh sách>'
& $tasknote task show $taskId
& $tasknote task update $taskId --status doing
& $tasknote task snooze $taskId --for 30m
& $tasknote task unsnooze $taskId
& $tasknote task complete $taskId
& $tasknote task reopen $taskId
& $tasknote task delete $taskId
& $tasknote task trash list --json
& $tasknote task restore $taskId
```

Hoãn chỉ chặn nhắc, không đổi deadline/trạng thái. `unsnooze` giữ ID và nội dung, gọi lặp an toàn. Xóa mềm chuyển Thùng rác, khôi phục đúng ID. Lịch lặp cần `--scope occurrence` hoặc `--scope future` khi update/delete; scope future không xóa lịch sử cũ đã hoàn thành.

```powershell
& $tasknote task add --title "Gọi khách" --date tomorrow --time 09:00
& $tasknote task list --date 2026-10-15 --status doing --json
& $tasknote task update $taskId --date 2026-10-15 --time 14:00 --dry-run --json
& $tasknote task snooze $taskId --until "2026-10-15T09:00:00+07:00"
& $tasknote notifications pause --for 60m
& $tasknote notifications resume
& $tasknote backup create --output "$env:USERPROFILE\Documents\TaskNoteBackups"
& $tasknote backup verify '<đường dẫn .sqlite vừa tạo>'
& $tasknote ai chat "Hôm nay thêm việc kiểm tra logo"
& $tasknote ai plan --date today --dry-run
& $tasknote open
```

`today/tomorrow` theo múi giờ cấu hình app. Thiếu ngày khi thêm việc mặc định hôm nay, không theo UTC terminal; thiếu giờ là cả ngày. `--dry-run` không ghi DB/task/scheduling, cần profile đã được khởi tạo. `ai plan` luôn là đề xuất, không thêm/xóa lịch đã có.

Khi tự động hóa, tạo một request ID cho mỗi ý định, dùng lại cùng ID và nguyên lệnh nếu timeout:

```powershell
& $tasknote task add --title "Kiểm tra tài liệu" --date today --request-id 'my-unique-request-001' --json
```

Không tái sử dụng ID cho lệnh khác. Không dùng raw SQL, không truyền API key trong arguments. JSON stdout có `schema_version`, `ok`, `data` hoặc `error.code/message`; diagnostics ra stderr.

Exit codes: 0 thành công; 2 input/conflict không hợp lệ; 3 không có ID/file; 4 DB/IPC/I/O; 5 ID/scope mơ hồ; 6 V1/core hoặc cấu hình không cho phép; 7 AI chưa sẵn sàng. Hai lệnh CLI được điều phối bằng owner lock; khi GUI chạy, chúng gửi request vào kênh file IPC trong AppData chỉ current user có quyền ghi. Không mở cổng mạng công khai.

Kiểm thử riêng: thêm `--data-dir '<thư mục test riêng>'`. Tuyệt đối không dùng đường dẫn dữ liệu thật cho script smoke.
