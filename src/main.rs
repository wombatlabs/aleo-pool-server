mod accounting;
mod api;
mod connection;
mod prover_peer;
mod server;

#[cfg(feature = "db")]
mod db;

use std::sync::Arc;

use clap::Parser;
use futures::stream::StreamExt;
use rand::seq::SliceRandom;
use signal_hook::consts::{SIGABRT, SIGHUP, SIGINT, SIGQUIT, SIGTERM, SIGTSTP, SIGUSR1};
use signal_hook_tokio::Signals;
use snarkvm::console::{network::MainnetV0, types::Address};
use tokio::sync::mpsc::Sender;
use tracing::{debug, error, info, warn};
use tracing_log::{log, LogTracer};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter};

use crate::prover_peer::Node;
use crate::{
    accounting::{Accounting, AccountingMessage},
    //    operator_peer::Node,
    server::{Server, ServerMessage},
};

pub(crate) type N = MainnetV0;

#[derive(Debug, Parser)]
#[clap(name = "pool_server", about = "Aleo proving pool server")]
struct Opt {
    /// snarkOS node address
    #[clap(short, long)]
    node: Option<String>,

    /// Proving pool address (aleo1...)
    #[clap(short, long)]
    address: Address<N>,

    /// Port to listen for incoming provers
    #[clap(short, long)]
    port: u16,

    /// API port
    #[clap(long = "api-port")]
    api_port: u16,

    /// AleoScan API URL root\n
    /// Mandatory if db feature is enabled, used to check if solution is on network
    #[clap(short, long = "aleoscan-url")]
    explorer_url: Option<String>,

    /// Genesis block path for testing
    #[clap(long)]
    genesis_block: Option<String>,

    /// Enable debug logging
    #[clap(short, long)]
    debug: bool,

    /// Enable trace logging
    #[clap(short, long)]
    trace: bool,

    /// Output log to file
    #[clap(long)]
    log: Option<String>,
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let opt = Opt::parse();
    let tracing_level = if opt.trace {
        tracing::Level::TRACE
    } else if opt.debug {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };
    let _ = LogTracer::init_with_filter(log::LevelFilter::Info);
    let filter = EnvFilter::from_default_env()
        .add_directive(tracing_level.into())
        .add_directive("hyper=info".parse().unwrap())
        .add_directive("warp=info".parse().unwrap())
        .add_directive("warp=warn".parse().unwrap())
        .add_directive("tokio_util=info".parse().unwrap())
        .add_directive("api".parse().unwrap());
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_env_filter(filter)
        .finish();
    // .with(
    //     tracing_subscriber::fmt::Layer::default()
    //         .with_ansi(true)
    //         .with_writer(std::io::stdout),
    // );
    if let Some(log) = opt.log {
        let file = std::fs::File::create(log).unwrap();
        let file = tracing_subscriber::fmt::layer().with_writer(file).with_ansi(false);
        tracing::subscriber::set_global_default(subscriber.with(file))
            .expect("unable to set global default subscriber");
    } else {
        tracing::subscriber::set_global_default(subscriber).expect("unable to set global default subscriber");
    }

    rayon::ThreadPoolBuilder::new()
        .stack_size(8 * 1024 * 1024)
        .num_threads(num_cpus::get())
        .build_global()
        .unwrap();

    let validator = opt.node.unwrap_or_else(|| {
        let bootstrap = [
            "node1.mainnet.aleoscan.org:4130",
            "node2.mainnet.aleoscan.org:4130",
            "node3.mainnet.aleoscan.org:4130",
        ];
        bootstrap.choose(&mut rand::thread_rng()).unwrap().to_string()
    });
    let port = opt.port;

    let address = opt.address;

    let accounting = Accounting::init(opt.explorer_url);

    let node = Node::init(validator);

    let server = Server::init(port, address, node.sender(), accounting.sender()).await;

    prover_peer::start(node, server.sender(), opt.genesis_block);

    api::start(opt.api_port, accounting.clone(), server.clone());

    match Signals::new([SIGABRT, SIGTERM, SIGHUP, SIGINT, SIGQUIT, SIGUSR1, SIGTSTP]) {
        Ok(signals) => {
            tokio::spawn(handle_signals(signals, accounting.clone(), server.sender()));
        }
        Err(err) => {
            error!("Unable to register signal handlers: {:?}", err);
            std::process::exit(1);
        }
    }

    std::future::pending::<()>().await;
}

async fn handle_signals(mut signals: Signals, accounting: Arc<Accounting>, server_sender: Sender<ServerMessage>) {
    while let Some(signal) = signals.next().await {
        info!("Received signal: {:?}", signal);
        let accounting_sender = accounting.sender();
        match signal {
            SIGABRT => {
                info!("Trying to salvage states before aborting...");
                let _ = accounting_sender.send(AccountingMessage::Exit).await;
                accounting.wait_for_exit().await;
                let _ = server_sender.send(ServerMessage::Exit).await;
                std::process::abort();
            }
            SIGTERM | SIGINT | SIGHUP | SIGQUIT => {
                info!("Saving states before exiting...");
                let _ = accounting_sender.send(AccountingMessage::Exit).await;
                accounting.wait_for_exit().await;
                let _ = server_sender.send(ServerMessage::Exit).await;
                std::process::exit(0);
            }
            SIGUSR1 => {
                debug!("Should do something useful here...");
            }
            SIGTSTP => {
                warn!("Suspending is not supported");
            }
            _ => unreachable!(),
        }
    }
}
