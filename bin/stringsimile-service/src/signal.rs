//! OS signal handling for stringsimile service.

use tokio::{
    runtime::Runtime,
    sync::broadcast::{self, Receiver, Sender},
};
use tokio_stream::{Stream, StreamExt};
use tracing::{error, info};

#[derive(Debug, Clone)]
pub enum ServiceSignal {
    /// Signal to reload service configuration
    ReloadConfig,
    /// Signal to gracefully shutdown the process.
    Shutdown,
    /// Signal to exit the process immediately.
    Quit,
}

/// Convenience struct for OS signal handling
pub struct ServiceOsSignals {
    pub handler: SignalHandler,
    pub receiver: Receiver<ServiceSignal>,
}

impl ServiceOsSignals {
    /// Create a new OS signals container and configure it to handle signals
    pub fn new(runtime: &Runtime) -> Self {
        let (handler, receiver) = SignalHandler::new();
        let signals = os_signals_stream(runtime);
        handler.handle_signal_stream(runtime, signals);
        Self { handler, receiver }
    }
}

/// SignalBroadcastHandler is a general signal receiver and transmitter.
/// It ensures that signals are passed further to listeners.
pub struct SignalHandler {
    tx: Sender<ServiceSignal>,
}

impl SignalHandler {
    /// Create a new signal handler with space for 128 control messages at a time, to
    /// ensure the channel doesn't overflow and drop signals.
    fn new() -> (Self, Receiver<ServiceSignal>) {
        let (tx, rx) = broadcast::channel(128);
        let handler = Self { tx };

        (handler, rx)
    }

    /// Subscribe to the stream, and return a new receiver.
    pub fn subscribe(&self) -> Receiver<ServiceSignal> {
        self.tx.subscribe()
    }

    /// Takes a stream of elements are convertible to `ServiceSignal`, and spawns a permanent
    /// task for transmitting to the receiver.
    fn handle_signal_stream<T, S>(&self, runtime: &Runtime, stream: S)
    where
        T: Into<ServiceSignal> + Send + Sync,
        S: Stream<Item = T> + 'static + Send,
    {
        let tx = self.tx.clone();

        runtime.spawn(async move {
            tokio::pin!(stream);

            while let Some(value) = stream.next().await {
                if tx.send(value.into()).is_err() {
                    error!(message = "Couldn't send signal.");
                    break;
                }
            }
        });
    }
}

/// Collect OS signals into a stream of service related signals
fn os_signals_stream(runtime: &Runtime) -> impl Stream<Item = ServiceSignal> + use<> {
    use tokio::signal::unix::{SignalKind, signal};

    runtime.block_on(async {
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to set up SIGINT handler.");
        let mut sigterm =
            signal(SignalKind::terminate()).expect("Failed to set up SIGTERM handler.");
        let mut sigquit = signal(SignalKind::quit()).expect("Failed to set up SIGQUIT handler.");
        let mut sighup = signal(SignalKind::hangup()).expect("Failed to set up SIGHUP handler.");

        async_stream::stream! {
            loop {
                let signal = tokio::select! {
                    _ = sigint.recv() => {
                        info!(message = "Signal received.", signal = "SIGINT");
                        ServiceSignal::Shutdown
                    },
                    _ = sigterm.recv() => {
                        info!(message = "Signal received.", signal = "SIGTERM");
                        ServiceSignal::Shutdown
                    } ,
                    _ = sigquit.recv() => {
                        info!(message = "Signal received.", signal = "SIGQUIT");
                        ServiceSignal::Quit
                    },
                    _ = sighup.recv() => {
                        info!(message = "Signal received.", signal = "SIGHUP");
                        ServiceSignal::ReloadConfig
                    },
                };
                yield signal;
            }
        }
    })
}
