pub use app_layer::AppEventLayer;
pub use scenario_layer::ScenarioEventLayer;
use tracing::Event;

mod app_layer;
mod scenario_layer;

pub trait EventLayer {
    fn process_event(&self, event: &Event<'_>);
}
