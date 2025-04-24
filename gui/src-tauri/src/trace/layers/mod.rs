pub use app_layer::AppEventLayer;
pub use scenario_layer::ScenarioEventLayer;
use tracing::Event;

mod app_layer;
mod scenario_layer;

/// A trait for processing tracing events in different contexts.
///
/// Implementors of this trait can process `tracing::Event` instances and
/// take appropriate actions (such as logging, sending notifications, or
/// collecting metrics) based on the event contents.
///
/// This trait is the foundation for the application's event processing
/// pipeline, allowing different components to handle tracing events in
/// their own specialized ways.
///
/// # Examples
///
/// ```
/// use tracing::Event;
/// use scenario_rs_gui::trace::layers::EventLayer;
///
/// struct SimpleLogLayer;
///
/// impl EventLayer for SimpleLogLayer {
///     fn process_event(&self, event: &Event<'_>) {
///         // Process the event, for example by logging it
///         println!("Event received: {:?}", event.metadata().name());
///     }
/// }
///
/// // The layer can now be used to process tracing events
/// ```
pub trait EventLayer {
    /// Processes a tracing event.
    ///
    /// This method is called when a tracing event occurs. Implementations
    /// should extract relevant information from the event and handle it
    /// according to their specific requirements.
    ///
    /// # Arguments
    ///
    /// * `event` - A reference to the tracing event to process
    fn process_event(&self, event: &Event<'_>);
}
