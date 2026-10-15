# Bảo toàn dữ liệu V1 → V2

## Quyết định triển khai

Người dùng đã chọn **giữ bản đang chạy, chỉ bàn giao bộ cài**. Không đóng/restart V1, không chạy installer lên profile thật, không migrate DB thật. Không có cutover, nên chưa có “backup cuối trước cutover”. Checkpoint khi bàn giao cũng không được dùng thay một backup mới tại thời điểm cài sau này.

## Dữ liệu và checkpoint

- Data thật đã xác minh: `C:\Users\HP\AppData\Roaming\vn.tasknoteai.desktop\task-note.sqlite`.
- Checkpoint đầu: `artifacts/preservation/initial-v1.sqlite`; Online Backup API lấy cả giao dịch đã commit trong WAL. 32 task, 2 template, 1 settings, 6 dispatch lịch sử tại snapshot. `integrity_check=ok`, `foreign_key_check=[]`.
- Manifest có SHA256 toàn file, hash từng bảng và danh sách trường; không đưa nội dung task cá nhân vào báo cáo.
- Source/lockfiles trước nâng: `artifacts/preservation/source-v1.zip`; binary cũ `task-note-ai-v1.exe`. Thư mục preservation bị loại khỏi Git và gói source công khai vì chứa dữ liệu riêng.
- Checkpoint bàn giao 15/10: `artifacts/preservation/handoff-v1-20261015.sqlite`, integrity/foreign keys đạt; **34 task**, 2 template, schema V1 vẫn 1. Có 2 task mới so với snapshot đầu trong khi người dùng tiếp tục dùng V1. Không khôi phục snapshot cũ đè lên dữ liệu mới; đối soát migration staging cuối dùng checkpoint mới này. Lịch sử dispatch tăng bình thường do V1 vẫn chạy.
- Bản V1 không có flush draft. Không tự đóng cửa sổ của người dùng. V2 có lưu nháp form/chat trong SQLite; khóa AI không được lưu vào draft.

## Migration và đối soát

Migration `0002_upgrade.sql` chỉ thêm bảng; không sửa/drop cột task V1, không đổi ID, deadline, todo/doing/done, snooze, archive hoặc settings V1. Bảng calendar mới được backfill theo timezone cũ để giữ ngày local cho task cả ngày. Schema V1 không có status paused, nên không cần đoán previous_status.

`scripts/compare_upgrade.py` đã đối chiếu từng giá trị của toàn bộ cột cũ, sắp xếp theo ID, trên bản sao trước/sau migration: **PASS tất cả 6 bảng dữ liệu cũ**, gồm tasks, templates, settings, dispatches, proposals và chat_history. Task doing giữ nguyên ID/trạng thái/ghi chú/hạn/snooze như mọi bản ghi khác. Không dùng số lượng hàng thay cho so sánh nội dung. Chỉ metadata `_sqlx_migrations` tăng phiên bản và các bảng V2 được thêm. Bằng chứng: `artifacts/v2-migration-comparison.json`.

`artifacts/v2-staging-tests.json`: **PASS** lỗi giữa migration (collision có chủ ý trên fixture), rollback transaction không để bảng V2 dở dang, retry thành công và đối chiếu lại giữ toàn bộ trường cũ. **PASS** external DB lock: lỗi có giới hạn, thả lock rồi retry cùng request ID chỉ tạo một task thử. SQLite runtime của binary là **3.51.3**, schema **2**. Chưa gây thiếu dung lượng đĩa thật hoặc chuyển Windows admin profile.

Khi mở DB V1 bằng V2, core giữ owner lock, dùng VACUUM INTO tạo `pre-v2-<UUID>.sqlite`, verify độc lập rồi mới migrate. Lỗi migrate trả lỗi khởi động, không reset DB và không tạo DB khác để che lỗi. SQLx migration chạy transaction; CLI và GUI dùng cùng lock trước khi mở writer/migrate.

CLI mặc định chặn profile có metadata V1 hoặc metadata đang ghi dở **không phụ thuộc tuổi heartbeat**: máy ngủ không được hiểu là V1 đã thoát. Cần lưu/thoát V1 rồi mở GUI V2 trước để hoàn tất nâng profile. Sau khi V2 đã chạy, CLI có thể xử lý headless dưới cùng owner lock khi GUI tắt. Profile staging không mang heartbeat V1 và được dùng riêng để thử migration.

## Quy trình nâng sau khi anh chọn cài

1. Lưu nội dung đang nhập trong V1; thoát sạch từ tray. Không kill app khi chưa lưu.
2. Tạo backup mới nhất bằng `scripts/preserve.py` (Python runtime) hoặc cơ chế online backup đã xác minh; verify integrity/foreign keys. Giữ bản V1 và binary/source cũ.
3. Chạy installer V2 bằng đúng Windows user, giữ install path và identifier `vn.tasknoteai.desktop`.
4. Mở Note, chạy `tasknote doctor --json`, đối chiếu công việc doing/note/deadline/snooze và backup mới nhất theo ID. Không thử xóa task thật để kiểm chứng.
5. Chỉ cho dùng bình thường sau đối soát. Không dùng checkpoint đầu buổi để ghi đè công việc phát sinh trong lúc build.

## Rollback

Không mở binary V1 trên DB V2 chưa được xác minh tương thích. Nếu chưa có ghi mới sau nâng, có thể quay lại **bản backup cuối trước cutover** sau khi đã backup trạng thái hiện tại và kiểm chứng không có giao dịch mới. Nếu đã ghi mới: giữ DB V2 và backup mới, sửa tiếp hoặc làm migration ngược/merge có đối soát; không restore mù snapshot cũ. Không có nút restore đè DB hoặc purge/reset trong CLI.

Các thử nghiệm cài tại chỗ/rollback thật và nội dung chưa lưu trong V1 chưa được kiểm chứng vì người dùng chọn giữ app đang chạy.
