use crate::{
    app::ScenarioAppState,
    event::{EventChannel, EventHandler},
    utils::SafeLock,
};
use chrono::Local;
use scenario_rs::scenario::utils::SendEvent;
use std::{
    fmt::{Debug, Write},
    ops::{Deref, DerefMut},
    sync::{
        mpsc::{Receiver, Sender},
        Mutex,
    },
};
use tauri::{AppHandle, Emitter, Manager};
use tracing::{
    field::{Field, Visit},
    Event, Subscriber,
};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

struct LogEventHandler;

impl EventHandler<String> for LogEventHandler {
    fn is_terminal(&self, _event: &String) -> bool {
        false
    }

    fn handle(&self, message: &String, app_handle: &AppHandle) {
        let state = app_handle.state::<Mutex<ScenarioAppState>>();
        let mut state = state.safe_lock();
        let timestamp = Local::now().format("%H:%M:%S.%3f").to_string();
        state
            .output_log
            .push_str(&format!("[{timestamp}] {message}\n"));
        let _ = app_handle.emit("log-update", ());
    }
}

pub(crate) struct FrontendLogEventChannel;

impl FrontendLogEventChannel {
    pub fn init(rx: Receiver<String>, app_handle: &AppHandle) {
        EventChannel::listen(rx, app_handle, LogEventHandler);
    }
}

pub(crate) struct FrontendLogLayer(Sender<String>);

impl Deref for FrontendLogLayer {
    type Target = Sender<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Sender<String>> for FrontendLogLayer {
    fn from(sender: Sender<String>) -> Self {
        FrontendLogLayer(sender)
    }
}

impl<S> Layer<S> for FrontendLogLayer
where
    S: Subscriber,
    S: for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut message = String::new();
        let mut visitor = MessageVisitor(&mut message);
        event.record(&mut visitor);
        self.send_event(message);
    }
}

struct MessageVisitor<'a>(&'a mut String);

impl Deref for MessageVisitor<'_> {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl DerefMut for MessageVisitor<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

impl<'a> Visit for MessageVisitor<'a> {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        if field.name() == "message" {
            let _ = write!(self.0, "{:?}", value);
        }
    }
}
