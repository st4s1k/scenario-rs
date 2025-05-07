pub use app_layer::AppEventLayer;
pub use scenario_layer::ScenarioEventLayer;
use tracing::span::Record;
use tracing::{Event, Id, Subscriber};
use tracing_subscriber::{layer::Context, registry::LookupSpan};

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
    /// Processes a new span event.
    ///
    /// This method is called when a new span is created. Implementations
    /// should handle the span creation event according to their specific
    /// requirements.
    ///
    /// # Arguments
    ///
    /// * `_attrs` - A reference to the attributes of the new span
    /// * `_id` - The ID of the new span
    /// * `_ctx` - The context in which the span was created
    fn on_new_span<S>(&self, _attrs: &tracing::span::Attributes<'_>, _id: &Id, _ctx: Context<'_, S>)
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
    }

    /// Processes on record event.
    ///
    /// This method is called when a record is created. Implementations
    /// should handle the record creation event according to their specific
    /// requirements.
    ///
    /// # Arguments
    ///
    /// * `_id` - A reference to the ID of the record
    /// * `_record` - A reference to the record
    /// * `_ctx` - The context in which the record was created
    fn on_record<S>(&self, _id: &Id, _record: &Record<'_>, _ctx: Context<'_, S>)
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
    }

    /// Processes a tracing event.
    ///
    /// This method is called when a tracing event occurs. Implementations
    /// should extract relevant information from the event and handle it
    /// according to their specific requirements.
    ///
    /// # Arguments
    ///
    /// * `event` - A reference to the tracing event to process
    /// * `ctx` - The context in which the event occurred
    fn process_event<S>(&self, event: &Event<'_>, ctx: Context<'_, S>)
    where
        S: Subscriber + for<'a> LookupSpan<'a>;
}
