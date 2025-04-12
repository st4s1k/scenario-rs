pub use app_layer::AppEventLayer;
pub use scenario::ScenarioEventLayer;
use tracing::Event;

mod app_layer;
mod scenario;

pub trait EventLayer {
    fn process_event(&self, event: &Event<'_>);
}
