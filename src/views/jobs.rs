use crate::k8s::{JobInfo, JobStatus};
use crate::views::common::*;
use egui::{Color32, Ui};
use egui_extras::{Column, TableBuilder};

pub struct JobsView {
    pub search_filter: String,
    pub selected_job: Option<JobInfo>,
    pub show_delete_dialog: bool,
    pub pending_action: Option<JobAction>,
}

#[derive(Clone)]
pub enum JobAction {
    Delete(String, String),
}

impl Default for JobsView {
    fn default() -> Self {
        Self {
            search_filter: String::new(),
            selected_job: None,
            show_delete_dialog: false,
            pending_action: None,
        }
    }
}

impl JobsView {
    pub fn show(
        &mut self,
        ui: &mut Ui,
        jobs: &[JobInfo],
        loading: bool,
        error: Option<&str>,
    ) -> Option<JobAction> {
        let mut action = None;

        ui.horizontal(|ui| {
            section_header(ui, "Jobs");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                search_bar(ui, &mut self.search_filter, "Search jobs...");
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

        let filtered: Vec<_> = jobs
            .iter()
            .filter(|j| {
                self.search_filter.is_empty()
                    || j.name.to_lowercase().contains(&self.search_filter.to_lowercase())
                    || j.namespace.to_lowercase().contains(&self.search_filter.to_lowercase())
            })
            .collect();

        if filtered.is_empty() {
            empty_state(ui, "No jobs found");
            return None;
        }

        let available_height = ui.available_height();

        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto().at_least(200.0)) // Name
            .column(Column::auto().at_least(100.0)) // Namespace
            .column(Column::auto().at_least(100.0)) // Status
            .column(Column::auto().at_least(80.0))  // Completions
            .column(Column::auto().at_least(80.0))  // Duration
            .column(Column::auto().at_least(60.0))  // Age
            .column(Column::remainder().at_least(100.0)) // Actions
            .min_scrolled_height(0.0)
            .max_scroll_height(available_height - 50.0)
            .header(25.0, |mut header| {
                header.col(|ui| { ui.strong("Name"); });
                header.col(|ui| { ui.strong("Namespace"); });
                header.col(|ui| { ui.strong("Status"); });
                header.col(|ui| { ui.strong("Completions"); });
                header.col(|ui| { ui.strong("Duration"); });
                header.col(|ui| { ui.strong("Age"); });
                header.col(|ui| { ui.strong("Actions"); });
            })
            .body(|mut body| {
                for job in &filtered {
                    body.row(30.0, |mut row| {
                        row.col(|ui| {
                            if ui.link(truncate_string(&job.name, 45)).on_hover_text(&job.name).clicked() {
                                self.selected_job = Some((*job).clone());
                            }
                        });
                        row.col(|ui| { ui.label(&job.namespace); });
                        row.col(|ui| {
                            let (status_text, color) = match &job.status {
                                JobStatus::Running => ("Running", Color32::from_rgb(59, 130, 246)),
                                JobStatus::Succeeded => ("Succeeded", Color32::from_rgb(34, 197, 94)),
                                JobStatus::Failed => ("Failed", Color32::from_rgb(239, 68, 68)),
                                JobStatus::Pending => ("Pending", Color32::from_rgb(234, 179, 8)),
                            };
                            status_badge(ui, status_text, color);
                        });
                        row.col(|ui| { ui.label(&job.completions); });
                        row.col(|ui| { ui.label(&job.duration); });
                        row.col(|ui| { ui.label(&job.age); });
                        row.col(|ui| {
                            if ui.small_button("Delete").clicked() {
                                self.selected_job = Some((*job).clone());
                                self.show_delete_dialog = true;
                            }
                        });
                    });
                }
            });

        // Delete dialog
        if self.show_delete_dialog {
            if let Some(job) = &self.selected_job {
                egui::Window::new("Confirm Delete")
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ui.ctx(), |ui| {
                        ui.label(format!("Are you sure you want to delete job '{}'?", job.name));
                        ui.add_space(16.0);
                        ui.horizontal(|ui| {
                            if ui.button("Cancel").clicked() {
                                self.show_delete_dialog = false;
                            }
                            if danger_button(ui, "Delete") {
                                action = Some(JobAction::Delete(
                                    job.namespace.clone(),
                                    job.name.clone(),
                                ));
                                self.show_delete_dialog = false;
                            }
                        });
                    });
            }
        }

        // Job detail panel
        if let Some(job) = self.selected_job.clone() {
            if !self.show_delete_dialog {
                let mut close_details = false;
                egui::Window::new("Job Details")
                    .resizable(true)
                    .default_width(400.0)
                    .show(ui.ctx(), |ui| {
                        if ui.button("Close").clicked() {
                            close_details = true;
                        }
                        ui.separator();

                        info_row(ui, "Name", &job.name);
                        info_row(ui, "Namespace", &job.namespace);
                        info_row(ui, "Completions", &job.completions);
                        info_row(ui, "Duration", &job.duration);
                        info_row(ui, "Age", &job.age);

                        let status_text = match &job.status {
                            JobStatus::Running => "Running",
                            JobStatus::Succeeded => "Succeeded",
                            JobStatus::Failed => "Failed",
                            JobStatus::Pending => "Pending",
                        };
                        info_row(ui, "Status", status_text);

                        if let Some(owner) = &job.owner {
                            info_row(ui, "Owner", owner);
                        }
                    });
                if close_details {
                    self.selected_job = None;
                }
            }
        }

        action
    }
}
