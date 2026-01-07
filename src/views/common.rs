use egui::{Color32, RichText, Ui, Vec2};

pub fn status_badge(ui: &mut Ui, status: &str, color: Color32) {
    ui.horizontal(|ui| {
        ui.add_space(4.0);
        let rect = ui.available_rect_before_wrap();
        let painter = ui.painter();
        let circle_center = egui::pos2(rect.min.x + 6.0, rect.center().y);
        painter.circle_filled(circle_center, 4.0, color);
        ui.add_space(12.0);
        ui.label(status);
    });
}

pub fn status_color(status: &str) -> Color32 {
    match status.to_lowercase().as_str() {
        "running" | "active" | "ready" | "succeeded" | "available" => Color32::from_rgb(34, 197, 94),
        "pending" | "waiting" | "creating" => Color32::from_rgb(234, 179, 8),
        "failed" | "error" | "crashloopbackoff" | "imagepullbackoff" => Color32::from_rgb(239, 68, 68),
        "terminating" | "terminated" => Color32::from_rgb(156, 163, 175),
        _ => Color32::from_rgb(156, 163, 175),
    }
}

pub fn section_header(ui: &mut Ui, title: &str) {
    ui.add_space(8.0);
    ui.heading(RichText::new(title).strong());
    ui.separator();
    ui.add_space(4.0);
}

pub fn info_row(ui: &mut Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("{}:", label)).strong());
        ui.label(value);
    });
}

pub fn action_button(ui: &mut Ui, text: &str, color: Color32) -> bool {
    let button = egui::Button::new(RichText::new(text).color(Color32::WHITE))
        .fill(color)
        .min_size(Vec2::new(80.0, 24.0));
    ui.add(button).clicked()
}

pub fn danger_button(ui: &mut Ui, text: &str) -> bool {
    action_button(ui, text, Color32::from_rgb(220, 38, 38))
}

pub fn primary_button(ui: &mut Ui, text: &str) -> bool {
    action_button(ui, text, Color32::from_rgb(59, 130, 246))
}

pub fn success_button(ui: &mut Ui, text: &str) -> bool {
    action_button(ui, text, Color32::from_rgb(34, 197, 94))
}

pub fn warning_button(ui: &mut Ui, text: &str) -> bool {
    action_button(ui, text, Color32::from_rgb(234, 179, 8))
}

pub fn loading_spinner(ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.spinner();
        ui.label("Loading...");
    });
}

pub fn error_label(ui: &mut Ui, error: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Error: ").color(Color32::from_rgb(239, 68, 68)).strong());
        ui.label(RichText::new(error).color(Color32::from_rgb(239, 68, 68)));
    });
}

pub fn empty_state(ui: &mut Ui, message: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(40.0);
        ui.label(RichText::new(message).size(16.0).color(Color32::GRAY));
        ui.add_space(40.0);
    });
}

pub fn confirm_dialog(ui: &mut Ui, title: &str, message: &str, confirm_text: &str) -> Option<bool> {
    let mut result = None;

    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ui.ctx(), |ui| {
            ui.label(message);
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    result = Some(false);
                }
                ui.add_space(8.0);
                if danger_button(ui, confirm_text) {
                    result = Some(true);
                }
            });
        });

    result
}

pub fn search_bar(ui: &mut Ui, search_text: &mut String, placeholder: &str) -> bool {
    let response = ui.add(
        egui::TextEdit::singleline(search_text)
            .hint_text(placeholder)
            .desired_width(200.0)
    );
    response.changed()
}

pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}
