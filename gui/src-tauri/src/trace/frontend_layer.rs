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
