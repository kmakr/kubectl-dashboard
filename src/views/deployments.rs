use crate::k8s::DeploymentInfo;
use crate::views::common::*;
use egui::{Color32, RichText, Ui};
use egui_extras::{Column, TableBuilder};

pub struct DeploymentsView {
    pub search_filter: String,
    pub selected_deployment: Option<DeploymentInfo>,
    pub scale_replicas: i32,
    pub show_scale_dialog: bool,
    pub show_delete_dialog: bool,
    pub pending_action: Option<DeploymentAction>,
}

#[derive(Clone)]
pub enum DeploymentAction {
    Scale(String, String, i32),
    Restart(String, String),
    Delete(String, String),
}

impl Default for DeploymentsView {
    fn default() -> Self {
        Self {
            search_filter: String::new(),
            selected_deployment: None,
            scale_replicas: 1,
            show_scale_dialog: false,
            show_delete_dialog: false,
            pending_action: None,
        }
    }
}

impl DeploymentsView {
    pub fn show(
        &mut self,
        ui: &mut Ui,
        deployments: &[DeploymentInfo],
        loading: bool,
        error: Option<&str>,
    ) -> Option<DeploymentAction> {
        let mut action = None;

        ui.horizontal(|ui| {
            section_header(ui, "Deployments");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                search_bar(ui, &mut self.search_filter, "Search deployments...");
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

        let filtered: Vec<_> = deployments
            .iter()
            .filter(|d| {
                self.search_filter.is_empty()
                    || d.name.to_lowercase().contains(&self.search_filter.to_lowercase())
                    || d.namespace.to_lowercase().contains(&self.search_filter.to_lowercase())
            })
            .collect();

        if filtered.is_empty() {
            empty_state(ui, "No deployments found");
            return None;
        }

        let available_height = ui.available_height();

        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto().at_least(150.0)) // Name
            .column(Column::auto().at_least(100.0)) // Namespace
            .column(Column::auto().at_least(80.0))  // Ready
            .column(Column::auto().at_least(80.0))  // Up-to-date
            .column(Column::auto().at_least(80.0))  // Available
            .column(Column::auto().at_least(60.0))  // Age
            .column(Column::remainder().at_least(200.0)) // Actions
            .min_scrolled_height(0.0)
            .max_scroll_height(available_height - 50.0)
            .header(25.0, |mut header| {
                header.col(|ui| { ui.strong("Name"); });
                header.col(|ui| { ui.strong("Namespace"); });
                header.col(|ui| { ui.strong("Ready"); });
                header.col(|ui| { ui.strong("Up-to-date"); });
                header.col(|ui| { ui.strong("Available"); });
                header.col(|ui| { ui.strong("Age"); });
                header.col(|ui| { ui.strong("Actions"); });
            })
            .body(|mut body| {
                for deployment in &filtered {
                    body.row(30.0, |mut row| {
                        row.col(|ui| {
                            if ui.link(&deployment.name).clicked() {
                                self.selected_deployment = Some((*deployment).clone());
                            }
                        });
                        row.col(|ui| { ui.label(&deployment.namespace); });
                        row.col(|ui| {
                            let ready_text = format!("{}/{}", deployment.ready, deployment.replicas);
                            let color = if deployment.ready == deployment.replicas {
                                Color32::from_rgb(34, 197, 94)
                            } else if deployment.ready > 0 {
                                Color32::from_rgb(234, 179, 8)
                            } else {
                                Color32::from_rgb(239, 68, 68)
                            };
                            ui.label(RichText::new(ready_text).color(color));
                        });
                        row.col(|ui| { ui.label(deployment.updated.to_string()); });
                        row.col(|ui| { ui.label(deployment.available.to_string()); });
                        row.col(|ui| { ui.label(&deployment.age); });
                        row.col(|ui| {
                            ui.horizontal(|ui| {
                                if ui.small_button("Scale").clicked() {
                                    self.selected_deployment = Some((*deployment).clone());
                                    self.scale_replicas = deployment.replicas;
                                    self.show_scale_dialog = true;
                                }
                                if ui.small_button("Restart").clicked() {
                                    action = Some(DeploymentAction::Restart(
                                        deployment.namespace.clone(),
                                        deployment.name.clone(),
                                    ));
                                }
                                if ui.small_button("Delete").on_hover_text("Delete deployment").clicked() {
                                    self.selected_deployment = Some((*deployment).clone());
                                    self.show_delete_dialog = true;
                                }
                            });
                        });
                    });
                }
            });

        // Scale dialog
        if self.show_scale_dialog {
            if let Some(dep) = &self.selected_deployment {
                egui::Window::new("Scale Deployment")
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ui.ctx(), |ui| {
                        ui.label(format!("Scale deployment: {}", dep.name));
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.label("Replicas:");
                            ui.add(egui::DragValue::new(&mut self.scale_replicas).range(0..=100));
                        });
                        ui.add_space(16.0);
                        ui.horizontal(|ui| {
                            if ui.button("Cancel").clicked() {
                                self.show_scale_dialog = false;
                            }
                            if primary_button(ui, "Scale") {
                                action = Some(DeploymentAction::Scale(
                                    dep.namespace.clone(),
                                    dep.name.clone(),
                                    self.scale_replicas,
                                ));
                                self.show_scale_dialog = false;
                            }
                        });
                    });
            }
        }

        // Delete dialog
        if self.show_delete_dialog {
            if let Some(dep) = &self.selected_deployment {
                egui::Window::new("Confirm Delete")
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ui.ctx(), |ui| {
                        ui.label(format!(
                            "Are you sure you want to delete deployment '{}'?",
                            dep.name
                        ));
                        ui.label("This action cannot be undone.");
                        ui.add_space(16.0);
                        ui.horizontal(|ui| {
                            if ui.button("Cancel").clicked() {
                                self.show_delete_dialog = false;
                            }
                            if danger_button(ui, "Delete") {
                                action = Some(DeploymentAction::Delete(
                                    dep.namespace.clone(),
                                    dep.name.clone(),
                                ));
                                self.show_delete_dialog = false;
                            }
                        });
                    });
            }
        }

        // Detail panel
        if let Some(dep) = self.selected_deployment.clone() {
            if !self.show_scale_dialog && !self.show_delete_dialog {
                let mut close_details = false;
                egui::Window::new("Deployment Details")
                    .resizable(true)
                    .default_width(400.0)
                    .show(ui.ctx(), |ui| {
                        if ui.button("Close").clicked() {
                            close_details = true;
                        }
                        ui.separator();
                        info_row(ui, "Name", &dep.name);
                        info_row(ui, "Namespace", &dep.namespace);
                        info_row(ui, "Replicas", &format!("{}/{}", dep.ready, dep.replicas));
                        info_row(ui, "Age", &dep.age);

                        if !dep.images.is_empty() {
                            ui.add_space(8.0);
                            ui.label(RichText::new("Images:").strong());
                            for image in &dep.images {
                                ui.label(format!("  • {}", image));
                            }
                        }

                        if !dep.labels.is_empty() {
                            ui.add_space(8.0);
                            ui.label(RichText::new("Labels:").strong());
                            for (k, v) in &dep.labels {
                                ui.label(format!("  {}={}", k, v));
                            }
                        }
                    });
                if close_details {
                    self.selected_deployment = None;
                }
            }
        }

        action
    }
}
