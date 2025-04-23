#![allow(missing_docs)]
use std::{process::ExitStatus, time::Duration};

use clap::Parser;
use exitcode::ExitCode;
use tokio::runtime::Runtime;
use tokio::sync::broadcast::Receiver;
use tokio::time::sleep;
use tracing::{Level, info, warn};

use crate::cli::CliArgs;
use crate::processor::StringProcessor;
use crate::signal::{ServiceOsSignals, ServiceSignal};

use std::os::unix::process::ExitStatusExt;
use tokio::runtime::Handle;

// TODO: add configuration options
pub struct GlobalConfig {}

pub struct Service<T> {
    pub config: GlobalConfig,
    pub state: T,
}

pub struct InitState {
    pub signals: ServiceOsSignals,
    pub processor: StringProcessor,
}

impl Service<()> {
    pub fn init_and_run() -> ExitStatus {
        Service::<InitState>::run()
    }
}

impl Service<InitState> {
    pub fn run() -> ExitStatus {
        let (runtime, app) = Self::prepare_start().unwrap_or_else(|code| std::process::exit(code));

        runtime.block_on(app.run())
    }

    pub fn prepare_start() -> Result<(Runtime, Service<StartedState>), ExitCode> {
        Self::prepare()
            .and_then(|(runtime, app)| app.start(runtime.handle()).map(|app| (runtime, app)))
    }

    pub fn prepare() -> Result<(Runtime, Self), ExitCode> {
        let args = CliArgs::try_parse().map_err(|error| {
            _ = error.print();
            exitcode::USAGE
        })?;

        Self::prepare_from_opts(args)
    }

    pub fn prepare_from_opts(args: CliArgs) -> Result<(Runtime, Self), ExitCode> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Building async runtime failed!");

        tracing_subscriber::fmt()
            .with_max_level(if args.verbose {
                Level::DEBUG
            } else {
                Level::INFO
            })
            .init();

        let signals = ServiceOsSignals::new(&runtime);
        let config = GlobalConfig {};
        let processor = StringProcessor::from_args(args);
        Ok((
            runtime,
            Self {
                config,
                state: InitState { signals, processor },
            },
        ))
    }

    pub fn start(self, handle: &Handle) -> Result<Service<StartedState>, ExitCode> {
        let Self {
            config,
            state: InitState { signals, processor },
        } = self;

        handle.spawn(processor.run(signals.handler.subscribe()));

        Ok(Service {
            config,
            state: StartedState { signals },
        })
    }
}

pub struct StartedState {
    pub signals: ServiceOsSignals,
}

impl Service<StartedState> {
    pub async fn run(self) -> ExitStatus {
        self.main().await.shutdown().await
    }

    pub async fn main(self) -> Service<FinishedState> {
        let Service {
            config,
            state: StartedState { signals },
        } = self;

        let mut signal_rx = signals.receiver;

        let signal = loop {
            tokio::select! {
                signal = signal_rx.recv() => {
                    info!(message = "Handling signal", signal = ?signal);
                    match signal.expect("Receiving OS signal failed") {
                        ServiceSignal::ReloadConfig => warn!(message = "SIHGUP handler is not implemented"),
                        signal @ ServiceSignal::Shutdown | signal @ ServiceSignal::Quit => break signal,
                    }
                }
                else => unreachable!("Signal streams never end"),
            }
        };

        Service {
            config,
            state: FinishedState {
                signal,
                signal_receiver: signal_rx,
            },
        }
    }
}

pub struct FinishedState {
    pub signal: ServiceSignal,
    pub signal_receiver: Receiver<ServiceSignal>,
}

impl Service<FinishedState> {
    pub async fn shutdown(self) -> ExitStatus {
        let Service {
            config: _config,
            state:
                FinishedState {
                    signal,
                    signal_receiver,
                },
        } = self;

        match signal {
            ServiceSignal::Shutdown => Self::stop(signal_receiver).await,
            ServiceSignal::Quit => Self::quit(),
            _ => unreachable!(),
        }
    }

    async fn stop(mut signal_rx: Receiver<ServiceSignal>) -> ExitStatus {
        tokio::select! {
            // TODO: replace with active tasks graceful shutdown
            _ = sleep(Duration::from_secs(1)) => ExitStatus::from_raw({
                exitcode::OK
            }), // Graceful shutdown finished
            _ = signal_rx.recv() => Self::quit(),
        }
    }

    fn quit() -> ExitStatus {
        ExitStatus::from_raw(exitcode::OK)
    }
}
