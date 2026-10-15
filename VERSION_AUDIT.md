# Kiểm tra phiên bản — 14/10/2026

Đã đối chiếu registry npm/crates.io trực tiếp và tài liệu chính thức. Không cài/cập nhật Codex hay công cụ toàn máy.

| Thành phần | V1 / hiện có | Đích và quyết định |
|---|---|---|
| App | Installed/file/product 1.0.0 | 2.0.0; đồng bộ package.json, Cargo manifests, Tauri config |
| Tauri | 2.12.3 | Giữ stable major 2; plugin theo Cargo.lock; identifier không đổi |
| Rust | stable MSVC 1.99.0 | Giữ; minimum 1.94 theo SQLx 0.9 |
| SQLx | 0.8.6 | 0.9.0 (stable registry); API hiện dùng đã compile, kiểm thử migration/CRUD |
| SQLite bundled | libsqlite3-sys 0.30.1 | 0.37.0 / SQLite 3.51.3, bản tương thích constraint SQLx `<0.38`. 0.38.2 mới hơn nhưng ngoài constraint |
| CLI | Chưa có | clap 4.6.7, fs2 0.4.3; console binary riêng |
| AI HTTP | Chưa có adapter thực | reqwest 0.13.5, REST OpenAI + strict JSON schema; model do người dùng nhập và test thực |
| Kho khóa | Chưa có | keyring 3.6.3 `windows-native`; giữ API đã kiểm chứng thay vì major 4.2.0 có thay đổi cấu hình |
| Node / pnpm | 24.19.0 / 11.25.0 | Giữ runtime hiện có |
| React / TypeScript | Major 19 / 5.9.3 | Giữ major tương thích; TypeScript stable registry mới 7.0.2 có thay đổi lớn, không ép nâng trong bản sửa dữ liệu |
| Vite / Vitest | 6.4.4 / 3.2.7 | Giữ major đang build/test; stable registry mới 8.3.4 / 5.0.4 cần migration riêng |
| ESLint | 9.39.5 | Giữ để tránh đổi rule/config trong lần nâng này; registry mới 10.12.0, 9 đã cảnh báo hết hỗ trợ |
| Testing Library user-event | 14.6.7 | 14.6.8 (compatible update) |

Lockfile được cập nhật bằng package manager, không dùng `audit fix --force`. Kết quả engine thực lấy từ `tasknote doctor --json` trong kiểm thử binary; xem artifact/báo cáo nghiệm thu, không suy từ version crate.

## Nguồn

- [SQLite WAL và WAL-reset bug](https://sqlite.org/wal.html): 3.51.3 là bản vá; V1 thuộc dải ảnh hưởng. Giảm pool xuống một connection và dùng một process owner; vẫn nâng engine.
- [SQLite online backup](https://sqlite.org/backup.html): script preserve dùng Online Backup API; CLI/GUI dùng VACUUM INTO nhất quán.
- [Tauri v2 config](https://v2.tauri.app/reference/config/), [Tauri path](https://v2.tauri.app/reference/javascript/api/namespacepath/): giữ identifier/data root; chỉ profile thử dùng định danh riêng.
- [SQLx](https://crates.io/crates/sqlx), [libsqlite3-sys](https://crates.io/crates/libsqlite3-sys), [clap](https://crates.io/crates/clap), [fs2](https://docs.rs/fs2/0.4.3/fs2/trait.FileExt.html).
- [keyring 3.6.3](https://docs.rs/keyring/3.6.3/keyring/): Windows Credential Manager; không export khóa.
- [OpenAI Structured Outputs](https://developers.openai.com/api/docs/guides/structured-outputs?api-mode=chat), [Chat Completions](https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create): POST từ Rust, `store:false`, provider cố định HTTPS, timeout/response size/schema validation; không thực thi tool/SQL/shell do mô hình trả về.

Các phiên bản major mới chưa được ép vào app; không tuyên bố toàn bộ dependency là bản mới nhất.
