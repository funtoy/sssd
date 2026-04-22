use anyhow::anyhow;
use clap::{Parser, Subcommand};
use std::env;
use std::fs::{create_dir_all, OpenOptions};
use std::future::Future;
use std::process::Command;
use sysinfo::{Signal, System};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Status,
    Start {
        #[arg(short = 'd', long = "daemon", help = "Run as background daemon")]
        daemon: bool,
    },
    Stop,
    Daemon,
}

/// # A simple way to let your app support like:
/// ```shell
/// ./your_app start [-d] | stop | status | daemon
/// ```
/// # create
///
/// ```rust
/// #[tokio::main]
/// async fn main() {
///     sssd::create(your_async_func).await.unwrap()
/// }
///```
pub async fn create<F, Fut>(func: F) -> anyhow::Result<()>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = anyhow::Result<()>>,
{
    let stop_callback = async || -> anyhow::Result<()> { Ok(()) };
    create_with_stop(func, stop_callback).await
}

/// # create_with_stop
///
/// ```rust
/// #[tokio::main]
/// async fn main() {
///     sssd::create_with_stop(your_async_func, stop_callback).await.unwrap()
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
pub async fn create_with_stop<F, Fut, S, SFut>(func: F, stop_callback: S) -> anyhow::Result<()>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = anyhow::Result<()>>,
    S: FnOnce() -> SFut,
    SFut: Future<Output = anyhow::Result<()>>,
{
    let args = Args::parse();
    match args.cmd {
        Commands::Status => println!("{}", status()?.0),
        Commands::Start { daemon } => {
            let (msg, running) = status()?;
            if running {
                println!("{msg}");
                return Ok(());
            }
            if daemon {
                spawn_daemon()?;
            } else {
                tokio::select! {
                    res = func() => {
                        if let Err(e) = res { eprintln!("Application Start error: {}", e); }
                    }
                    _ = tokio::signal::ctrl_c() => {
                        println!("Gracefully shutting down...");
                    }
                }
                if let Err(e) = stop_callback().await { eprintln!("Application Stop error: {}", e); }
            }
        }
        Commands::Stop => {
            let exe = env::current_exe()?;
            let app = exe.file_name()
                .ok_or_else(|| anyhow!("Failed to get exe name"))?
                .to_owned();
            let current_pid = std::process::id();
            System::new_all()
                .processes_by_exact_name(app.as_ref())
                .filter(|p| p.pid().as_u32() != current_pid)
                .for_each(|p| {
                    println!("<{}> {} is stopping...", p.pid(), p.name().to_string_lossy());
                    match p.kill_with_and_wait(Signal::Interrupt) {
                        Ok(_) => println!("<{}> {} is stopped now", p.pid(), p.name().to_string_lossy()),
                        Err(e) => eprintln!("Stop Error: {e:?}"),
                    }
                });
        }
        Commands::Daemon => {
            let (msg, running) = status()?;
            if running {
                println!("{msg}");
            } else {
                spawn_daemon()?;
            }
        }
    }
    Ok(())
}

fn spawn_daemon() -> anyhow::Result<()> {
    let exe_full = env::current_exe()?;
    let app_name = exe_full
        .file_stem()
        .ok_or_else(|| anyhow!("Failed to get exe stem"))?
        .to_string_lossy()
        .into_owned();
    let mut log_path = exe_full
        .parent()
        .ok_or_else(|| anyhow!("Failed to get exe parent path"))?
        .to_path_buf();
    log_path.push("logs");
    create_dir_all(&log_path)?;
    log_path.push(format!("{app_name}.log"));
    let stdout = OpenOptions::new().create(true).append(true).open(&log_path)?;
    let stderr = stdout.try_clone()?;
    Command::new(exe_full).arg("start").stdout(stdout).stderr(stderr).spawn()?;
    Ok(())
}

fn status() -> anyhow::Result<(String, bool)> {
    let exe = env::current_exe()?;
    let app = exe
        .file_name()
        .ok_or_else(|| anyhow!("Failed to get exe name"))?
        .to_owned();
    let current_pid = std::process::id();
    let sys = System::new_all();
    let processes: Vec<_> = sys
        .processes_by_exact_name(app.as_os_str())
        .filter(|p| p.pid().as_u32() != current_pid)
        .collect();

    if processes.is_empty() {
        Ok((format!("{} is stopped!", app.to_string_lossy()), false))
    } else {
        let msg = processes
            .iter()
            .map(|p| format!("<{}> {} is running.", p.pid(), p.name().to_string_lossy()))
            .collect::<Vec<_>>()
            .join("\n");
        Ok((msg, true))
    }
}
