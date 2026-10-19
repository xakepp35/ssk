use clap::{Parser, Subcommand, ValueEnum};
mod config;
mod constants;

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

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
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
        Command::InitConfig => unreachable!("handled before configuration loading"),
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
            });
            Ok(())
        }
    }
}
