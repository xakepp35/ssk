use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(version, about = "Syrup Space Kadet — procedural space arcade")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    #[arg(long, global = true, default_value_t = 42)]
    seed: u64,
    #[arg(long, global = true, value_enum, default_value_t = Class::Cadet)]
    class: Class,
    #[arg(long, global = true)]
    config: Option<std::path::PathBuf>,
}

#[derive(Subcommand)]
enum Command {
    Play,
    Online {
        #[arg(long)]
        connect: Option<String>,
    },
    Node,
    InitConfig,
    Bench,
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
            ssk_net::Config::write_default(cli.config.as_deref())?.display()
        );
        return Ok(());
    }
    let network = ssk_net::Config::load(cli.config.as_deref())?;
    match cli.command {
        Some(Command::Node) => ssk_net::run_node(network),
        Some(Command::InitConfig | Command::Bench) => {
            anyhow::bail!("Available in the network phase")
        }
        command => {
            let online = matches!(command, Some(Command::Online { .. }));
            let play = command.is_some();
            let connect = match command {
                Some(Command::Online { connect }) => connect,
                _ => None,
            };
            let class = match cli.class {
                Class::Spark => ssk_sim::ShipClass::Spark,
                Class::Cadet => ssk_sim::ShipClass::Cadet,
                Class::Thunder => ssk_sim::ShipClass::Thunder,
            };
            ssk_client::launch(ssk_client::Options {
                seed: cli.seed,
                class,
                play,
                online,
                connect,
                network,
            });
            Ok(())
        }
    }
}
