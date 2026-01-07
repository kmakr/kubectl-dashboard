use crate::k8s::{ConfigMapInfo, SecretInfo};
use crate::views::common::*;
use egui::{RichText, Ui, ScrollArea};
use egui_extras::{Column, TableBuilder};
use std::collections::BTreeMap;

pub struct ConfigView {
    pub search_filter: String,
    pub active_tab: ConfigTab,
    pub selected_configmap: Option<ConfigMapInfo>,
    pub selected_secret: Option<SecretInfo>,
    pub editing_configmap: bool,
    pub edit_data: BTreeMap<String, String>,
    pub new_key: String,
    pub new_value: String,
    pub pending_action: Option<ConfigAction>,
}

#[derive(Clone, Copy, PartialEq, Default)]
pub enum ConfigTab {
    #[default]
    ConfigMaps,
    Secrets,
}

#[derive(Clone)]
pub enum ConfigAction {
    UpdateConfigMap(String, String, BTreeMap<String, String>),
}

impl Default for ConfigView {
    fn default() -> Self {
        Self {
            search_filter: String::new(),
            active_tab: ConfigTab::ConfigMaps,
            selected_configmap: None,
            selected_secret: None,
            editing_configmap: false,
            edit_data: BTreeMap::new(),
            new_key: String::new(),
            new_value: String::new(),
            pending_action: None,
        }
    }
}

impl ConfigView {
    pub fn show(
        &mut self,
        ui: &mut Ui,
        configmaps: &[ConfigMapInfo],
        secrets: &[SecretInfo],
        loading: bool,
        error: Option<&str>,
    ) -> Option<ConfigAction> {
        let mut action = None;

        ui.horizontal(|ui| {
            if ui.selectable_label(self.active_tab == ConfigTab::ConfigMaps, "ConfigMaps").clicked() {
                self.active_tab = ConfigTab::ConfigMaps;
            }
            if ui.selectable_label(self.active_tab == ConfigTab::Secrets, "Secrets").clicked() {
                self.active_tab = ConfigTab::Secrets;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                search_bar(ui, &mut self.search_filter, "Search...");
            });
        });
        ui.separator();

        if loading {
            loading_spinner(ui);
            return None;
        }

        if let Some(err) = error {
            error_label(ui, err);
            return None;
        }

        match self.active_tab {
            ConfigTab::ConfigMaps => action = self.show_configmaps(ui, configmaps),
            ConfigTab::Secrets => self.show_secrets(ui, secrets),
        }

        action
    }

    fn show_configmaps(&mut self, ui: &mut Ui, configmaps: &[ConfigMapInfo]) -> Option<ConfigAction> {
        let mut action = None;

        let filtered: Vec<_> = configmaps
            .iter()
            .filter(|cm| {
                self.search_filter.is_empty()
                    || cm.name.to_lowercase().contains(&self.search_filter.to_lowercase())
                    || cm.namespace.to_lowercase().contains(&self.search_filter.to_lowercase())
            })
            .collect();

        if filtered.is_empty() {
            empty_state(ui, "No ConfigMaps found");
            return None;
        }

        let available_height = ui.available_height();

        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto().at_least(200.0)) // Name
            .column(Column::auto().at_least(120.0)) // Namespace
            .column(Column::auto().at_least(80.0))  // Data
            .column(Column::remainder().at_least(60.0)) // Age
            .min_scrolled_height(0.0)
            .max_scroll_height(available_height - 50.0)
            .header(25.0, |mut header| {
                header.col(|ui| { ui.strong("Name"); });
                header.col(|ui| { ui.strong("Namespace"); });
                header.col(|ui| { ui.strong("Data"); });
                header.col(|ui| { ui.strong("Age"); });
            })
            .body(|mut body| {
                for cm in &filtered {
                    body.row(30.0, |mut row| {
                        row.col(|ui| {
                            if ui.link(&cm.name).clicked() {
                                self.selected_configmap = Some((*cm).clone());
                                self.editing_configmap = false;
                            }
                        });
                        row.col(|ui| { ui.label(&cm.namespace); });
                        row.col(|ui| { ui.label(cm.data_count.to_string()); });
                        row.col(|ui| { ui.label(&cm.age); });
                    });
                }
            });

        // ConfigMap detail/edit panel
        if let Some(cm) = &self.selected_configmap.clone() {
            egui::Window::new(format!("ConfigMap: {}", cm.name))
                .resizable(true)
                .default_size([600.0, 500.0])
                .show(ui.ctx(), |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("Close").clicked() {
                            self.selected_configmap = None;
                            self.editing_configmap = false;
                        }
                        if !self.editing_configmap {
                            if ui.button("Edit").clicked() {
                                self.editing_configmap = true;
                                self.edit_data = cm.data.clone();
                                self.new_key.clear();
                                self.new_value.clear();
                            }
                        }
                    });
                    ui.separator();

                    info_row(ui, "Name", &cm.name);
                    info_row(ui, "Namespace", &cm.namespace);
                    info_row(ui, "Age", &cm.age);
                    ui.add_space(8.0);

                    if self.editing_configmap {
                        ui.label(RichText::new("Edit Data:").strong());
                        ui.separator();

                        let mut keys_to_remove = Vec::new();

                        ScrollArea::vertical()
                            .max_height(300.0)
                            .show(ui, |ui| {
                                for (key, value) in self.edit_data.clone() {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(&key).strong());
                                        if ui.small_button("×").clicked() {
                                            keys_to_remove.push(key.clone());
                                        }
                                    });
                                    let mut val = value.clone();
                                    if ui.add(
                                        egui::TextEdit::multiline(&mut val)
                                            .font(egui::TextStyle::Monospace)
                                            .desired_width(f32::INFINITY)
                                            .desired_rows(3)
                                    ).changed() {
                                        self.edit_data.insert(key, val);
                                    }
                                    ui.add_space(8.0);
                                }
                            });

                        for key in keys_to_remove {
                            self.edit_data.remove(&key);
                        }

                        ui.separator();
                        ui.label(RichText::new("Add New Key:").strong());
                        ui.horizontal(|ui| {
                            ui.label("Key:");
                            ui.text_edit_singleline(&mut self.new_key);
                        });
                        ui.label("Value:");
                        ui.add(
                            egui::TextEdit::multiline(&mut self.new_value)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(f32::INFINITY)
                                .desired_rows(2)
                        );
                        if ui.button("Add Key").clicked() && !self.new_key.is_empty() {
                            self.edit_data.insert(self.new_key.clone(), self.new_value.clone());
                            self.new_key.clear();
                            self.new_value.clear();
                        }

                        ui.add_space(16.0);
                        ui.horizontal(|ui| {
                            if ui.button("Cancel").clicked() {
                                self.editing_configmap = false;
                            }
                            if success_button(ui, "Save") {
                                action = Some(ConfigAction::UpdateConfigMap(
                                    cm.namespace.clone(),
                                    cm.name.clone(),
                                    self.edit_data.clone(),
                                ));
                                self.editing_configmap = false;
                            }
                        });
                    } else {
                        ui.label(RichText::new("Data:").strong());
                        ui.separator();

                        ScrollArea::vertical()
                            .max_height(400.0)
                            .show(ui, |ui| {
                                for (key, value) in &cm.data {
                                    ui.collapsing(RichText::new(key).strong(), |ui| {
                                        ui.add(
                                            egui::TextEdit::multiline(&mut value.as_str())
                                                .font(egui::TextStyle::Monospace)
                                                .desired_width(f32::INFINITY)
                                        );
                                    });
                                }
                            });
                    }
                });
        }

        action
    }

    fn show_secrets(&mut self, ui: &mut Ui, secrets: &[SecretInfo]) {
        let filtered: Vec<_> = secrets
            .iter()
            .filter(|s| {
                self.search_filter.is_empty()
                    || s.name.to_lowercase().contains(&self.search_filter.to_lowercase())
                    || s.namespace.to_lowercase().contains(&self.search_filter.to_lowercase())
            })
            .collect();

        if filtered.is_empty() {
            empty_state(ui, "No Secrets found");
            return;
        }

        let available_height = ui.available_height();

        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto().at_least(200.0)) // Name
            .column(Column::auto().at_least(120.0)) // Namespace
            .column(Column::auto().at_least(150.0)) // Type
            .column(Column::auto().at_least(80.0))  // Data
            .column(Column::remainder().at_least(60.0)) // Age
            .min_scrolled_height(0.0)
            .max_scroll_height(available_height - 50.0)
            .header(25.0, |mut header| {
                header.col(|ui| { ui.strong("Name"); });
                header.col(|ui| { ui.strong("Namespace"); });
                header.col(|ui| { ui.strong("Type"); });
                header.col(|ui| { ui.strong("Data"); });
                header.col(|ui| { ui.strong("Age"); });
            })
            .body(|mut body| {
                for secret in &filtered {
                    body.row(30.0, |mut row| {
                        row.col(|ui| {
                            if ui.link(&secret.name).clicked() {
                                self.selected_secret = Some((*secret).clone());
                            }
                        });
                        row.col(|ui| { ui.label(&secret.namespace); });
                        row.col(|ui| { ui.label(&secret.secret_type); });
                        row.col(|ui| { ui.label(secret.data_count.to_string()); });
                        row.col(|ui| { ui.label(&secret.age); });
                    });
                }
            });

        // Secret detail panel
        if let Some(secret) = self.selected_secret.clone() {
            let mut close_details = false;
            egui::Window::new(format!("Secret: {}", secret.name))
                .resizable(true)
                .default_width(400.0)
                .show(ui.ctx(), |ui| {
                    if ui.button("Close").clicked() {
                        close_details = true;
                    }
                    ui.separator();

                    info_row(ui, "Name", &secret.name);
                    info_row(ui, "Namespace", &secret.namespace);
                    info_row(ui, "Type", &secret.secret_type);
                    info_row(ui, "Age", &secret.age);

                    ui.add_space(8.0);
                    ui.label(RichText::new("Keys:").strong());
                    ui.label(RichText::new("(Values hidden for security)").small().weak());
                    for key in &secret.data_keys {
                        ui.label(format!("  • {}", key));
                    }
                });
            if close_details {
                self.selected_secret = None;
            }
        }
    }
}
