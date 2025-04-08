pub use app::AppEventLayer;
pub use scenario::ScenarioEventLayer;
use tracing::Event;

mod app;
mod scenario;

pub trait EventLayer {
    fn process_event(&self, event: &Event<'_>);
}
