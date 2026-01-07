use crate::k8s::PodInfo;
use crate::views::common::*;
use egui::{Color32, RichText, Ui, ScrollArea};
use egui_extras::{Column, TableBuilder};

pub struct PodsView {
    pub search_filter: String,
    pub selected_pod: Option<PodInfo>,
    pub show_delete_dialog: bool,
    pub show_logs: bool,
    pub logs_content: String,
    pub logs_loading: bool,
    pub selected_container: Option<String>,
    pub tail_lines: i64,
    pub pending_action: Option<PodAction>,
}

#[derive(Clone)]
pub enum PodAction {
    Delete(String, String),
    GetLogs(String, String, Option<String>, i64),
}

impl Default for PodsView {
    fn default() -> Self {
        Self {
            search_filter: String::new(),
            selected_pod: None,
            show_delete_dialog: false,
            show_logs: false,
            logs_content: String::new(),
            logs_loading: false,
            selected_container: None,
            tail_lines: 100,
            pending_action: None,
        }
    }
}

impl PodsView {
    pub fn show(
        &mut self,
        ui: &mut Ui,
        pods: &[PodInfo],
        loading: bool,
        error: Option<&str>,
    ) -> Option<PodAction> {
        let mut action = None;

        ui.horizontal(|ui| {
            section_header(ui, "Pods");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                search_bar(ui, &mut self.search_filter, "Search pods...");
            });
        });

        if loading {
            loading_spinner(ui);
            return None;
        }

        if let Some(err) = error {
            error_label(ui, err);
            return None;
        }

        let filtered: Vec<_> = pods
            .iter()
            .filter(|p| {
                self.search_filter.is_empty()
                    || p.name.to_lowercase().contains(&self.search_filter.to_lowercase())
                    || p.namespace.to_lowercase().contains(&self.search_filter.to_lowercase())
                    || p.status.to_lowercase().contains(&self.search_filter.to_lowercase())
            })
            .collect();

        if filtered.is_empty() {
            empty_state(ui, "No pods found");
            return None;
        }

        let available_height = ui.available_height();

        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto().at_least(180.0)) // Name
            .column(Column::auto().at_least(100.0)) // Namespace
            .column(Column::auto().at_least(60.0))  // Ready
            .column(Column::auto().at_least(100.0)) // Status
            .column(Column::auto().at_least(70.0))  // Restarts
            .column(Column::auto().at_least(60.0))  // Age
            .column(Column::auto().at_least(120.0)) // Node
            .column(Column::remainder().at_least(150.0)) // Actions
            .min_scrolled_height(0.0)
            .max_scroll_height(available_height - 50.0)
            .header(25.0, |mut header| {
                header.col(|ui| { ui.strong("Name"); });
                header.col(|ui| { ui.strong("Namespace"); });
                header.col(|ui| { ui.strong("Ready"); });
                header.col(|ui| { ui.strong("Status"); });
                header.col(|ui| { ui.strong("Restarts"); });
                header.col(|ui| { ui.strong("Age"); });
                header.col(|ui| { ui.strong("Node"); });
                header.col(|ui| { ui.strong("Actions"); });
            })
            .body(|mut body| {
                for pod in &filtered {
                    body.row(30.0, |mut row| {
                        row.col(|ui| {
                            if ui.link(&pod.name).clicked() {
                                self.selected_pod = Some((*pod).clone());
                            }
                        });
                        row.col(|ui| { ui.label(&pod.namespace); });
                        row.col(|ui| { ui.label(&pod.ready); });
                        row.col(|ui| {
                            let color = status_color(&pod.status);
                            status_badge(ui, &pod.status, color);
                        });
                        row.col(|ui| {
                            let color = if pod.restarts > 0 {
                                Color32::from_rgb(234, 179, 8)
                            } else {
                                Color32::GRAY
                            };
                            ui.label(RichText::new(pod.restarts.to_string()).color(color));
                        });
                        row.col(|ui| { ui.label(&pod.age); });
                        row.col(|ui| { ui.label(&pod.node); });
                        row.col(|ui| {
                            ui.horizontal(|ui| {
                                if ui.small_button("Logs").clicked() {
                                    self.selected_pod = Some((*pod).clone());
                                    self.show_logs = true;
                                    self.logs_content.clear();
                                    self.selected_container = pod.containers.first().map(|c| c.name.clone());
                                    if let Some(container) = &self.selected_container {
                                        action = Some(PodAction::GetLogs(
                                            pod.namespace.clone(),
                                            pod.name.clone(),
                                            Some(container.clone()),
                                            self.tail_lines,
                                        ));
                                    }
                                }
                                if ui.small_button("Delete").clicked() {
                                    self.selected_pod = Some((*pod).clone());
                                    self.show_delete_dialog = true;
                                }
                            });
                        });
                    });
                }
            });

        // Logs window
        if self.show_logs {
            if let Some(pod) = &self.selected_pod {
                let mut open = true;
                egui::Window::new(format!("Logs - {}", pod.name))
                    .open(&mut open)
                    .resizable(true)
                    .default_size([800.0, 500.0])
                    .show(ui.ctx(), |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Container:");
                            egui::ComboBox::from_id_salt("container_select")
                                .selected_text(self.selected_container.as_deref().unwrap_or("Select..."))
                                .show_ui(ui, |ui| {
                                    for container in &pod.containers {
                                        if ui.selectable_label(
                                            self.selected_container.as_ref() == Some(&container.name),
                                            &container.name,
                                        ).clicked() {
                                            self.selected_container = Some(container.name.clone());
                                            action = Some(PodAction::GetLogs(
                                                pod.namespace.clone(),
                                                pod.name.clone(),
                                                Some(container.name.clone()),
                                                self.tail_lines,
                                            ));
                                        }
                                    }
                                });

                            ui.label("Tail lines:");
                            if ui.add(egui::DragValue::new(&mut self.tail_lines).range(10..=10000)).changed() {
                                if let Some(container) = &self.selected_container {
                                    action = Some(PodAction::GetLogs(
                                        pod.namespace.clone(),
                                        pod.name.clone(),
                                        Some(container.clone()),
                                        self.tail_lines,
                                    ));
                                }
                            }

                            if ui.button("Refresh").clicked() {
                                action = Some(PodAction::GetLogs(
                                    pod.namespace.clone(),
                                    pod.name.clone(),
                                    self.selected_container.clone(),
                                    self.tail_lines,
                                ));
                            }
                        });

                        ui.separator();

                        if self.logs_loading {
                            loading_spinner(ui);
                        } else {
                            ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::TextEdit::multiline(&mut self.logs_content.as_str())
                                            .font(egui::TextStyle::Monospace)
                                            .desired_width(f32::INFINITY)
                                    );
                                });
                        }
                    });

                if !open {
                    self.show_logs = false;
                    self.selected_pod = None;
                }
            }
        }

        // Delete dialog
        if self.show_delete_dialog {
            if let Some(pod) = &self.selected_pod {
                egui::Window::new("Confirm Delete")
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ui.ctx(), |ui| {
                        ui.label(format!("Are you sure you want to delete pod '{}'?", pod.name));
                        ui.add_space(16.0);
                        ui.horizontal(|ui| {
                            if ui.button("Cancel").clicked() {
                                self.show_delete_dialog = false;
                            }
                            if danger_button(ui, "Delete") {
                                action = Some(PodAction::Delete(
                                    pod.namespace.clone(),
                                    pod.name.clone(),
                                ));
                                self.show_delete_dialog = false;
                            }
                        });
                    });
            }
        }

        // Pod detail panel
        if let Some(pod) = self.selected_pod.clone() {
            if !self.show_logs && !self.show_delete_dialog {
                let mut close_details = false;
                egui::Window::new("Pod Details")
                    .resizable(true)
                    .default_width(450.0)
                    .show(ui.ctx(), |ui| {
                        if ui.button("Close").clicked() {
                            close_details = true;
                        }
                        ui.separator();

                        info_row(ui, "Name", &pod.name);
                        info_row(ui, "Namespace", &pod.namespace);
                        info_row(ui, "Status", &pod.status);
                        info_row(ui, "Ready", &pod.ready);
                        info_row(ui, "IP", &pod.ip);
                        info_row(ui, "Node", &pod.node);
                        info_row(ui, "Age", &pod.age);
                        info_row(ui, "Restarts", &pod.restarts.to_string());

                        ui.add_space(12.0);
                        ui.label(RichText::new("Containers:").strong());
                        ui.separator();

                        for container in &pod.containers {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    let color = status_color(&container.state);
                                    ui.colored_label(color, "●");
                                    ui.strong(&container.name);
                                });
                                info_row(ui, "Image", &container.image);
                                info_row(ui, "State", &container.state);
                                info_row(ui, "Ready", if container.ready { "Yes" } else { "No" });
                                info_row(ui, "Restarts", &container.restarts.to_string());
                            });
                            ui.add_space(4.0);
                        }
                    });
                if close_details {
                    self.selected_pod = None;
                }
            }
        }

        action
    }

    pub fn set_logs(&mut self, logs: String) {
        self.logs_content = logs;
        self.logs_loading = false;
    }

    pub fn set_logs_loading(&mut self) {
        self.logs_loading = true;
    }
}
