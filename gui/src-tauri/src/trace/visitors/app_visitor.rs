use std::fmt::Debug;
use tracing::{
    error,
    field::{Field, Visit},
};

/// A visitor struct for tracing application events.
///
/// This struct collects event fields from tracing spans and events,
/// providing structured access to application-specific event properties.
///
/// # Examples
///
/// ```
/// use std::fmt::Debug;
/// use tracing::field::{Field, Visit};
/// use scenario_rs_gui::trace::visitors::AppEventVisitor;
///
/// // Create a new visitor
/// let mut visitor = AppEventVisitor::default();
///
/// // Record string field
/// visitor.record_str(&create_test_field("message"), "Clearing application logs");
///
/// // Access the collected field
/// assert_eq!(visitor.message.unwrap(), "Clearing application logs");
/// ```
///
/// # Helper function for examples
///
/// ```
/// # fn create_test_field(name: &str) -> tracing::field::Field {
/// #     struct TestCallsite();
/// #     impl tracing::callsite::Callsite for TestCallsite {
/// #         fn set_interest(&self, _: tracing::subscriber::Interest) {
/// #             unimplemented!()
/// #         }
/// #         fn metadata(&self) -> &tracing::Metadata<'_> {
/// #             &TEST_META
/// #         }
/// #     }
/// #     static TEST_CALLSITE: TestCallsite = TestCallsite();
/// #     static TEST_META: tracing::Metadata<'static> = tracing::metadata! {
/// #         name: "field_test",
/// #         target: module_path!(),
/// #         level: tracing::metadata::Level::INFO,
/// #         fields: &["message"],
/// #         callsite: &TEST_CALLSITE,
/// #         kind: tracing::metadata::Kind::SPAN,
/// #     };
/// #     tracing::field::AsField::as_field(name, &TEST_META).unwrap()
/// # }
/// ```
pub struct AppEventVisitor {
    /// Message content associated with the event
    pub message: Option<String>,
}

impl Default for AppEventVisitor {
    /// Creates a new empty visitor with all fields initialized to `None`.
    fn default() -> Self {
        AppEventVisitor { message: None }
    }
}

impl Visit for AppEventVisitor {
    /// Records string values from tracing events.
    ///
    /// This method processes string fields from tracing events and stores them
    /// in the appropriate field based on the field name.
    ///
    /// # Arguments
    ///
    /// * `field` - The field metadata containing the field name
    /// * `value` - The string value to record
    fn record_str(&mut self, field: &Field, value: &str) {
        match field.name() {
            "message" => self.message = Some(value.to_string()),
            _ => {
                error!("Unrecognized field: {}", field.name());
            }
        }
    }

    /// Records debug-formatted values from tracing events.
    ///
    /// This method processes fields that implement `Debug` and attempts to
    /// convert and store them in the appropriate field based on the field name.
    /// Currently only used for "message" fields.
    ///
    /// # Arguments
    ///
    /// * `field` - The field metadata containing the field name
    /// * `value` - The debug-formattable value to record
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        if field.name() == "message" {
            let value_str = format!("{:?}", value);
            self.message = Some(value_str.trim_matches('"').to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::trace::visitors::AppEventVisitor;
    use tracing::field::{Field, Visit};

    #[test]
    fn test_appvisitor_initialization_with_default() {
        // Given & When
        let visitor = AppEventVisitor::default();

        // Then
        assert!(visitor.message.is_none());
    }

    #[test]
    fn test_appvisitor_record_str_with_valid_fields() {
        // Given
        let mut visitor = AppEventVisitor::default();

        // When
        visitor.record_str(&field("message"), "Test message");
        visitor.record_str(&field("ignored_field"), "Should be ignored");

        // Then
        assert_eq!(visitor.message.unwrap(), "Test message");
    }

    #[test]
    fn test_appvisitor_record_str_with_empty_value() {
        // Given
        let mut visitor = AppEventVisitor::default();

        // When
        visitor.record_str(&field("message"), "");

        // Then
        assert_eq!(visitor.message.unwrap(), "");
    }

    // Test helpers
    fn field(name: &str) -> Field {
        struct TestCallsite();
        impl tracing::callsite::Callsite for TestCallsite {
            fn set_interest(&self, _: tracing::subscriber::Interest) {
                unimplemented!()
            }

            fn metadata(&self) -> &tracing::Metadata<'_> {
                &TEST_META
            }
        }
        static TEST_CALLSITE: TestCallsite = TestCallsite();
        static TEST_META: tracing::Metadata<'static> = tracing::metadata! {
            name: "field_test",
            target: module_path!(),
            level: tracing::metadata::Level::INFO,
            fields: &[
                "message",
                "ignored_field",
            ],
            callsite: &TEST_CALLSITE,
            kind: tracing::metadata::Kind::SPAN,
        };

        tracing::field::AsField::as_field(name, &TEST_META).unwrap()
    }
}
