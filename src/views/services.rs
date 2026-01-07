use crate::k8s::{ServiceInfo, IngressInfo};
use crate::views::common::*;
use egui::{RichText, Ui};
use egui_extras::{Column, TableBuilder};

pub struct ServicesView {
    pub search_filter: String,
    pub selected_service: Option<ServiceInfo>,
    pub selected_ingress: Option<IngressInfo>,
    pub active_tab: ServiceTab,
}

#[derive(Clone, Copy, PartialEq, Default)]
pub enum ServiceTab {
    #[default]
    Services,
    Ingresses,
}

impl Default for ServicesView {
    fn default() -> Self {
        Self {
            search_filter: String::new(),
            selected_service: None,
            selected_ingress: None,
            active_tab: ServiceTab::Services,
        }
    }
}

impl ServicesView {
    pub fn show(
        &mut self,
        ui: &mut Ui,
        services: &[ServiceInfo],
        ingresses: &[IngressInfo],
        loading: bool,
        error: Option<&str>,
    ) {
        ui.horizontal(|ui| {
            if ui.selectable_label(self.active_tab == ServiceTab::Services, "Services").clicked() {
                self.active_tab = ServiceTab::Services;
            }
            if ui.selectable_label(self.active_tab == ServiceTab::Ingresses, "Ingresses").clicked() {
                self.active_tab = ServiceTab::Ingresses;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                search_bar(ui, &mut self.search_filter, "Search...");
            });
        });
        ui.separator();

        if loading {
            loading_spinner(ui);
            return;
        }

        if let Some(err) = error {
            error_label(ui, err);
            return;
        }

        match self.active_tab {
            ServiceTab::Services => self.show_services(ui, services),
            ServiceTab::Ingresses => self.show_ingresses(ui, ingresses),
        }
    }

    fn show_services(&mut self, ui: &mut Ui, services: &[ServiceInfo]) {
        let filtered: Vec<_> = services
            .iter()
            .filter(|s| {
                self.search_filter.is_empty()
                    || s.name.to_lowercase().contains(&self.search_filter.to_lowercase())
                    || s.namespace.to_lowercase().contains(&self.search_filter.to_lowercase())
            })
            .collect();

        if filtered.is_empty() {
            empty_state(ui, "No services found");
            return;
        }

        let available_height = ui.available_height();

        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto().at_least(150.0)) // Name
            .column(Column::auto().at_least(100.0)) // Namespace
            .column(Column::auto().at_least(100.0)) // Type
            .column(Column::auto().at_least(120.0)) // Cluster IP
            .column(Column::auto().at_least(120.0)) // External IP
            .column(Column::auto().at_least(150.0)) // Ports
            .column(Column::remainder().at_least(60.0)) // Age
            .min_scrolled_height(0.0)
            .max_scroll_height(available_height - 50.0)
            .header(25.0, |mut header| {
                header.col(|ui| { ui.strong("Name"); });
                header.col(|ui| { ui.strong("Namespace"); });
                header.col(|ui| { ui.strong("Type"); });
                header.col(|ui| { ui.strong("Cluster IP"); });
                header.col(|ui| { ui.strong("External IP"); });
                header.col(|ui| { ui.strong("Ports"); });
                header.col(|ui| { ui.strong("Age"); });
            })
            .body(|mut body| {
                for service in &filtered {
                    body.row(30.0, |mut row| {
                        row.col(|ui| {
                            if ui.link(&service.name).clicked() {
                                self.selected_service = Some((*service).clone());
                            }
                        });
                        row.col(|ui| { ui.label(&service.namespace); });
                        row.col(|ui| { ui.label(&service.service_type); });
                        row.col(|ui| { ui.label(&service.cluster_ip); });
                        row.col(|ui| { ui.label(&service.external_ip); });
                        row.col(|ui| {
                            let ports_str = service.ports.join(", ");
                            ui.label(truncate_string(&ports_str, 30))
                                .on_hover_text(&ports_str);
                        });
                        row.col(|ui| { ui.label(&service.age); });
                    });
                }
            });

        // Service detail panel
        if let Some(svc) = self.selected_service.clone() {
            let mut close_details = false;
            egui::Window::new("Service Details")
                .resizable(true)
                .default_width(400.0)
                .show(ui.ctx(), |ui| {
                    if ui.button("Close").clicked() {
                        close_details = true;
                    }
                    ui.separator();

                    info_row(ui, "Name", &svc.name);
                    info_row(ui, "Namespace", &svc.namespace);
                    info_row(ui, "Type", &svc.service_type);
                    info_row(ui, "Cluster IP", &svc.cluster_ip);
                    info_row(ui, "External IP", &svc.external_ip);
                    info_row(ui, "Age", &svc.age);

                    ui.add_space(8.0);
                    ui.label(RichText::new("Ports:").strong());
                    for port in &svc.ports {
                        ui.label(format!("  • {}", port));
                    }

                    if !svc.selector.is_empty() {
                        ui.add_space(8.0);
                        ui.label(RichText::new("Selector:").strong());
                        for (k, v) in &svc.selector {
                            ui.label(format!("  {}={}", k, v));
                        }
                    }
                });
            if close_details {
                self.selected_service = None;
            }
        }
    }

    fn show_ingresses(&mut self, ui: &mut Ui, ingresses: &[IngressInfo]) {
        let filtered: Vec<_> = ingresses
            .iter()
            .filter(|i| {
                self.search_filter.is_empty()
                    || i.name.to_lowercase().contains(&self.search_filter.to_lowercase())
                    || i.namespace.to_lowercase().contains(&self.search_filter.to_lowercase())
            })
            .collect();

        if filtered.is_empty() {
            empty_state(ui, "No ingresses found");
            return;
        }

        let available_height = ui.available_height();

        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto().at_least(150.0)) // Name
            .column(Column::auto().at_least(100.0)) // Namespace
            .column(Column::auto().at_least(200.0)) // Hosts
            .column(Column::auto().at_least(150.0)) // Paths
            .column(Column::remainder().at_least(60.0)) // Age
            .min_scrolled_height(0.0)
            .max_scroll_height(available_height - 50.0)
            .header(25.0, |mut header| {
                header.col(|ui| { ui.strong("Name"); });
                header.col(|ui| { ui.strong("Namespace"); });
                header.col(|ui| { ui.strong("Hosts"); });
                header.col(|ui| { ui.strong("Paths"); });
                header.col(|ui| { ui.strong("Age"); });
            })
            .body(|mut body| {
                for ingress in &filtered {
                    body.row(30.0, |mut row| {
                        row.col(|ui| {
                            if ui.link(&ingress.name).clicked() {
                                self.selected_ingress = Some((*ingress).clone());
                            }
                        });
                        row.col(|ui| { ui.label(&ingress.namespace); });
                        row.col(|ui| {
                            let hosts = ingress.hosts.join(", ");
                            ui.label(truncate_string(&hosts, 40)).on_hover_text(&hosts);
                        });
                        row.col(|ui| {
                            let paths = ingress.paths.join(", ");
                            ui.label(truncate_string(&paths, 30)).on_hover_text(&paths);
                        });
                        row.col(|ui| { ui.label(&ingress.age); });
                    });
                }
            });

        // Ingress detail panel
        if let Some(ing) = self.selected_ingress.clone() {
            let mut close_details = false;
            egui::Window::new("Ingress Details")
                .resizable(true)
                .default_width(400.0)
                .show(ui.ctx(), |ui| {
                    if ui.button("Close").clicked() {
                        close_details = true;
                    }
                    ui.separator();

                    info_row(ui, "Name", &ing.name);
                    info_row(ui, "Namespace", &ing.namespace);
                    info_row(ui, "Age", &ing.age);

                    ui.add_space(8.0);
                    ui.label(RichText::new("Hosts:").strong());
                    for host in &ing.hosts {
                        ui.label(format!("  • {}", host));
                    }

                    ui.add_space(8.0);
                    ui.label(RichText::new("Paths:").strong());
                    for path in &ing.paths {
                        ui.label(format!("  • {}", path));
                    }
                });
            if close_details {
                self.selected_ingress = None;
            }
        }
    }
}
