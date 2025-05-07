#![allow(missing_docs)]
use std::{process::ExitStatus, time::Duration};

use clap::Parser;
use exitcode::ExitCode;
use metrics_exporter_prometheus::PrometheusBuilder;
use tokio::runtime::Runtime;
use tokio::sync::broadcast::Receiver;
use tokio::time::sleep;
use tracing::{Level, error, info};
use tracing_subscriber::fmt::format::FmtSpan;

use crate::cli::CliArgs;
use crate::config::ServiceConfig;
use crate::metrics::MetricsProcessor;
use crate::processor::StringProcessor;
use crate::signal::{ServiceOsSignals, ServiceSignal};

use std::os::unix::process::ExitStatusExt;
use tokio::runtime::Handle;

pub struct Service<T> {
    pub config: ServiceConfig,
    pub state: T,
}

pub struct InitState {
    pub signals: ServiceOsSignals,
    pub processor: StringProcessor,
    pub metrics_processor: MetricsProcessor,
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

        Self::prepare_from_config(args.try_into().map_err(|err| {
            // The tracing subscriber is never initialized before this
            tracing_subscriber::fmt()
                .with_file(false)
                .with_target(false)
                .with_max_level(Level::INFO)
                .init();
            error!(message = "Configuration error.", error = %err);
            exitcode::USAGE
        })?)
    }

    pub fn prepare_from_config(config: ServiceConfig) -> Result<(Runtime, Self), ExitCode> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(config.process.threads)
            .enable_all()
            .build()
            .expect("Building async runtime failed!");

        tracing_subscriber::fmt()
            .with_file(false)
            .with_target(false)
            .with_max_level(config.process.log_level)
            .with_span_events(FmtSpan::FULL)
            .init();

        let metrics_recorder = PrometheusBuilder::new().build_recorder();
        let metrics_handle = metrics_recorder.handle();
        metrics::set_global_recorder(metrics_recorder).expect("Failed preparing metrics recorder!");

        let signals = ServiceOsSignals::new(&runtime);
        let processor = StringProcessor::from_config(config.clone());
        let metrics_processor = MetricsProcessor::from_config(config.clone(), metrics_handle);
        Ok((
            runtime,
            Self {
                config,
                state: InitState {
                    signals,
                    processor,
                    metrics_processor,
                },
            },
        ))
    }

    pub fn start(self, handle: &Handle) -> Result<Service<StartedState>, ExitCode> {
        let Self {
            config,
            state:
                InitState {
                    signals,
                    processor,
                    metrics_processor,
                },
        } = self;

        handle.spawn(processor.run(signals.handler.subscribe()));
        handle.spawn(metrics_processor.run(signals.handler.subscribe()));

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
                    match signal{
                        Ok(ServiceSignal::ReloadConfig) => (),
                        Ok(signal @ ServiceSignal::Shutdown | signal @ ServiceSignal::Quit) => break signal,
                        Err(err) => {
                            error!(message = "Receiving OS signal failed!", error = %err);
                        }
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
