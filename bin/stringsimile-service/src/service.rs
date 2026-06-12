#![allow(missing_docs)]
use std::io;
use std::process::ExitStatus;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use clap::Parser;
use exitcode::ExitCode;
use metrics::{Unit, describe_gauge, gauge};
use metrics_exporter_prometheus::PrometheusBuilder;
use metrics_util::layers::{PrefixLayer, Stack};
use tokio::runtime::{Handle, Runtime};
use tokio::sync::broadcast::{Receiver, Sender};
use tokio::task::JoinHandle;
use tokio::time::interval;
use tracing::{Level, error, info, warn};
use tracing_subscriber::fmt::format::FmtSpan;

use crate::cli::CliArgs;
use crate::config::ServiceConfig;
use crate::metrics::MetricsProcessor;
use crate::processor::StringProcessor;
use crate::signal::{ServiceOsSignals, ServiceSignal};

use std::os::unix::process::ExitStatusExt;

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
        Service::<InitState>::run(false)
    }

    pub fn reload() -> ExitStatus {
        Service::<InitState>::run(true)
    }
}

impl Service<InitState> {
    pub fn run(reload: bool) -> ExitStatus {
        let (runtime, app) =
            Self::prepare_start(reload).unwrap_or_else(|code| std::process::exit(code));

        let res = runtime.block_on(app.run());
        if let Some(exit) = res {
            return exit;
        }

        // Restart path
        Service::reload()
    }

    pub fn prepare_start(reload: bool) -> Result<(Runtime, Service<StartedState>), ExitCode> {
        Self::prepare(reload)
            .and_then(|(runtime, app)| app.start(runtime.handle()).map(|app| (runtime, app)))
    }

    pub fn prepare(reload: bool) -> Result<(Runtime, Self), ExitCode> {
        let args = CliArgs::try_parse().map_err(|error| {
            _ = error.print();
            error.exit_code()
        })?;

        Self::prepare_from_config(
            args.try_into().map_err(|err| {
                if !reload {
                    // The tracing subscriber is never initialized before this
                    tracing_subscriber::fmt()
                        .with_file(false)
                        .with_target(false)
                        .with_max_level(Level::INFO)
                        .with_writer(io::stderr)
                        .init();
                }
                error!(message = "Configuration error.", error = %err);
                exitcode::USAGE
            })?,
            reload,
        )
    }

    pub fn prepare_from_config(
        config: ServiceConfig,
        reload: bool,
    ) -> Result<(Runtime, Self), ExitCode> {
        let init_time = Instant::now();

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(config.process.threads)
            .enable_all()
            .build()
            .expect("Building async runtime failed!");

        if !reload {
            tracing_subscriber::fmt()
                .with_file(false)
                .with_target(false)
                .with_max_level(config.process.log_level)
                .with_span_events(FmtSpan::FULL)
                .with_writer(io::stderr)
                .init();
        }

        let metrics_recorder = PrometheusBuilder::new().build_recorder();
        let metrics_handle = metrics_recorder.handle();

        if !reload {
            let layers = Stack::new(metrics_recorder);

            let metrics_prefix = config.metrics.prefix.clone();
            if !metrics_prefix.is_empty() {
                layers
                    .push(PrefixLayer::new(metrics_prefix))
                    .install()
                    .expect("Failed preparing metrics recorder!");
            } else {
                layers
                    .install()
                    .expect("Failed preparing metrics recorder!");
            }
        }

        let signals = ServiceOsSignals::new(&runtime);
        let processor = StringProcessor::from_config(config.clone());
        let metrics_processor =
            MetricsProcessor::from_config(config.clone(), metrics_handle, init_time);
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

        let (shutdown_tx, _) = tokio::sync::broadcast::channel::<()>(1);

        let processor_handle =
            handle.spawn(processor.run(signals.handler.subscribe(), shutdown_tx.subscribe()));
        let metrics_handle =
            handle.spawn(metrics_processor.run(handle.clone(), shutdown_tx.subscribe()));

        Ok(Service {
            config,
            state: StartedState {
                signals,
                processor_handle,
                metrics_handle,
                shutdown_tx,
            },
        })
    }
}

pub struct StartedState {
    pub signals: ServiceOsSignals,
    pub processor_handle: JoinHandle<()>,
    pub metrics_handle: JoinHandle<()>,
    pub shutdown_tx: Sender<()>,
}

impl Service<StartedState> {
    /// Runs the service to completion (exit or restart)
    /// If the service should be restarted, None is returned
    pub async fn run(self) -> Option<ExitStatus> {
        self.main().await.shutdown().await
    }

    pub async fn main(self) -> Service<FinishedState> {
        let Service {
            config,
            state:
                StartedState {
                    signals,
                    mut processor_handle,
                    metrics_handle,
                    shutdown_tx,
                },
        } = self;

        let mut signal_rx = signals.receiver;

        describe_gauge!(
            "last_reload_signal",
            Unit::Seconds,
            "Timestamp of last reload signal received"
        );
        let last_reload_signal = gauge!("last_reload_signal");

        let signal = loop {
            tokio::select! {
                signal = signal_rx.recv() => {
                    info!(message = "Handling signal", signal = ?signal);
                    match signal{
                        Ok(ServiceSignal::ReloadConfig) => {
                            // Other components will handle config reload
                            // Here we will just emit the metric
                            let now = SystemTime::now();
                            // Ignoring error here, since there is nothing meaningful to be done
                            if let Ok(epoch_timestamp) = now.duration_since(UNIX_EPOCH) {
                                last_reload_signal.set(epoch_timestamp.as_secs_f64())
                            }

                            if config.process.enable_config_reload {
                                // First validate the configuration

                                if let Err(err) = ServiceConfig::try_from(CliArgs::parse()) {
                                    warn!(message = "Invalid configuration, aborting config reload...", error = %err);
                                    continue;
                                }

                                let _ = shutdown_tx.send(());
                                info!("Starting graceful shutdown for config reload. ({} ms)", config.process.shutdown_timeout.as_millis());
                                break ServiceSignal::ReloadConfig;
                            }
                        },
                        Ok(ServiceSignal::Shutdown) => {
                            let _ = shutdown_tx.send(());
                            info!("Starting graceful shutdown. ({} ms)", config.process.shutdown_timeout.as_millis());
                            break ServiceSignal::Shutdown;
                        }
                        Ok(ServiceSignal::Quit) => {
                            break ServiceSignal::Quit;
                        }
                        Err(err) => {
                            error!(message = "Receiving OS signal failed!", error = %err);
                        }
                    }
                }

                _ = &mut processor_handle => {
                    break ServiceSignal::Shutdown;
                }

                else => unreachable!("Signal streams never end"),
            }
        };

        Service {
            config,
            state: FinishedState {
                signal,
                signal_receiver: signal_rx,
                processor_handle,
                metrics_handle,
            },
        }
    }
}

pub struct FinishedState {
    pub signal: ServiceSignal,
    pub signal_receiver: Receiver<ServiceSignal>,
    pub processor_handle: JoinHandle<()>,
    pub metrics_handle: JoinHandle<()>,
}

impl Service<FinishedState> {
    pub async fn shutdown(self) -> Option<ExitStatus> {
        let Service {
            config,
            state:
                FinishedState {
                    signal,
                    signal_receiver,
                    processor_handle,
                    metrics_handle,
                },
        } = self;

        match signal {
            ServiceSignal::Shutdown => {
                Some(Self::stop(config, processor_handle, metrics_handle, signal_receiver).await)
            }
            ServiceSignal::ReloadConfig => {
                Self::stop(config, processor_handle, metrics_handle, signal_receiver).await;
                None
            }
            ServiceSignal::Quit => Some(Self::quit()),
        }
    }

    async fn stop(
        config: ServiceConfig,
        processor_handle: JoinHandle<()>,
        metrics_handle: JoinHandle<()>,
        mut signal_rx: Receiver<ServiceSignal>,
    ) -> ExitStatus {
        let mut graceful_timeout = interval(config.process.shutdown_timeout);
        graceful_timeout.reset();

        if processor_handle.is_finished() {
            metrics_handle.abort();
            let _ = metrics_handle.await;
            return ExitStatus::from_raw(exitcode::OK);
        }

        tokio::select! {
            _ = processor_handle, if !processor_handle.is_finished() => {
                metrics_handle.abort();
                let _ = metrics_handle.await;
                ExitStatus::from_raw(exitcode::OK)
            }

            _ = graceful_timeout.tick() => {
                warn!("Graceful shutdown timed out. Forcing shutdown.");
                ExitStatus::from_raw(exitcode::OK)
            }

            _ = signal_rx.recv() => Self::quit(),
        }
    }

    fn quit() -> ExitStatus {
        ExitStatus::from_raw(exitcode::OK)
    }
}
