/// A simple way to let your app support like ./your_app start | stop | status | daemon.
/// # Examples
/// ```
/// #[actix_web::main]
/// async fn main() {
///     sssd::create(|| your_async_func()).await
/// }
/// ```
use std::env;
use std::fs::{create_dir, OpenOptions};
use std::future::Future;
use std::process::Command;
use sysinfo::{PidExt, ProcessExt, System, SystemExt};

pub async fn create<F, Fut>(func: F)
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<(), std::io::Error>>,
{
    let app = get_exec_name().unwrap();
    let args: Vec<String> = env::args().collect();

    match matching(args).as_str() {
        "status" => {
            let (msg, _) = status(&app);
            println!("{msg}");
        }

        "stop" => {
            let sys = System::new_all();
            for process in sys.processes_by_name(&app) {
                if process.pid().as_u32().ne(&std::process::id()) {
                    println!("<{}> {} is stopping...", process.pid(), process.name());
                    process.kill();
                }
            }
        }

        "daemon" => {
            let (msg, is_running) = status(&app);
            if is_running {
                println!("{msg}");
            } else {
                let app = format!("./{app}");
                create_dir("./logs").ok();
                let out = format!("./logs/{app}.log");
                let stdout = OpenOptions::new()
                    .write(true)
                    .append(true)
                    .create(true)
                    .open(&out)
                    .unwrap();
                let stderr = OpenOptions::new()
                    .write(true)
                    .append(true)
                    .create(true)
                    .open(&out)
                    .unwrap();
                Command::new(app)
                    .arg("start")
                    .stdout(stdout)
                    .stderr(stderr)
                    .spawn()
                    .expect("fail to start the app in daemon mode");
            }
        }

        "start" => {
            func().await.expect("fail to start the app");
        }

        _ => {
            println!("Help: ./{app} status | start | stop | daemon");
        }
    }
}

fn get_exec_name() -> Option<String> {
    env::current_exe()
        .ok()
        .and_then(|pb| pb.file_name().map(|s| s.to_os_string()))
        .and_then(|s| s.into_string().ok())
}

fn status(app: &str) -> (String, bool) {
    let sys = System::new_all();
    for process in sys.processes_by_name(&app) {
        if process.pid().as_u32().ne(&std::process::id()) {
            return (
                format!("<{}> {} is running.", process.pid(), process.name()),
                true,
            );
        }
    }
    return (format!("{app} is stopped!"), false);
}

fn matching(args: Vec<String>) -> String {
    for arg in args {
        if arg.eq("status") {
            return arg;
        }
        if arg.eq("start") {
            return arg;
        }
        if arg.eq("daemon") {
            return arg;
        }
        if arg.eq("stop") {
            return arg;
        }
    }
    return "help".to_string();
}
