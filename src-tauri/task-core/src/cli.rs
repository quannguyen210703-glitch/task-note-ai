use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use task_note_core::{
    coordinator,
    service::{Command as CoreCommand, Patch, Request, Response},
    *,
};

#[derive(Parser)]
#[command(
    name = "tasknote",
    version,
    about = "Task Note AI — quản lý công việc dùng chung dữ liệu với Sticky Note",
    long_about = "Task Note AI — CLI tiếng Việt. Mặc định dùng profile Windows hiện tại. Dữ liệu thử có thể chỉ định --data-dir. Không nhận API key qua dòng lệnh."
)]
struct Cli {
    #[arg(
        long,
        global = true,
        help = "Xuất duy nhất một JSON có schema_version/ok/data hoặc error"
    )]
    json: bool,
    #[arg(
        long,
        global = true,
        help = "Chỉ xem dự kiến; không ghi task hoặc lịch nhắc"
    )]
    dry_run: bool,
    #[arg(
        long,
        global = true,
        help = "ID ổn định khi retry mutation (1–120 ký tự)"
    )]
    request_id: Option<String>,
    #[arg(
        long,
        global = true,
        help = "Profile riêng cho kiểm thử; bỏ trống để dùng AppData thật"
    )]
    data_dir: Option<PathBuf>,
    #[command(subcommand)]
    command: Top,
}
#[derive(Subcommand)]
enum Top {
    Version,
    Doctor,
    Status,
    Open,
    Task {
        #[command(subcommand)]
        command: TaskCommand,
    },
    Notifications {
        #[command(subcommand)]
        command: NotificationCommand,
    },
    Backup {
        #[command(subcommand)]
        command: BackupCommand,
    },
    Ai {
        #[command(subcommand)]
        command: AiCommand,
    },
}
#[derive(Args)]
struct Scope {
    #[arg(long,value_parser=["occurrence","future"],help="Lần này hoặc lần này và các lần sau; bắt buộc khi sửa/xóa chuỗi")]
    scope: Option<String>,
}
#[derive(Subcommand)]
enum TaskCommand {
    List {
        #[arg(long, conflicts_with = "date")]
        today: bool,
        #[arg(long)]
        date: Option<String>,
        #[arg(long,value_parser=["todo","doing","done"])]
        status: Option<String>,
    },
    Show {
        id: String,
    },
    Add {
        #[arg(long)]
        title: String,
        #[arg(long)]
        date: Option<String>,
        #[arg(long)]
        time: Option<String>,
    },
    Update {
        id: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        date: Option<String>,
        #[arg(long)]
        time: Option<String>,
        #[arg(long,value_parser=["todo","doing","done"])]
        status: Option<String>,
        #[arg(long,value_parser=["high","medium","low"])]
        priority: Option<String>,
        #[command(flatten)]
        scope: Scope,
    },
    Complete {
        id: String,
    },
    Reopen {
        id: String,
    },
    Snooze {
        id: String,
        #[arg(
            long = "for",
            conflicts_with = "until",
            required_unless_present = "until"
        )]
        duration: Option<String>,
        #[arg(long)]
        until: Option<String>,
    },
    Unsnooze {
        id: String,
    },
    Delete {
        id: String,
        #[command(flatten)]
        scope: Scope,
    },
    Restore {
        id: String,
    },
    Trash {
        #[command(subcommand)]
        command: TrashCommand,
    },
}
#[derive(Subcommand)]
enum TrashCommand {
    List,
}
#[derive(Subcommand)]
enum NotificationCommand {
    Pause {
        #[arg(long = "for")]
        duration: String,
    },
    Resume,
}
#[derive(Subcommand)]
enum BackupCommand {
    Create {
        #[arg(long)]
        output: PathBuf,
    },
    Verify {
        path: PathBuf,
    },
}
#[derive(Subcommand)]
enum AiCommand {
    Chat {
        text: String,
    },
    Plan {
        #[arg(long, default_value = "today")]
        date: String,
    },
}
fn minutes(text: &str) -> Result<i64> {
    let (value, scale) = if let Some(v) = text.strip_suffix('m') {
        (v, 1)
    } else if let Some(v) = text.strip_suffix('h') {
        (v, 60)
    } else {
        return Err(Error::Invalid("Thời lượng cần 30m hoặc 1h.".into()));
    };
    let value = value
        .parse::<i64>()
        .ok()
        .and_then(|v| v.checked_mul(scale))
        .filter(|v| (1..=525600).contains(v))
        .ok_or_else(|| Error::Invalid("Thời lượng 1–525600 phút.".into()))?;
    Ok(value)
}
fn command(top: Top) -> Result<CoreCommand> {
    Ok(match top {
        Top::Status | Top::Doctor => CoreCommand::Status,
        Top::Task { command } => match command {
            TaskCommand::List {
                today,
                date,
                status,
            } => CoreCommand::List {
                today,
                date,
                status,
                trash: false,
            },
            TaskCommand::Show { id } => CoreCommand::Show { id },
            TaskCommand::Add { title, date, time } => CoreCommand::Add { title, date, time },
            TaskCommand::Update {
                id,
                title,
                description,
                date,
                time,
                status,
                priority,
                scope,
            } => CoreCommand::Update {
                id,
                patch: Patch {
                    title,
                    description,
                    date,
                    time,
                    status,
                    priority,
                },
                scope: scope.scope,
            },
            TaskCommand::Complete { id } => CoreCommand::Complete { id },
            TaskCommand::Reopen { id } => CoreCommand::Reopen { id },
            TaskCommand::Snooze {
                id,
                duration,
                until,
            } => CoreCommand::Snooze {
                id,
                minutes: duration.as_deref().map(minutes).transpose()?,
                until,
            },
            TaskCommand::Unsnooze { id } => CoreCommand::Unsnooze { id },
            TaskCommand::Delete { id, scope } => CoreCommand::Delete {
                id,
                scope: scope.scope,
            },
            TaskCommand::Restore { id } => CoreCommand::Restore { id },
            TaskCommand::Trash { .. } => CoreCommand::List {
                today: false,
                date: None,
                status: None,
                trash: true,
            },
        },
        Top::Notifications { command } => match command {
            NotificationCommand::Pause { duration } => CoreCommand::Pause {
                minutes: minutes(&duration)?,
            },
            NotificationCommand::Resume => CoreCommand::Resume,
        },
        Top::Backup { command } => match command {
            BackupCommand::Create { output } => CoreCommand::BackupCreate {
                output: std::path::absolute(output)?,
            },
            BackupCommand::Verify { path } => CoreCommand::BackupVerify {
                path: std::path::absolute(path)?,
            },
        },
        Top::Ai { command } => match command {
            AiCommand::Chat { text } => CoreCommand::Chat { text },
            AiCommand::Plan { date } => CoreCommand::Plan { date },
        },
        _ => unreachable!(),
    })
}
#[tokio::main]
async fn main() {
    let json = std::env::args().any(|a| a == "--json");
    let cli = match Cli::try_parse() {
        Ok(v) => v,
        Err(e) => {
            if e.exit_code() == 0 {
                print!("{e}");
                return;
            }
            if json {
                println!(
                    "{}",
                    serde_json::to_string(&Response::failure(2, e.to_string())).unwrap()
                );
            } else {
                eprintln!("{e}");
            }
            std::process::exit(2);
        }
    };
    if matches!(cli.command, Top::Version) {
        let r = Response::from_result(Ok(serde_json::json!({"version":env!("CARGO_PKG_VERSION")})));
        print_response(r, cli.json);
        return;
    }
    if matches!(cli.command, Top::Open) {
        let result = (|| -> Result<serde_json::Value> {
            if cli.data_dir.is_some() {
                return Err(Error::Invalid(
                    "open dùng app đã cài; không tự mở GUI profile thử.".into(),
                ));
            }
            let exe = std::env::current_exe()?.with_file_name("task-note-ai.exe");
            std::process::Command::new(exe).spawn()?;
            Ok(serde_json::json!({"message":"Đã gửi yêu cầu mở Note"}))
        })();
        print_response(Response::from_result(result), cli.json);
        return;
    }
    let response=async {
        let root=match cli.data_dir {Some(p)=>std::path::absolute(p)?,None=>coordinator::data_root()?};
        let now=chrono::Utc::now().timestamp();
        let request=Request{request_id:cli.request_id.unwrap_or_else(||uuid::Uuid::new_v4().to_string()),received_at_utc:now,expires_at_utc:now+15,dry_run:cli.dry_run,action:command(cli.command)?};
        // Older GUI versions do not use the coordinator. Do not write beside them.
        let mut owner=coordinator::try_owner(&root)?;
        for _ in 0..150 {
            if owner.is_some(){break;}
            if root.join("gui-session.json").exists(){return coordinator::send(&root,&request).await;}
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            owner=coordinator::try_owner(&root)?;
        }
        if owner.is_none(){return Ok(Response::failure(4,"Core đang bận/đang thoát; thử lại cùng request ID.".into()));}
        let _=std::fs::remove_file(root.join("gui-session.json"));
        if coordinator::legacy_active(&root){return Ok(Response::failure(6,"Profile V1 chưa có core V2. Lưu và thoát V1 rồi mở V2 để hoàn tất nâng cấp; CLI không tạo writer song song với V1.".into()));}
        let path=root.join("task-note.sqlite");
        let repo=if request.action.readonly() || request.dry_run {Repository::open_readonly(&path).await?}else{Repository::open(&path).await?};
        let response=repo.execute(&request,&root,false).await;
        repo.pool.close().await;drop(owner);Ok(response)
    }.await;
    print_response(
        match response {
            Ok(v) => v,
            Err(e) => Response::from_result(Err(e)),
        },
        cli.json,
    );
}
fn print_response(response: Response, json: bool) {
    let code = response.error.as_ref().map_or(0, |e| e.code);
    if json {
        println!("{}", serde_json::to_string(&response).unwrap());
    } else if let Some(error) = response.error {
        eprintln!("{}", error.message);
    } else if let Some(data) = response.data {
        println!("{}", serde_json::to_string_pretty(&data).unwrap());
    }
    if code != 0 {
        std::process::exit(code);
    }
}
