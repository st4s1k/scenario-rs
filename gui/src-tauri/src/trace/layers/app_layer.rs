use crate::{
    trace::{layers::EventLayer, visitors::AppEventVisitor, AppEvent},
    utils::SendEvent,
};
use std::sync::mpsc::Sender;
use tracing::Event;
use tracing_subscriber::layer::Context;

/// A tracing layer that processes application events and forwards them as `AppEvent`s.
///
/// This layer captures application-specific events and converts them into messages
/// that can be sent to the frontend.
///
/// # Examples
///
/// ```
/// use std::sync::mpsc;
/// use scenario_rs_gui::trace::layers::AppEventLayer;
/// use scenario_rs_gui::trace::AppEvent;
/// use tracing::{event, Level};
///
/// // Create a channel for sending events
/// let (sender, receiver) = mpsc::channel();
///
/// // Create a new AppEventLayer
/// let layer = AppEventLayer::new(sender);
///
/// // The layer will process tracing events and convert them to AppEvents
/// // For example:
/// event!(Level::INFO, "Status update");
/// ```
pub struct AppEventLayer {
    pub sender: Sender<AppEvent>,
}

impl AppEventLayer {
    /// Creates a new `AppEventLayer` with the provided channel sender.
    ///
    /// # Arguments
    ///
    /// * `sender` - A channel sender for `AppEvent` messages
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::mpsc;
    /// use scenario_rs_gui::trace::layers::AppEventLayer;
    /// use scenario_rs_gui::trace::AppEvent;
    ///
    /// let (sender, _receiver) = mpsc::channel();
    /// let layer = AppEventLayer::new(sender);
    /// ```
    pub fn new(sender: Sender<AppEvent>) -> Self {
        Self { sender }
    }

    /// Sends a log message as an `AppEvent`.
    ///
    /// This method wraps the message in an `AppEvent::LogMessage` variant
    /// and sends it through the configured channel.
    ///
    /// # Arguments
    ///
    /// * `message` - The message string to send
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::sync::mpsc;
    /// # use scenario_rs_gui::trace::layers::AppEventLayer;
    /// # use scenario_rs_gui::trace::AppEvent;
    /// #
    /// # let (sender, receiver) = mpsc::channel();
    /// # let layer = AppEventLayer::new(sender);
    /// #
    /// layer.send_event("System initialized".to_string());
    ///
    /// // The receiver would get an AppEvent::LogMessage
    /// let event = receiver.try_recv().unwrap();
    /// assert!(matches!(event, AppEvent::LogMessage(_)));
    /// ```
    fn send_event(&self, message: String) {
        self.sender.send_event(AppEvent::LogMessage(message));
    }
}

impl EventLayer for AppEventLayer {
    /// Processes a tracing event and potentially forwards it as an `AppEvent`.
    fn process_event<S>(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = AppEventVisitor::default();

        event.record(&mut visitor);

        const APP_PREFIX: &str = "[APP] ";

        if let Some(message) = &visitor.message {
            self.send_event(format!("{}{}", APP_PREFIX, message));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::trace::{
        layers::{app_layer::AppEventLayer, EventLayer},
        AppEvent,
    };
    use std::sync::mpsc;
    use tracing::{event, subscriber, Event, Level, Subscriber};
    use tracing_subscriber::{layer::Context, prelude::*, registry::LookupSpan, Layer, Registry};

    #[test]
    fn test_appelayer_initialization_with_new() {
        // Given
        let (sender, _receiver) = mpsc::channel();

        // When & Then
        let _layer = AppEventLayer::new(sender);
    }

    #[test]
    fn test_appelayer_send_event_with_message() {
        // Given
        let (sender, receiver) = mpsc::channel();
        let layer = AppEventLayer::new(sender);

        // When
        layer.send_event("Hello".into());

        // Then
        match receiver.try_recv() {
            Ok(AppEvent::LogMessage(msg)) => assert_eq!(msg, "Hello"),
            other => panic!("Unexpected event: {other:?}"),
        }
    }

    #[test]
    fn test_appelayer_process_event_with_generic_message() {
        // Given
        let (sender, receiver) = mpsc::channel();
        let layer = TestAppEventLayer(AppEventLayer::new(sender));
        let test_subscriber = Registry::default().with(layer);
        let _guard = subscriber::set_default(test_subscriber);

        // When
        event!(Level::INFO, message = "Hello world");

        // Then
        match receiver.try_recv() {
            Ok(AppEvent::LogMessage(msg)) => assert_eq!(msg, "[APP] Hello world"),
            other => panic!("Expected LogMessage, got {other:?}"),
        }
    }

    // Test helpers
    struct TestAppEventLayer(AppEventLayer);

    impl<S> Layer<S> for TestAppEventLayer
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
            self.0.process_event(event, ctx);
        }
    }
}
