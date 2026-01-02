use clap::{Parser, ValueEnum};
use std::env;
use std::fs::{create_dir_all, OpenOptions};
use std::future::Future;
use std::process::Command;
use sysinfo::{Signal, System};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(value_name = "status | start | stop | daemon")]
    cmd: Commands,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum Commands {
    Status,
    Start,
    Stop,
    Daemon,
}

/// # A simple way to let your app support like:
/// ```shell
/// ./your_app start | stop | status | daemon
/// ```
/// # Examples
///
/// ```rust
/// #[tokio::main]
/// async fn main() {
///     sssd::create(your_async_func, Some(stop_callback)).await
/// }
///
/// async fn your_async_func() -> anyhow::Result<()> {
///     // ...
/// }
///
/// async fn stop_callback() -> anyhow::Result<()> {
///     // ...
/// }
///```
pub async fn create<F, Fut, S, SFut>(func: F, stop_callback: Option<S>)
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = anyhow::Result<()>>,
    S: FnOnce() -> SFut,
    SFut: Future<Output = anyhow::Result<()>>,
{
    let args = Args::parse();
    match args.cmd {
        Commands::Status => println!("{}", status().0),
        Commands::Start => {
            tokio::select! {
                res = func() => {
                    if let Err(e) = res { eprintln!("Application Start error: {}", e); }
                }
                _ = tokio::signal::ctrl_c() => {
                    println!("Gracefully shutting down...");
                    if let Some(callback) = stop_callback {
                        if let Err(e) = callback().await { eprintln!("Application Stop error: {}", e); }
                    }
                }
            }
        }
        Commands::Stop => {
            let app = env::current_exe().unwrap().file_name().unwrap().to_owned();
            let current_pid = std::process::id();
            System::new_all().processes_by_exact_name(app.as_ref()).filter(|p| p.pid().as_u32() != current_pid).for_each(|p| {
                println!("<{}> {} is stopping...", p.pid(), p.name().to_string_lossy());
                match p.kill_with_and_wait(Signal::Interrupt) {
                    Ok(_) => println!("<{}> {} is stopped now", p.pid(), p.name().to_string_lossy()),
                    Err(e) => eprintln!("Stop Error: {e:?}"),
                }
            });
        }
        Commands::Daemon => {
            let (msg, running) = status();
            if running {
                println!("{msg}");
            } else {
                let exe_full = env::current_exe().unwrap();
                let app_name = exe_full.file_stem().unwrap().to_string_lossy().into_owned();
                let mut log_path = exe_full.parent().expect("Failed to get exec parent's path").to_path_buf();
                log_path.push("logs");
                create_dir_all(&log_path).expect("Failed to create log path");
                log_path.push(format!("{app_name}.log"));
                let stdout = OpenOptions::new().create(true).append(true).open(&log_path).expect("Failed to create `stdout`");
                let stderr = stdout.try_clone().expect("Failed to clone stdout handle");
                Command::new(exe_full).arg("start").stdout(stdout).stderr(stderr).spawn().expect("Failed to daemonize");
            }
        }
    }
}

fn status() -> (String, bool) {
    let app = env::current_exe().unwrap().file_name().unwrap().to_owned();
    let current_pid = std::process::id();
    System::new_all()
        .processes_by_exact_name(app.clone().as_os_str())
        .find(|p| p.pid().as_u32() != current_pid)
        .map(|p| (format!("<{}> {} is running.", p.pid(), p.name().to_string_lossy()), true))
        .unwrap_or_else(|| (format!("{} is stopped!", app.to_string_lossy()), false))
}
