use crate::k8s::{CronJobInfo, JobInfo, JobStatus};
use crate::views::common::*;
use egui::{Color32, RichText, Ui};
use egui_extras::{Column, TableBuilder};

pub struct CronJobsView {
    pub search_filter: String,
    pub selected_cronjob: Option<CronJobInfo>,
    pub show_history: bool,
    pub history_jobs: Vec<JobInfo>,
    pub history_loading: bool,
    pub pending_action: Option<CronJobAction>,
}

#[derive(Clone)]
pub enum CronJobAction {
    Trigger(String, String),
    Suspend(String, String, bool),
    GetHistory(String, String),
}

impl Default for CronJobsView {
    fn default() -> Self {
        Self {
            search_filter: String::new(),
            selected_cronjob: None,
            show_history: false,
            history_jobs: Vec::new(),
            history_loading: false,
            pending_action: None,
        }
    }
}

impl CronJobsView {
    pub fn show(
        &mut self,
        ui: &mut Ui,
        cronjobs: &[CronJobInfo],
        loading: bool,
        error: Option<&str>,
    ) -> Option<CronJobAction> {
        let mut action = None;

        ui.horizontal(|ui| {
            section_header(ui, "CronJobs");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                search_bar(ui, &mut self.search_filter, "Search cronjobs...");
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

        let filtered: Vec<_> = cronjobs
            .iter()
            .filter(|cj| {
                self.search_filter.is_empty()
                    || cj.name.to_lowercase().contains(&self.search_filter.to_lowercase())
                    || cj.namespace.to_lowercase().contains(&self.search_filter.to_lowercase())
            })
            .collect();

        if filtered.is_empty() {
            empty_state(ui, "No cronjobs found");
            return None;
        }

        let available_height = ui.available_height();

        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto().at_least(180.0)) // Name
            .column(Column::auto().at_least(100.0)) // Namespace
            .column(Column::auto().at_least(120.0)) // Schedule
            .column(Column::auto().at_least(80.0))  // Suspend
            .column(Column::auto().at_least(60.0))  // Active
            .column(Column::auto().at_least(100.0)) // Last Schedule
            .column(Column::auto().at_least(60.0))  // Age
            .column(Column::remainder().at_least(200.0)) // Actions
            .min_scrolled_height(0.0)
            .max_scroll_height(available_height - 50.0)
            .header(25.0, |mut header| {
                header.col(|ui| { ui.strong("Name"); });
                header.col(|ui| { ui.strong("Namespace"); });
                header.col(|ui| { ui.strong("Schedule"); });
                header.col(|ui| { ui.strong("Suspend"); });
                header.col(|ui| { ui.strong("Active"); });
                header.col(|ui| { ui.strong("Last Schedule"); });
                header.col(|ui| { ui.strong("Age"); });
                header.col(|ui| { ui.strong("Actions"); });
            })
            .body(|mut body| {
                for cj in &filtered {
                    body.row(30.0, |mut row| {
                        row.col(|ui| {
                            if ui.link(truncate_string(&cj.name, 40)).on_hover_text(&cj.name).clicked() {
                                self.selected_cronjob = Some((*cj).clone());
                            }
                        });
                        row.col(|ui| { ui.label(&cj.namespace); });
                        row.col(|ui| { ui.label(&cj.schedule); });
                        row.col(|ui| {
                            if cj.suspend {
                                ui.label(RichText::new("Yes").color(Color32::from_rgb(234, 179, 8)));
                            } else {
                                ui.label(RichText::new("No").color(Color32::from_rgb(34, 197, 94)));
                            }
                        });
                        row.col(|ui| { ui.label(cj.active.to_string()); });
                        row.col(|ui| {
                            if let Some(last) = &cj.last_schedule {
                                ui.label(format!("{} ago", last));
                            } else {
                                ui.label("-");
                            }
                        });
                        row.col(|ui| { ui.label(&cj.age); });
                        row.col(|ui| {
                            ui.horizontal(|ui| {
                                if success_button(ui, "Run Now") {
                                    action = Some(CronJobAction::Trigger(
                                        cj.namespace.clone(),
                                        cj.name.clone(),
                                    ));
                                }
                                if cj.suspend {
                                    if ui.small_button("Resume").clicked() {
                                        action = Some(CronJobAction::Suspend(
                                            cj.namespace.clone(),
                                            cj.name.clone(),
                                            false,
                                        ));
                                    }
                                } else {
                                    if warning_button(ui, "Suspend") {
                                        action = Some(CronJobAction::Suspend(
                                            cj.namespace.clone(),
                                            cj.name.clone(),
                                            true,
                                        ));
                                    }
                                }
                                if ui.small_button("History").clicked() {
                                    self.selected_cronjob = Some((*cj).clone());
                                    self.show_history = true;
                                    self.history_loading = true;
                                    action = Some(CronJobAction::GetHistory(
                                        cj.namespace.clone(),
                                        cj.name.clone(),
                                    ));
                                }
                            });
                        });
                    });
                }
            });

        // History window
        if self.show_history {
            if let Some(cj) = &self.selected_cronjob {
                let mut open = true;
                egui::Window::new(format!("Job History - {}", cj.name))
                    .open(&mut open)
                    .resizable(true)
                    .default_size([700.0, 400.0])
                    .show(ui.ctx(), |ui| {
                        if self.history_loading {
                            loading_spinner(ui);
                        } else if self.history_jobs.is_empty() {
                            empty_state(ui, "No job history found");
                        } else {
                            TableBuilder::new(ui)
                                .striped(true)
                                .resizable(true)
                                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                                .column(Column::auto().at_least(250.0)) // Name
                                .column(Column::auto().at_least(100.0)) // Status
                                .column(Column::auto().at_least(100.0)) // Completions
                                .column(Column::auto().at_least(100.0)) // Duration
                                .column(Column::remainder().at_least(80.0)) // Age
                                .header(25.0, |mut header| {
                                    header.col(|ui| { ui.strong("Job Name"); });
                                    header.col(|ui| { ui.strong("Status"); });
                                    header.col(|ui| { ui.strong("Completions"); });
                                    header.col(|ui| { ui.strong("Duration"); });
                                    header.col(|ui| { ui.strong("Age"); });
                                })
                                .body(|mut body| {
                                    for job in &self.history_jobs {
                                        body.row(28.0, |mut row| {
                                            row.col(|ui| { ui.label(&job.name); });
                                            row.col(|ui| {
                                                let (text, color) = match &job.status {
                                                    JobStatus::Running => ("Running", Color32::from_rgb(59, 130, 246)),
                                                    JobStatus::Succeeded => ("Succeeded", Color32::from_rgb(34, 197, 94)),
                                                    JobStatus::Failed => ("Failed", Color32::from_rgb(239, 68, 68)),
                                                    JobStatus::Pending => ("Pending", Color32::from_rgb(234, 179, 8)),
                                                };
                                                status_badge(ui, text, color);
                                            });
                                            row.col(|ui| { ui.label(&job.completions); });
                                            row.col(|ui| { ui.label(&job.duration); });
                                            row.col(|ui| { ui.label(&job.age); });
                                        });
                                    }
                                });
                        }
                    });

                if !open {
                    self.show_history = false;
                    self.selected_cronjob = None;
                }
            }
        }

        // CronJob detail panel
        if let Some(cj) = self.selected_cronjob.clone() {
            if !self.show_history {
                let mut close_details = false;
                egui::Window::new("CronJob Details")
                    .resizable(true)
                    .default_width(400.0)
                    .show(ui.ctx(), |ui| {
                        if ui.button("Close").clicked() {
                            close_details = true;
                        }
                        ui.separator();

                        info_row(ui, "Name", &cj.name);
                        info_row(ui, "Namespace", &cj.namespace);
                        info_row(ui, "Schedule", &cj.schedule);
                        info_row(ui, "Suspended", if cj.suspend { "Yes" } else { "No" });
                        info_row(ui, "Active Jobs", &cj.active.to_string());
                        info_row(ui, "Age", &cj.age);

                        if let Some(last) = &cj.last_schedule {
                            info_row(ui, "Last Schedule", &format!("{} ago", last));
                        }
                    });
                if close_details {
                    self.selected_cronjob = None;
                }
            }
        }

        action
    }

    pub fn set_history(&mut self, jobs: Vec<JobInfo>) {
        self.history_jobs = jobs;
        self.history_loading = false;
    }
}
