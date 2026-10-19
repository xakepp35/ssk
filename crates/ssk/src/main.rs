use clap::{Parser, Subcommand, ValueEnum};
mod config;
mod constants;
mod observation;

#[derive(Parser)]
#[command(version, about = "Syrup Space Kadet — procedural space arcade")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    #[arg(long, global = true)]
    seed: Option<u64>,
    /// Optional initial ship override; F2 opens the ship selection menu.
    #[arg(long, global = true, value_enum)]
    class: Option<Class>,
    #[arg(long, global = true)]
    config: Option<std::path::PathBuf>,
    /// Skip native audio initialization (useful when diagnosing a USB/audio freeze).
    #[arg(long, global = true)]
    no_audio: bool,
    /// NDJSON destination override (startup-log is a compatibility alias).
    #[arg(long, alias = "startup-log", global = true)]
    log_file: Option<std::path::PathBuf>,
    #[arg(long, global = true, default_value = constants::observation::LOG_DIR)]
    log_dir: std::path::PathBuf,
    /// tracing filter; otherwise SSK_LOG or own DEBUG / dependency WARN.
    #[arg(long, global = true)]
    log_filter: Option<String>,
    /// Always sync each record by default; flush omits the storage sync request.
    #[arg(long, global = true, value_enum, default_value_t = LogSync::Always)]
    log_sync: LogSync,
    #[arg(long, global = true)]
    log_no_console: bool,
    /// Repetitive non-error records per callsite per second; 0 disables suppression.
    #[arg(long, global = true, default_value_t = constants::observation::SITE_RECORDS_PER_SEC)]
    log_rate_limit: u64,
}

#[derive(Subcommand)]
enum Command {
    Play,
    /// Play and accept LAN joins on TCP 4202.
    Host,
    Online {
        #[arg(long)]
        connect: Option<String>,
    },
    /// Headless rendezvous/discovery node; players retain ownership of their ships.
    #[command(alias = "node")]
    Server,
    InitConfig,
    /// Read-only host/process snapshot and exit; no graphics, game or network.
    Doctor,
    /// Initialize only graphics, clear one frame, then exit; no game/network/audio.
    GraphicsProbe {
        #[arg(long)]
        log: Option<std::path::PathBuf>,
    },
    Bench {
        #[arg(long, default_value_t = ssk_client::constants::BENCH_DEFAULT_PLAYERS)]
        players: usize,
        #[arg(long, default_value_t = ssk_client::constants::BENCH_DEFAULT_SECONDS)]
        seconds: u64,
        #[arg(long, default_value_t = ssk_client::constants::BENCH_DEFAULT_SECONDS)]
        warmup: u64,
        #[arg(long)]
        no_fire: bool,
        #[arg(long)]
        connect: Option<String>,
        #[arg(long)]
        invite_file: Option<std::path::PathBuf>,
        #[arg(long)]
        output: Option<std::path::PathBuf>,
        #[arg(long, default_value_t = 0)]
        drop_count: usize,
        #[arg(long, default_value_t = ssk_client::constants::BENCH_DEFAULT_WORKERS)]
        workers: usize,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum Class {
    Spark,
    Cadet,
    Thunder,
}

#[derive(Clone, Copy, ValueEnum)]
enum LogSync {
    Always,
    Flush,
}

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    let file = cli.log_file.clone().or_else(|| match &cli.command {
        Some(Command::GraphicsProbe { log, .. }) => log.clone(),
        _ => None,
    });
    let settings = observation::Settings {
        directory: cli.log_dir.clone(),
        file,
        filter: cli
            .log_filter
            .clone()
            .or_else(|| std::env::var("SSK_LOG").ok())
            .unwrap_or_else(|| constants::observation::FILTER.into()),
        console: !cli.log_no_console,
        sync: matches!(cli.log_sync, LogSync::Always),
        rate_limit: cli.log_rate_limit,
    };
    let _observation = match observation::install(settings) {
        Ok(guard) => guard,
        Err(error) => {
            use std::io::Write;
            let record = serde_json::json!({"schema":constants::observation::SCHEMA,"event_id":"ssk.observe.init_failed","level":"ERROR","fields":{"error":error.to_string()}});
            let _ = writeln!(std::io::stderr().lock(), "{record}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let _session =
        tracing::info_span!("ssk.process.run", version = env!("CARGO_PKG_VERSION")).entered();
    tracing::info!(name:"ssk.process.begin",version=env!("CARGO_PKG_VERSION"),"Process entry before application config");
    match run(cli) {
        Ok(()) => {
            tracing::info!(name:"ssk.process.end",success=true,"Process work completed");
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            tracing::error!(name:"ssk.process.failed",error=%format!("{error:#}"),"Process work failed");
            std::process::ExitCode::FAILURE
        }
    }
}
fn run(cli: Cli) -> anyhow::Result<()> {
    if matches!(cli.command, Some(Command::Doctor)) {
        observation::snapshot();
        return Ok(());
    }
    if matches!(cli.command, Some(Command::InitConfig)) {
        println!(
            "{}",
            config::Config::write_default(cli.config.as_deref())?.display()
        );
        return Ok(());
    }
    let mut app = config::Config::load(cli.config.as_deref())?;
    if cli.no_audio {
        app.audio.enabled = false;
    }
    if let Some(seed) = cli.seed {
        app.network.seed = seed;
    }
    let command = cli.command.unwrap_or(match app.mode {
        config::Mode::Offline => Command::Play,
        config::Mode::Host => Command::Host,
        config::Mode::Join => Command::Online { connect: None },
    });
    if matches!(command, Command::Host | Command::Server) {
        app.network
            .bootstrap_bind
            .get_or_insert(ssk_net::constants::DEFAULT_BOOTSTRAP_BIND.parse()?);
    } else {
        app.network.bootstrap_bind = None;
    }
    let network = app.network;
    match command {
        Command::GraphicsProbe { .. } => {
            ssk_client::diagnostics::graphics_probe(&app.window)?;
            Ok(())
        }
        Command::Server => ssk_net::run_node(network),
        Command::Bench {
            players,
            seconds,
            warmup,
            no_fire,
            connect,
            invite_file,
            output,
            drop_count,
            workers,
        } => {
            let report = ssk_client::bench::run(
                network,
                ssk_client::bench::Options {
                    players,
                    seconds,
                    warmup,
                    fire: !no_fire,
                    connect,
                    invite_file,
                    drop_count,
                    workers,
                },
            )
            .map_err(anyhow::Error::msg)?;
            let json = serde_json::to_string_pretty(&report)?;
            if let Some(path) = output {
                if let Some(p) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                    std::fs::create_dir_all(p)?;
                }
                std::fs::write(path, json)?;
            } else {
                println!("{json}");
            }
            Ok(())
        }
        Command::InitConfig | Command::Doctor => {
            unreachable!("handled before configuration loading")
        }
        command => {
            let online = matches!(command, Command::Online { .. } | Command::Host);
            let connect = match command {
                Command::Online { connect } => connect.or(app.connect),
                _ => None,
            };
            let class = cli.class.map(|class| match class {
                Class::Spark => ssk_sim::ShipClass::Spark,
                Class::Cadet => ssk_sim::ShipClass::Cadet,
                Class::Thunder => ssk_sim::ShipClass::Thunder,
            });
            ssk_client::launch(ssk_client::Options {
                seed: network.seed,
                class,
                window: app.window,
                audio: app.audio,
                play: true,
                online,
                connect,
                network,
            })?;
            Ok(())
        }
    }
}
