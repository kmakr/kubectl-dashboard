use crate::k8s::{
    self, ConfigMapInfo, CronJobInfo, DeploymentInfo, IngressInfo, JobInfo, K8sClient, PodInfo,
    SecretInfo, ServiceInfo,
};
use crate::views::{
    ConfigView, CronJobsView, DeploymentsView, JobsView, PodsView, ServicesView,
    cronjobs::CronJobAction, deployments::DeploymentAction, jobs::JobAction, pods::PodAction,
    config::ConfigAction,
};
use eframe::egui;
use egui::{Color32, RichText};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use tokio::runtime::Runtime;

#[derive(Clone, Copy, PartialEq, Default)]
pub enum View {
    #[default]
    Deployments,
    Pods,
    Services,
    Config,
    Jobs,
    CronJobs,
}

pub struct KubeDashboard {
    runtime: Arc<Runtime>,
    k8s_client: K8sClient,

    // State
    current_view: View,
    selected_namespace: Option<String>,
    namespaces: Vec<String>,
    contexts: Vec<k8s::ContextInfo>,
    current_context: Option<String>,
    initialized: bool,
    init_error: Option<String>,

    // Data
    deployments: Vec<DeploymentInfo>,
    pods: Vec<PodInfo>,
    services: Vec<ServiceInfo>,
    ingresses: Vec<IngressInfo>,
    configmaps: Vec<ConfigMapInfo>,
    secrets: Vec<SecretInfo>,
    jobs: Vec<JobInfo>,
    cronjobs: Vec<CronJobInfo>,

    // Loading states
    loading_deployments: bool,
    loading_pods: bool,
    loading_services: bool,
    loading_config: bool,
    loading_jobs: bool,
    loading_cronjobs: bool,

    // Errors
    error_deployments: Option<String>,
    error_pods: Option<String>,
    error_services: Option<String>,
    error_config: Option<String>,
    error_jobs: Option<String>,
    error_cronjobs: Option<String>,

    // Views
    deployments_view: DeploymentsView,
    pods_view: PodsView,
    services_view: ServicesView,
    config_view: ConfigView,
    jobs_view: JobsView,
    cronjobs_view: CronJobsView,

    // Message channels
    message_tx: Sender<AppMessage>,
    message_rx: Receiver<AppMessage>,

    // Notifications
    notifications: Vec<Notification>,
}

struct Notification {
    message: String,
    is_error: bool,
    timestamp: std::time::Instant,
}

enum AppMessage {
    Initialized(Result<(), String>),
    ContextsLoaded(Vec<k8s::ContextInfo>, Option<String>),
    NamespacesLoaded(Vec<String>),
    ContextSwitched(Result<(), String>),
    DeploymentsLoaded(Result<Vec<DeploymentInfo>, String>),
    PodsLoaded(Result<Vec<PodInfo>, String>),
    ServicesLoaded(Result<Vec<ServiceInfo>, String>),
    IngressesLoaded(Result<Vec<IngressInfo>, String>),
    ConfigMapsLoaded(Result<Vec<ConfigMapInfo>, String>),
    SecretsLoaded(Result<Vec<SecretInfo>, String>),
    JobsLoaded(Result<Vec<JobInfo>, String>),
    CronJobsLoaded(Result<Vec<CronJobInfo>, String>),
    PodLogsLoaded(Result<String, String>),
    CronJobHistoryLoaded(Result<Vec<JobInfo>, String>),
    ActionCompleted(Result<String, String>),
}

impl KubeDashboard {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let runtime = Arc::new(Runtime::new().expect("Failed to create Tokio runtime"));
        let (message_tx, message_rx) = channel();

        let mut app = Self {
            runtime,
            k8s_client: K8sClient::new(),
            current_view: View::Deployments,
            selected_namespace: None,
            namespaces: vec![],
            contexts: vec![],
            current_context: None,
            initialized: false,
            init_error: None,
            deployments: vec![],
            pods: vec![],
            services: vec![],
            ingresses: vec![],
            configmaps: vec![],
            secrets: vec![],
            jobs: vec![],
            cronjobs: vec![],
            loading_deployments: false,
            loading_pods: false,
            loading_services: false,
            loading_config: false,
            loading_jobs: false,
            loading_cronjobs: false,
            error_deployments: None,
            error_pods: None,
            error_services: None,
            error_config: None,
            error_jobs: None,
            error_cronjobs: None,
            deployments_view: DeploymentsView::default(),
            pods_view: PodsView::default(),
            services_view: ServicesView::default(),
            config_view: ConfigView::default(),
            jobs_view: JobsView::default(),
            cronjobs_view: CronJobsView::default(),
            message_tx,
            message_rx,
            notifications: vec![],
        };

        app.initialize();
        app
    }

    fn initialize(&mut self) {
        let client = self.k8s_client.clone();
        let tx = self.message_tx.clone();

        self.runtime.spawn(async move {
            match client.initialize().await {
                Ok(()) => {
                    let contexts = client.list_contexts().await;
                    let current = client.get_current_context().await;
                    let _ = tx.send(AppMessage::ContextsLoaded(contexts, current));

                    match client.list_namespaces().await {
                        Ok(ns) => {
                            let _ = tx.send(AppMessage::NamespacesLoaded(ns));
                        }
                        Err(e) => {
                            tracing::warn!("Failed to load namespaces: {}", e);
                        }
                    }

                    let _ = tx.send(AppMessage::Initialized(Ok(())));
                }
                Err(e) => {
                    let _ = tx.send(AppMessage::Initialized(Err(e.to_string())));
                }
            }
        });
    }

    fn switch_context(&mut self, context_name: &str) {
        let client = self.k8s_client.clone();
        let tx = self.message_tx.clone();
        let name = context_name.to_string();

        self.runtime.spawn(async move {
            match client.switch_context(&name).await {
                Ok(()) => {
                    let _ = tx.send(AppMessage::ContextSwitched(Ok(())));

                    match client.list_namespaces().await {
                        Ok(ns) => {
                            let _ = tx.send(AppMessage::NamespacesLoaded(ns));
                        }
                        Err(e) => {
                            tracing::warn!("Failed to load namespaces: {}", e);
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(AppMessage::ContextSwitched(Err(e.to_string())));
                }
            }
        });
    }

    fn refresh_current_view(&mut self) {
        match self.current_view {
            View::Deployments => self.load_deployments(),
            View::Pods => self.load_pods(),
            View::Services => {
                self.load_services();
                self.load_ingresses();
            }
            View::Config => {
                self.load_configmaps();
                self.load_secrets();
            }
            View::Jobs => self.load_jobs(),
            View::CronJobs => self.load_cronjobs(),
        }
    }

    fn load_deployments(&mut self) {
        self.loading_deployments = true;
        self.error_deployments = None;

        let client = self.k8s_client.clone();
        let tx = self.message_tx.clone();
        let ns = self.selected_namespace.clone();

        self.runtime.spawn(async move {
            if let Some(c) = client.get_client().await {
                match k8s::list_deployments(&c, ns.as_deref()).await {
                    Ok(deps) => {
                        let _ = tx.send(AppMessage::DeploymentsLoaded(Ok(deps)));
                    }
                    Err(e) => {
                        let _ = tx.send(AppMessage::DeploymentsLoaded(Err(e.to_string())));
                    }
                }
            }
        });
    }

    fn load_pods(&mut self) {
        self.loading_pods = true;
        self.error_pods = None;

        let client = self.k8s_client.clone();
        let tx = self.message_tx.clone();
        let ns = self.selected_namespace.clone();

        self.runtime.spawn(async move {
            if let Some(c) = client.get_client().await {
                match k8s::list_pods(&c, ns.as_deref()).await {
                    Ok(pods) => {
                        let _ = tx.send(AppMessage::PodsLoaded(Ok(pods)));
                    }
                    Err(e) => {
                        let _ = tx.send(AppMessage::PodsLoaded(Err(e.to_string())));
                    }
                }
            }
        });
    }

    fn load_services(&mut self) {
        self.loading_services = true;
        self.error_services = None;

        let client = self.k8s_client.clone();
        let tx = self.message_tx.clone();
        let ns = self.selected_namespace.clone();

        self.runtime.spawn(async move {
            if let Some(c) = client.get_client().await {
                match k8s::list_services(&c, ns.as_deref()).await {
                    Ok(svcs) => {
                        let _ = tx.send(AppMessage::ServicesLoaded(Ok(svcs)));
                    }
                    Err(e) => {
                        let _ = tx.send(AppMessage::ServicesLoaded(Err(e.to_string())));
                    }
                }
            }
        });
    }

    fn load_ingresses(&mut self) {
        let client = self.k8s_client.clone();
        let tx = self.message_tx.clone();
        let ns = self.selected_namespace.clone();

        self.runtime.spawn(async move {
            if let Some(c) = client.get_client().await {
                match k8s::list_ingresses(&c, ns.as_deref()).await {
                    Ok(ings) => {
                        let _ = tx.send(AppMessage::IngressesLoaded(Ok(ings)));
                    }
                    Err(e) => {
                        let _ = tx.send(AppMessage::IngressesLoaded(Err(e.to_string())));
                    }
                }
            }
        });
    }

    fn load_configmaps(&mut self) {
        self.loading_config = true;
        self.error_config = None;

        let client = self.k8s_client.clone();
        let tx = self.message_tx.clone();
        let ns = self.selected_namespace.clone();

        self.runtime.spawn(async move {
            if let Some(c) = client.get_client().await {
                match k8s::list_configmaps(&c, ns.as_deref()).await {
                    Ok(cms) => {
                        let _ = tx.send(AppMessage::ConfigMapsLoaded(Ok(cms)));
                    }
                    Err(e) => {
                        let _ = tx.send(AppMessage::ConfigMapsLoaded(Err(e.to_string())));
                    }
                }
            }
        });
    }

    fn load_secrets(&mut self) {
        let client = self.k8s_client.clone();
        let tx = self.message_tx.clone();
        let ns = self.selected_namespace.clone();

        self.runtime.spawn(async move {
            if let Some(c) = client.get_client().await {
                match k8s::list_secrets(&c, ns.as_deref()).await {
                    Ok(secrets) => {
                        let _ = tx.send(AppMessage::SecretsLoaded(Ok(secrets)));
                    }
                    Err(e) => {
                        let _ = tx.send(AppMessage::SecretsLoaded(Err(e.to_string())));
                    }
                }
            }
        });
    }

    fn load_jobs(&mut self) {
        self.loading_jobs = true;
        self.error_jobs = None;

        let client = self.k8s_client.clone();
        let tx = self.message_tx.clone();
        let ns = self.selected_namespace.clone();

        self.runtime.spawn(async move {
            if let Some(c) = client.get_client().await {
                match k8s::list_jobs(&c, ns.as_deref()).await {
                    Ok(jobs) => {
                        let _ = tx.send(AppMessage::JobsLoaded(Ok(jobs)));
                    }
                    Err(e) => {
                        let _ = tx.send(AppMessage::JobsLoaded(Err(e.to_string())));
                    }
                }
            }
        });
    }

    fn load_cronjobs(&mut self) {
        self.loading_cronjobs = true;
        self.error_cronjobs = None;

        let client = self.k8s_client.clone();
        let tx = self.message_tx.clone();
        let ns = self.selected_namespace.clone();

        self.runtime.spawn(async move {
            if let Some(c) = client.get_client().await {
                match k8s::list_cronjobs(&c, ns.as_deref()).await {
                    Ok(cjs) => {
                        let _ = tx.send(AppMessage::CronJobsLoaded(Ok(cjs)));
                    }
                    Err(e) => {
                        let _ = tx.send(AppMessage::CronJobsLoaded(Err(e.to_string())));
                    }
                }
            }
        });
    }

    fn handle_deployment_action(&mut self, action: DeploymentAction) {
        let client = self.k8s_client.clone();
        let tx = self.message_tx.clone();

        match action {
            DeploymentAction::Scale(ns, name, replicas) => {
                self.runtime.spawn(async move {
                    if let Some(c) = client.get_client().await {
                        match k8s::scale_deployment(&c, &ns, &name, replicas).await {
                            Ok(()) => {
                                let _ = tx.send(AppMessage::ActionCompleted(Ok(format!(
                                    "Scaled {} to {} replicas",
                                    name, replicas
                                ))));
                            }
                            Err(e) => {
                                let _ = tx.send(AppMessage::ActionCompleted(Err(e.to_string())));
                            }
                        }
                    }
                });
            }
            DeploymentAction::Restart(ns, name) => {
                self.runtime.spawn(async move {
                    if let Some(c) = client.get_client().await {
                        match k8s::restart_deployment(&c, &ns, &name).await {
                            Ok(()) => {
                                let _ = tx.send(AppMessage::ActionCompleted(Ok(format!(
                                    "Restarted deployment {}",
                                    name
                                ))));
                            }
                            Err(e) => {
                                let _ = tx.send(AppMessage::ActionCompleted(Err(e.to_string())));
                            }
                        }
                    }
                });
            }
            DeploymentAction::Delete(ns, name) => {
                self.runtime.spawn(async move {
                    if let Some(c) = client.get_client().await {
                        match k8s::delete_deployment(&c, &ns, &name).await {
                            Ok(()) => {
                                let _ = tx.send(AppMessage::ActionCompleted(Ok(format!(
                                    "Deleted deployment {}",
                                    name
                                ))));
                            }
                            Err(e) => {
                                let _ = tx.send(AppMessage::ActionCompleted(Err(e.to_string())));
                            }
                        }
                    }
                });
            }
        }
    }

    fn handle_pod_action(&mut self, action: PodAction) {
        let client = self.k8s_client.clone();
        let tx = self.message_tx.clone();

        match action {
            PodAction::Delete(ns, name) => {
                self.runtime.spawn(async move {
                    if let Some(c) = client.get_client().await {
                        match k8s::delete_pod(&c, &ns, &name).await {
                            Ok(()) => {
                                let _ = tx.send(AppMessage::ActionCompleted(Ok(format!(
                                    "Deleted pod {}",
                                    name
                                ))));
                            }
                            Err(e) => {
                                let _ = tx.send(AppMessage::ActionCompleted(Err(e.to_string())));
                            }
                        }
                    }
                });
            }
            PodAction::GetLogs(ns, name, container, tail_lines) => {
                self.pods_view.set_logs_loading();
                self.runtime.spawn(async move {
                    if let Some(c) = client.get_client().await {
                        match k8s::get_pod_logs(&c, &ns, &name, container.as_deref(), Some(tail_lines)).await {
                            Ok(logs) => {
                                let _ = tx.send(AppMessage::PodLogsLoaded(Ok(logs)));
                            }
                            Err(e) => {
                                let _ = tx.send(AppMessage::PodLogsLoaded(Err(e.to_string())));
                            }
                        }
                    }
                });
            }
        }
    }

    fn handle_config_action(&mut self, action: ConfigAction) {
        let client = self.k8s_client.clone();
        let tx = self.message_tx.clone();

        match action {
            ConfigAction::UpdateConfigMap(ns, name, data) => {
                self.runtime.spawn(async move {
                    if let Some(c) = client.get_client().await {
                        match k8s::update_configmap(&c, &ns, &name, data).await {
                            Ok(()) => {
                                let _ = tx.send(AppMessage::ActionCompleted(Ok(format!(
                                    "Updated configmap {}",
                                    name
                                ))));
                            }
                            Err(e) => {
                                let _ = tx.send(AppMessage::ActionCompleted(Err(e.to_string())));
                            }
                        }
                    }
                });
            }
        }
    }

    fn handle_job_action(&mut self, action: JobAction) {
        let client = self.k8s_client.clone();
        let tx = self.message_tx.clone();

        match action {
            JobAction::Delete(ns, name) => {
                self.runtime.spawn(async move {
                    if let Some(c) = client.get_client().await {
                        match k8s::delete_job(&c, &ns, &name).await {
                            Ok(()) => {
                                let _ = tx.send(AppMessage::ActionCompleted(Ok(format!(
                                    "Deleted job {}",
                                    name
                                ))));
                            }
                            Err(e) => {
                                let _ = tx.send(AppMessage::ActionCompleted(Err(e.to_string())));
                            }
                        }
                    }
                });
            }
        }
    }

    fn handle_cronjob_action(&mut self, action: CronJobAction) {
        let client = self.k8s_client.clone();
        let tx = self.message_tx.clone();

        match action {
            CronJobAction::Trigger(ns, name) => {
                self.runtime.spawn(async move {
                    if let Some(c) = client.get_client().await {
                        match k8s::trigger_cronjob(&c, &ns, &name).await {
                            Ok(job_name) => {
                                let _ = tx.send(AppMessage::ActionCompleted(Ok(format!(
                                    "Created job {} from cronjob {}",
                                    job_name, name
                                ))));
                            }
                            Err(e) => {
                                let _ = tx.send(AppMessage::ActionCompleted(Err(e.to_string())));
                            }
                        }
                    }
                });
            }
            CronJobAction::Suspend(ns, name, suspend) => {
                self.runtime.spawn(async move {
                    if let Some(c) = client.get_client().await {
                        match k8s::suspend_cronjob(&c, &ns, &name, suspend).await {
                            Ok(()) => {
                                let msg = if suspend {
                                    format!("Suspended cronjob {}", name)
                                } else {
                                    format!("Resumed cronjob {}", name)
                                };
                                let _ = tx.send(AppMessage::ActionCompleted(Ok(msg)));
                            }
                            Err(e) => {
                                let _ = tx.send(AppMessage::ActionCompleted(Err(e.to_string())));
                            }
                        }
                    }
                });
            }
            CronJobAction::GetHistory(ns, name) => {
                self.runtime.spawn(async move {
                    if let Some(c) = client.get_client().await {
                        match k8s::get_cronjob_history(&c, &ns, &name).await {
                            Ok(jobs) => {
                                let _ = tx.send(AppMessage::CronJobHistoryLoaded(Ok(jobs)));
                            }
                            Err(e) => {
                                let _ = tx.send(AppMessage::CronJobHistoryLoaded(Err(e.to_string())));
                            }
                        }
                    }
                });
            }
        }
    }

    fn process_messages(&mut self) {
        while let Ok(msg) = self.message_rx.try_recv() {
            match msg {
                AppMessage::Initialized(result) => {
                    match result {
                        Ok(()) => {
                            self.initialized = true;
                            self.refresh_current_view();
                        }
                        Err(e) => {
                            self.init_error = Some(e);
                        }
                    }
                }
                AppMessage::ContextsLoaded(contexts, current) => {
                    self.contexts = contexts;
                    self.current_context = current;
                }
                AppMessage::NamespacesLoaded(ns) => {
                    self.namespaces = ns;
                }
                AppMessage::ContextSwitched(result) => {
                    match result {
                        Ok(()) => {
                            self.add_notification("Context switched successfully", false);
                            self.refresh_current_view();
                        }
                        Err(e) => {
                            self.add_notification(&format!("Failed to switch context: {}", e), true);
                        }
                    }
                }
                AppMessage::DeploymentsLoaded(result) => {
                    self.loading_deployments = false;
                    match result {
                        Ok(deps) => self.deployments = deps,
                        Err(e) => self.error_deployments = Some(e),
                    }
                }
                AppMessage::PodsLoaded(result) => {
                    self.loading_pods = false;
                    match result {
                        Ok(pods) => self.pods = pods,
                        Err(e) => self.error_pods = Some(e),
                    }
                }
                AppMessage::ServicesLoaded(result) => {
                    self.loading_services = false;
                    match result {
                        Ok(svcs) => self.services = svcs,
                        Err(e) => self.error_services = Some(e),
                    }
                }
                AppMessage::IngressesLoaded(result) => {
                    match result {
                        Ok(ings) => self.ingresses = ings,
                        Err(e) => self.error_services = Some(e),
                    }
                }
                AppMessage::ConfigMapsLoaded(result) => {
                    self.loading_config = false;
                    match result {
                        Ok(cms) => self.configmaps = cms,
                        Err(e) => self.error_config = Some(e),
                    }
                }
                AppMessage::SecretsLoaded(result) => {
                    match result {
                        Ok(secrets) => self.secrets = secrets,
                        Err(e) => self.error_config = Some(e),
                    }
                }
                AppMessage::JobsLoaded(result) => {
                    self.loading_jobs = false;
                    match result {
                        Ok(jobs) => self.jobs = jobs,
                        Err(e) => self.error_jobs = Some(e),
                    }
                }
                AppMessage::CronJobsLoaded(result) => {
                    self.loading_cronjobs = false;
                    match result {
                        Ok(cjs) => self.cronjobs = cjs,
                        Err(e) => self.error_cronjobs = Some(e),
                    }
                }
                AppMessage::PodLogsLoaded(result) => {
                    match result {
                        Ok(logs) => self.pods_view.set_logs(logs),
                        Err(e) => self.pods_view.set_logs(format!("Error: {}", e)),
                    }
                }
                AppMessage::CronJobHistoryLoaded(result) => {
                    match result {
                        Ok(jobs) => self.cronjobs_view.set_history(jobs),
                        Err(e) => {
                            self.add_notification(&format!("Failed to load history: {}", e), true);
                            self.cronjobs_view.set_history(vec![]);
                        }
                    }
                }
                AppMessage::ActionCompleted(result) => {
                    match result {
                        Ok(msg) => {
                            self.add_notification(&msg, false);
                            self.refresh_current_view();
                        }
                        Err(e) => {
                            self.add_notification(&format!("Error: {}", e), true);
                        }
                    }
                }
            }
        }
    }

    fn add_notification(&mut self, message: &str, is_error: bool) {
        self.notifications.push(Notification {
            message: message.to_string(),
            is_error,
            timestamp: std::time::Instant::now(),
        });
    }

    fn show_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.add_space(8.0);
            ui.heading(RichText::new("Kubectl Dashboard").strong());
            ui.add_space(16.0);

            // Context selector
            ui.label(RichText::new("Context").strong());
            egui::ComboBox::from_id_salt("context_selector")
                .selected_text(self.current_context.as_deref().unwrap_or("Select context..."))
                .width(180.0)
                .show_ui(ui, |ui| {
                    for ctx in &self.contexts.clone() {
                        let selected = self.current_context.as_ref() == Some(&ctx.name);
                        if ui.selectable_label(selected, &ctx.name).clicked() {
                            self.current_context = Some(ctx.name.clone());
                            self.switch_context(&ctx.name);
                        }
                    }
                });

            ui.add_space(12.0);

            // Namespace selector
            ui.label(RichText::new("Namespace").strong());
            egui::ComboBox::from_id_salt("namespace_selector")
                .selected_text(
                    self.selected_namespace
                        .as_deref()
                        .unwrap_or("All namespaces"),
                )
                .width(180.0)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(self.selected_namespace.is_none(), "All namespaces")
                        .clicked()
                    {
                        self.selected_namespace = None;
                        self.refresh_current_view();
                    }
                    for ns in &self.namespaces.clone() {
                        let selected = self.selected_namespace.as_ref() == Some(ns);
                        if ui.selectable_label(selected, ns).clicked() {
                            self.selected_namespace = Some(ns.clone());
                            self.refresh_current_view();
                        }
                    }
                });

            ui.add_space(24.0);
            ui.separator();
            ui.add_space(8.0);

            // Navigation
            ui.label(RichText::new("Workloads").strong().small());
            if ui
                .selectable_label(self.current_view == View::Deployments, "  Deployments")
                .clicked()
            {
                self.current_view = View::Deployments;
                self.load_deployments();
            }
            if ui
                .selectable_label(self.current_view == View::Pods, "  Pods")
                .clicked()
            {
                self.current_view = View::Pods;
                self.load_pods();
            }
            if ui
                .selectable_label(self.current_view == View::Jobs, "  Jobs")
                .clicked()
            {
                self.current_view = View::Jobs;
                self.load_jobs();
            }
            if ui
                .selectable_label(self.current_view == View::CronJobs, "  CronJobs")
                .clicked()
            {
                self.current_view = View::CronJobs;
                self.load_cronjobs();
            }

            ui.add_space(12.0);
            ui.label(RichText::new("Network").strong().small());
            if ui
                .selectable_label(self.current_view == View::Services, "  Services & Ingresses")
                .clicked()
            {
                self.current_view = View::Services;
                self.load_services();
                self.load_ingresses();
            }

            ui.add_space(12.0);
            ui.label(RichText::new("Configuration").strong().small());
            if ui
                .selectable_label(self.current_view == View::Config, "  ConfigMaps & Secrets")
                .clicked()
            {
                self.current_view = View::Config;
                self.load_configmaps();
                self.load_secrets();
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.add_space(8.0);
                if ui.button("Refresh").clicked() {
                    self.refresh_current_view();
                }
            });
        });
    }

    fn show_notifications(&mut self, ctx: &egui::Context) {
        let now = std::time::Instant::now();
        self.notifications.retain(|n| now.duration_since(n.timestamp).as_secs() < 5);

        for (i, notification) in self.notifications.iter().enumerate() {
            let color = if notification.is_error {
                Color32::from_rgb(239, 68, 68)
            } else {
                Color32::from_rgb(34, 197, 94)
            };

            egui::Area::new(egui::Id::new(format!("notification_{}", i)))
                .anchor(egui::Align2::RIGHT_TOP, [-20.0, 50.0 + i as f32 * 60.0])
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .fill(Color32::from_rgb(30, 30, 30))
                        .stroke(egui::Stroke::new(2.0, color))
                        .rounding(8.0)
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.colored_label(color, "●");
                                ui.label(&notification.message);
                            });
                        });
                });
        }
    }
}

impl eframe::App for KubeDashboard {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_messages();

        // Request continuous repaints for animations and updates
        ctx.request_repaint_after(std::time::Duration::from_millis(100));

        // Show notifications
        self.show_notifications(ctx);

        // Check initialization
        if let Some(error) = self.init_error.clone() {
            let mut retry_clicked = false;
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.heading(RichText::new("Failed to Initialize").color(Color32::from_rgb(239, 68, 68)));
                    ui.add_space(16.0);
                    ui.label(&error);
                    ui.add_space(24.0);
                    ui.label("Please ensure your kubeconfig is properly configured.");
                    ui.add_space(16.0);
                    if ui.button("Retry").clicked() {
                        retry_clicked = true;
                    }
                });
            });
            if retry_clicked {
                self.init_error = None;
                self.initialize();
            }
            return;
        }

        if !self.initialized {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.heading("Kubectl Dashboard");
                    ui.add_space(24.0);
                    ui.spinner();
                    ui.add_space(8.0);
                    ui.label("Connecting to Kubernetes...");
                });
            });
            return;
        }

        // Sidebar
        egui::SidePanel::left("sidebar")
            .resizable(false)
            .default_width(220.0)
            .show(ctx, |ui| {
                self.show_sidebar(ui);
            });

        // Main content
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(8.0);

            match self.current_view {
                View::Deployments => {
                    if let Some(action) = self.deployments_view.show(
                        ui,
                        &self.deployments,
                        self.loading_deployments,
                        self.error_deployments.as_deref(),
                    ) {
                        self.handle_deployment_action(action);
                    }
                }
                View::Pods => {
                    if let Some(action) = self.pods_view.show(
                        ui,
                        &self.pods,
                        self.loading_pods,
                        self.error_pods.as_deref(),
                    ) {
                        self.handle_pod_action(action);
                    }
                }
                View::Services => {
                    self.services_view.show(
                        ui,
                        &self.services,
                        &self.ingresses,
                        self.loading_services,
                        self.error_services.as_deref(),
                    );
                }
                View::Config => {
                    if let Some(action) = self.config_view.show(
                        ui,
                        &self.configmaps,
                        &self.secrets,
                        self.loading_config,
                        self.error_config.as_deref(),
                    ) {
                        self.handle_config_action(action);
                    }
                }
                View::Jobs => {
                    if let Some(action) = self.jobs_view.show(
                        ui,
                        &self.jobs,
                        self.loading_jobs,
                        self.error_jobs.as_deref(),
                    ) {
                        self.handle_job_action(action);
                    }
                }
                View::CronJobs => {
                    if let Some(action) = self.cronjobs_view.show(
                        ui,
                        &self.cronjobs,
                        self.loading_cronjobs,
                        self.error_cronjobs.as_deref(),
                    ) {
                        self.handle_cronjob_action(action);
                    }
                }
            }
        });
    }
}
