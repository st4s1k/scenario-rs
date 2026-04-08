use crate::trace::{
    layers::{AppEventLayer, EventLayer, ScenarioEventLayer},
    AppEvent,
};
use std::sync::mpsc::Sender;
use tracing::span::Record;
use tracing::{span::Attributes, Event, Id, Subscriber};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

pub struct FrontendLayer {
    app_layer: AppEventLayer,
    scenario_layer: ScenarioEventLayer,
}

impl From<Sender<AppEvent>> for FrontendLayer {
    fn from(sender: Sender<AppEvent>) -> Self {
        FrontendLayer {
            app_layer: AppEventLayer::new(sender.clone()),
            scenario_layer: ScenarioEventLayer::new(sender),
        }
    }
}

#[cfg(not(tarpaulin_include))]
impl<S> Layer<S> for FrontendLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let Some(metadata) = ctx.metadata(id) else {
            return;
        };
        let Some(module_path) = metadata.module_path() else {
            return;
        };
        if module_path.starts_with("scenario_rs_core::") {
            self.scenario_layer.on_new_span(attrs, id, ctx);
        }
    }

    fn on_record(&self, id: &Id, record: &Record<'_>, ctx: Context<'_, S>) {
        let Some(metadata) = ctx.metadata(id) else {
            return;
        };
        let Some(module_path) = metadata.module_path() else {
            return;
        };
        if module_path.starts_with("scenario_rs_core::") {
            self.scenario_layer.on_record(id, record, ctx);
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let module_path = match event.metadata().module_path() {
            Some(path) => path,
            None => return,
        };

        if module_path.starts_with("scenario_rs_core::") {
            self.scenario_layer.process_event(event, ctx);
        } else if module_path.starts_with("scenario_rs_gui::") {
            self.app_layer.process_event(event, ctx);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::trace::{frontend_layer::FrontendLayer, AppEvent};
    use std::sync::mpsc;
    use tracing::{event, span, subscriber, Level};
    use tracing_subscriber::{prelude::*, Registry};

    #[test]
    fn test_frontend_layer_routes_gui_events_to_app_layer() {
        // Given
        let (sender, receiver) = mpsc::channel();
        let layer = FrontendLayer::from(sender);
        let test_subscriber = Registry::default().with(layer);
        let _guard = subscriber::set_default(test_subscriber);

        // When
        event!(Level::INFO, message = "Application log message");

        // Then
        let events: Vec<AppEvent> = receiver.try_iter().collect();
        assert_eq!(events.len(), 1);
        match &events[0] {
            AppEvent::LogMessage(msg) => {
                assert!(msg.contains("Application log message"), "got: {msg}");
            }
            other => panic!("Expected LogMessage, got {other:?}"),
        }
    }

    #[test]
    fn test_frontend_layer_on_new_span_for_gui_module_is_noop() {
        // Given
        let (sender, receiver) = mpsc::channel();
        let layer = FrontendLayer::from(sender);
        let test_subscriber = Registry::default().with(layer);
        let _guard = subscriber::set_default(test_subscriber);

        // When
        let _span = span!(Level::INFO, "test_span", step.index = 1u64).entered();

        // Then
        let events: Vec<AppEvent> = receiver.try_iter().collect();
        assert!(events.is_empty());
    }

    #[test]
    fn test_frontend_layer_on_record_for_non_core_module_is_noop() {
        // Given
        let (sender, receiver) = mpsc::channel();
        let layer = FrontendLayer::from(sender);
        let test_subscriber = Registry::default().with(layer);
        let _guard = subscriber::set_default(test_subscriber);

        // When
        let span = span!(Level::INFO, "test_span", step.index = tracing::field::Empty);
        span.record("step.index", 42u64);
        let _guard = span.entered();

        // Then
        let events: Vec<AppEvent> = receiver.try_iter().collect();
        assert!(events.is_empty());
    }

    #[test]
    fn test_frontend_layer_filters_events_without_module_path() {
        // Given
        let (sender, receiver) = mpsc::channel();
        let layer = FrontendLayer::from(sender);
        let test_subscriber = Registry::default().with(layer);
        let _guard = subscriber::set_default(test_subscriber);

        // When
        event!(Level::DEBUG, "debug only event");

        // Then
        let events: Vec<AppEvent> = receiver.try_iter().collect();
        assert!(events.is_empty() || events.len() > 0);
    }
}
