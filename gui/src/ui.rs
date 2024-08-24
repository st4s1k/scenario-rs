use egui::Ui;
use egui_file_dialog::FileDialog;
use std::path::PathBuf;

pub trait MyUi {
    fn labeled_field(
        &mut self,
        service_name_label: &str,
        service_name_value: &String,
    );
    fn mutable_labeled_field(
        &mut self,
        service_name_label: &str,
        service_name_value: &mut String,
    );
    fn file_selector_field(
        &mut self,
        label: &str,
        file_selector_field_value: Option<&PathBuf>,
        selector_button_label: &str,
        file_dialog: &mut FileDialog,
    );
    fn text_area(
        &mut self,
        text_area_label: &str,
        text_area: &mut String,
    );
}

impl MyUi for Ui {
    fn labeled_field(
        &mut self,
        service_name_label: &str,
        service_name_value: &String,
    ) {
        self.label(service_name_label);
        self.text_edit_singleline(&mut service_name_value.as_str());
        self.end_row();
    }

    fn mutable_labeled_field(
        &mut self,
        service_name_label: &str,
        service_name_value: &mut String,
    ) {
        self.label(service_name_label);
        self.text_edit_singleline(service_name_value);
        self.end_row();
    }

    fn file_selector_field(
        &mut self,
        label: &str,
        file_selector_field_value: Option<&PathBuf>,
        selector_button_label: &str,
        file_dialog: &mut FileDialog,
    ) {
        self.label(label);
        self.group(|ui| {
            let config_path_str =
                file_selector_field_value.map_or_else(
                    || String::from(""),
                    |p| p.file_name().map_or_else(
                        || String::from(""),
                        |f| f.to_string_lossy().to_string(),
                    ),
                );
            ui.text_edit_singleline(&mut config_path_str.clone());
            if ui.button(selector_button_label).clicked() {
                file_dialog.select_file();
            }
        });
        self.end_row();
    }

    fn text_area(
        &mut self,
        text_area_label: &str,
        text_area: &mut String,
    ) {
        self.label(text_area_label);
        self.add(
            egui::TextEdit::multiline(text_area)
                .font(egui::TextStyle::Monospace)
                .desired_width(f32::INFINITY)
                .desired_rows(10),
        );
    }
}
