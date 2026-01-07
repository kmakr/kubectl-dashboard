use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use k8s_openapi::api::{
    apps::v1::Deployment,
    batch::v1::{CronJob, Job},
    core::v1::{ConfigMap, Pod, Secret, Service},
    networking::v1::Ingress,
};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::OwnerReference;
use kube::{
    api::{Api, DeleteParams, ListParams, ObjectMeta, Patch, PatchParams, PostParams},
    Client,
};

// Resource data structures for UI display

#[derive(Clone, Debug)]
pub struct DeploymentInfo {
    pub name: String,
    pub namespace: String,
    pub replicas: i32,
    pub available: i32,
    pub ready: i32,
    pub updated: i32,
    pub age: String,
    pub images: Vec<String>,
    pub labels: std::collections::BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct PodInfo {
    pub name: String,
    pub namespace: String,
    pub status: String,
    pub ready: String,
    pub restarts: i32,
    pub age: String,
    pub node: String,
    pub ip: String,
    pub containers: Vec<ContainerInfo>,
}

#[derive(Clone, Debug)]
pub struct ContainerInfo {
    pub name: String,
    pub image: String,
    pub ready: bool,
    pub restarts: i32,
    pub state: String,
}

#[derive(Clone, Debug)]
pub struct ServiceInfo {
    pub name: String,
    pub namespace: String,
    pub service_type: String,
    pub cluster_ip: String,
    pub external_ip: String,
    pub ports: Vec<String>,
    pub age: String,
    pub selector: std::collections::BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct IngressInfo {
    pub name: String,
    pub namespace: String,
    pub hosts: Vec<String>,
    pub paths: Vec<String>,
    pub age: String,
}

#[derive(Clone, Debug)]
pub struct ConfigMapInfo {
    pub name: String,
    pub namespace: String,
    pub data_count: usize,
    pub age: String,
    pub data: std::collections::BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct SecretInfo {
    pub name: String,
    pub namespace: String,
    pub secret_type: String,
    pub data_count: usize,
    pub age: String,
    pub data_keys: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct JobInfo {
    pub name: String,
    pub namespace: String,
    pub completions: String,
    pub duration: String,
    pub age: String,
    pub status: JobStatus,
    pub owner: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum JobStatus {
    Running,
    Succeeded,
    Failed,
    Pending,
}

#[derive(Clone, Debug)]
pub struct CronJobInfo {
    pub name: String,
    pub namespace: String,
    pub schedule: String,
    pub suspend: bool,
    pub active: i32,
    pub last_schedule: Option<String>,
    pub age: String,
}

fn format_age(creation_timestamp: Option<&k8s_openapi::apimachinery::pkg::apis::meta::v1::Time>) -> String {
    let Some(ts) = creation_timestamp else {
        return "Unknown".to_string();
    };

    let created: DateTime<Utc> = ts.0;
    let now = Utc::now();
    let duration = now.signed_duration_since(created);

    if duration.num_days() > 0 {
        format!("{}d", duration.num_days())
    } else if duration.num_hours() > 0 {
        format!("{}h", duration.num_hours())
    } else if duration.num_minutes() > 0 {
        format!("{}m", duration.num_minutes())
    } else {
        format!("{}s", duration.num_seconds())
    }
}

// Deployment operations

pub async fn list_deployments(client: &Client, namespace: Option<&str>) -> Result<Vec<DeploymentInfo>> {
    let deployments: Api<Deployment> = match namespace {
        Some(ns) => Api::namespaced(client.clone(), ns),
        None => Api::all(client.clone()),
    };

    let list = deployments
        .list(&ListParams::default())
        .await
        .context("Failed to list deployments")?;

    Ok(list
        .items
        .into_iter()
        .map(|d| {
            let spec = d.spec.as_ref();
            let status = d.status.as_ref();
            let meta = &d.metadata;

            let images: Vec<String> = spec
                .and_then(|s| s.template.spec.as_ref())
                .map(|ps| ps.containers.iter().map(|c| c.image.clone().unwrap_or_default()).collect())
                .unwrap_or_default();

            DeploymentInfo {
                name: meta.name.clone().unwrap_or_default(),
                namespace: meta.namespace.clone().unwrap_or_default(),
                replicas: spec.and_then(|s| s.replicas).unwrap_or(0),
                available: status.and_then(|s| s.available_replicas).unwrap_or(0),
                ready: status.and_then(|s| s.ready_replicas).unwrap_or(0),
                updated: status.and_then(|s| s.updated_replicas).unwrap_or(0),
                age: format_age(meta.creation_timestamp.as_ref()),
                images,
                labels: meta.labels.clone().unwrap_or_default(),
            }
        })
        .collect())
}

pub async fn scale_deployment(client: &Client, namespace: &str, name: &str, replicas: i32) -> Result<()> {
    let deployments: Api<Deployment> = Api::namespaced(client.clone(), namespace);

    let patch = serde_json::json!({
        "spec": {
            "replicas": replicas
        }
    });

    deployments
        .patch(name, &PatchParams::default(), &Patch::Merge(&patch))
        .await
        .context("Failed to scale deployment")?;

    Ok(())
}

pub async fn restart_deployment(client: &Client, namespace: &str, name: &str) -> Result<()> {
    let deployments: Api<Deployment> = Api::namespaced(client.clone(), namespace);

    let now = Utc::now().to_rfc3339();
    let patch = serde_json::json!({
        "spec": {
            "template": {
                "metadata": {
                    "annotations": {
                        "kubectl.kubernetes.io/restartedAt": now
                    }
                }
            }
        }
    });

    deployments
        .patch(name, &PatchParams::default(), &Patch::Merge(&patch))
        .await
        .context("Failed to restart deployment")?;

    Ok(())
}

pub async fn delete_deployment(client: &Client, namespace: &str, name: &str) -> Result<()> {
    let deployments: Api<Deployment> = Api::namespaced(client.clone(), namespace);
    deployments
        .delete(name, &DeleteParams::default())
        .await
        .context("Failed to delete deployment")?;
    Ok(())
}

// Pod operations

pub async fn list_pods(client: &Client, namespace: Option<&str>) -> Result<Vec<PodInfo>> {
    let pods: Api<Pod> = match namespace {
        Some(ns) => Api::namespaced(client.clone(), ns),
        None => Api::all(client.clone()),
    };

    let list = pods
        .list(&ListParams::default())
        .await
        .context("Failed to list pods")?;

    Ok(list
        .items
        .into_iter()
        .map(|p| {
            let meta = &p.metadata;
            let spec = p.spec.as_ref();
            let status = p.status.as_ref();

            let containers: Vec<ContainerInfo> = spec
                .map(|s| {
                    s.containers
                        .iter()
                        .map(|c| {
                            let container_status = status
                                .and_then(|st| st.container_statuses.as_ref())
                                .and_then(|cs| cs.iter().find(|cs| cs.name == c.name));

                            let state = container_status
                                .and_then(|cs| cs.state.as_ref())
                                .map(|s| {
                                    if s.running.is_some() {
                                        "Running".to_string()
                                    } else if let Some(w) = &s.waiting {
                                        w.reason.clone().unwrap_or_else(|| "Waiting".to_string())
                                    } else if let Some(t) = &s.terminated {
                                        t.reason.clone().unwrap_or_else(|| "Terminated".to_string())
                                    } else {
                                        "Unknown".to_string()
                                    }
                                })
                                .unwrap_or_else(|| "Unknown".to_string());

                            ContainerInfo {
                                name: c.name.clone(),
                                image: c.image.clone().unwrap_or_default(),
                                ready: container_status.map(|cs| cs.ready).unwrap_or(false),
                                restarts: container_status.map(|cs| cs.restart_count).unwrap_or(0),
                                state,
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();

            let total_restarts: i32 = containers.iter().map(|c| c.restarts).sum();
            let ready_containers = containers.iter().filter(|c| c.ready).count();

            let pod_status = status
                .and_then(|s| s.phase.clone())
                .unwrap_or_else(|| "Unknown".to_string());

            PodInfo {
                name: meta.name.clone().unwrap_or_default(),
                namespace: meta.namespace.clone().unwrap_or_default(),
                status: pod_status,
                ready: format!("{}/{}", ready_containers, containers.len()),
                restarts: total_restarts,
                age: format_age(meta.creation_timestamp.as_ref()),
                node: spec.and_then(|s| s.node_name.clone()).unwrap_or_default(),
                ip: status.and_then(|s| s.pod_ip.clone()).unwrap_or_default(),
                containers,
            }
        })
        .collect())
}

pub async fn get_pod_logs(client: &Client, namespace: &str, name: &str, container: Option<&str>, tail_lines: Option<i64>) -> Result<String> {
    use kube::api::LogParams;

    let pods: Api<Pod> = Api::namespaced(client.clone(), namespace);

    let mut params = LogParams::default();
    if let Some(c) = container {
        params.container = Some(c.to_string());
    }
    if let Some(lines) = tail_lines {
        params.tail_lines = Some(lines);
    }

    let logs = pods
        .logs(name, &params)
        .await
        .context("Failed to get pod logs")?;

    Ok(logs)
}

pub async fn delete_pod(client: &Client, namespace: &str, name: &str) -> Result<()> {
    let pods: Api<Pod> = Api::namespaced(client.clone(), namespace);
    pods.delete(name, &DeleteParams::default())
        .await
        .context("Failed to delete pod")?;
    Ok(())
}

// Service operations

pub async fn list_services(client: &Client, namespace: Option<&str>) -> Result<Vec<ServiceInfo>> {
    let services: Api<Service> = match namespace {
        Some(ns) => Api::namespaced(client.clone(), ns),
        None => Api::all(client.clone()),
    };

    let list = services
        .list(&ListParams::default())
        .await
        .context("Failed to list services")?;

    Ok(list
        .items
        .into_iter()
        .map(|s| {
            let meta = &s.metadata;
            let spec = s.spec.as_ref();

            let ports: Vec<String> = spec
                .and_then(|s| s.ports.as_ref())
                .map(|ports| {
                    ports
                        .iter()
                        .map(|p| {
                            let port_str = if let Some(np) = p.node_port {
                                format!("{}:{}/{}", p.port, np, p.protocol.clone().unwrap_or_else(|| "TCP".to_string()))
                            } else {
                                format!("{}/{}", p.port, p.protocol.clone().unwrap_or_else(|| "TCP".to_string()))
                            };
                            port_str
                        })
                        .collect()
                })
                .unwrap_or_default();

            let external_ips: String = spec
                .and_then(|s| s.external_ips.as_ref())
                .map(|ips| ips.join(", "))
                .or_else(|| {
                    s.status
                        .as_ref()
                        .and_then(|st| st.load_balancer.as_ref())
                        .and_then(|lb| lb.ingress.as_ref())
                        .map(|ingress| {
                            ingress
                                .iter()
                                .filter_map(|i| i.ip.clone().or_else(|| i.hostname.clone()))
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                })
                .unwrap_or_else(|| "<none>".to_string());

            ServiceInfo {
                name: meta.name.clone().unwrap_or_default(),
                namespace: meta.namespace.clone().unwrap_or_default(),
                service_type: spec.and_then(|s| s.type_.clone()).unwrap_or_else(|| "ClusterIP".to_string()),
                cluster_ip: spec.and_then(|s| s.cluster_ip.clone()).unwrap_or_default(),
                external_ip: external_ips,
                ports,
                age: format_age(meta.creation_timestamp.as_ref()),
                selector: spec.and_then(|s| s.selector.clone()).unwrap_or_default(),
            }
        })
        .collect())
}

// Ingress operations

pub async fn list_ingresses(client: &Client, namespace: Option<&str>) -> Result<Vec<IngressInfo>> {
    let ingresses: Api<Ingress> = match namespace {
        Some(ns) => Api::namespaced(client.clone(), ns),
        None => Api::all(client.clone()),
    };

    let list = ingresses
        .list(&ListParams::default())
        .await
        .context("Failed to list ingresses")?;

    Ok(list
        .items
        .into_iter()
        .map(|i| {
            let meta = &i.metadata;
            let spec = i.spec.as_ref();

            let mut hosts = Vec::new();
            let mut paths = Vec::new();

            if let Some(rules) = spec.and_then(|s| s.rules.as_ref()) {
                for rule in rules {
                    if let Some(host) = &rule.host {
                        hosts.push(host.clone());
                    }
                    if let Some(http) = &rule.http {
                        for path in &http.paths {
                            paths.push(path.path.clone().unwrap_or_else(|| "/".to_string()));
                        }
                    }
                }
            }

            IngressInfo {
                name: meta.name.clone().unwrap_or_default(),
                namespace: meta.namespace.clone().unwrap_or_default(),
                hosts,
                paths,
                age: format_age(meta.creation_timestamp.as_ref()),
            }
        })
        .collect())
}

// ConfigMap operations

pub async fn list_configmaps(client: &Client, namespace: Option<&str>) -> Result<Vec<ConfigMapInfo>> {
    let configmaps: Api<ConfigMap> = match namespace {
        Some(ns) => Api::namespaced(client.clone(), ns),
        None => Api::all(client.clone()),
    };

    let list = configmaps
        .list(&ListParams::default())
        .await
        .context("Failed to list configmaps")?;

    Ok(list
        .items
        .into_iter()
        .map(|cm| {
            let meta = &cm.metadata;
            let data = cm.data.clone().unwrap_or_default();

            ConfigMapInfo {
                name: meta.name.clone().unwrap_or_default(),
                namespace: meta.namespace.clone().unwrap_or_default(),
                data_count: data.len(),
                age: format_age(meta.creation_timestamp.as_ref()),
                data,
            }
        })
        .collect())
}

pub async fn update_configmap(client: &Client, namespace: &str, name: &str, data: std::collections::BTreeMap<String, String>) -> Result<()> {
    let configmaps: Api<ConfigMap> = Api::namespaced(client.clone(), namespace);

    let patch = serde_json::json!({
        "data": data
    });

    configmaps
        .patch(name, &PatchParams::default(), &Patch::Merge(&patch))
        .await
        .context("Failed to update configmap")?;

    Ok(())
}

// Secret operations

pub async fn list_secrets(client: &Client, namespace: Option<&str>) -> Result<Vec<SecretInfo>> {
    let secrets: Api<Secret> = match namespace {
        Some(ns) => Api::namespaced(client.clone(), ns),
        None => Api::all(client.clone()),
    };

    let list = secrets
        .list(&ListParams::default())
        .await
        .context("Failed to list secrets")?;

    Ok(list
        .items
        .into_iter()
        .map(|s| {
            let meta = &s.metadata;
            let data_keys: Vec<String> = s.data.as_ref()
                .map(|d| d.keys().cloned().collect())
                .unwrap_or_default();

            SecretInfo {
                name: meta.name.clone().unwrap_or_default(),
                namespace: meta.namespace.clone().unwrap_or_default(),
                secret_type: s.type_.clone().unwrap_or_else(|| "Opaque".to_string()),
                data_count: data_keys.len(),
                age: format_age(meta.creation_timestamp.as_ref()),
                data_keys,
            }
        })
        .collect())
}

// Job operations

pub async fn list_jobs(client: &Client, namespace: Option<&str>) -> Result<Vec<JobInfo>> {
    let jobs: Api<Job> = match namespace {
        Some(ns) => Api::namespaced(client.clone(), ns),
        None => Api::all(client.clone()),
    };

    let list = jobs
        .list(&ListParams::default())
        .await
        .context("Failed to list jobs")?;

    Ok(list
        .items
        .into_iter()
        .map(|j| {
            let meta = &j.metadata;
            let spec = j.spec.as_ref();
            let status = j.status.as_ref();

            let completions = format!(
                "{}/{}",
                status.and_then(|s| s.succeeded).unwrap_or(0),
                spec.and_then(|s| s.completions).unwrap_or(1)
            );

            let job_status = if status.and_then(|s| s.succeeded).unwrap_or(0) > 0 {
                JobStatus::Succeeded
            } else if status.and_then(|s| s.failed).unwrap_or(0) > 0 {
                JobStatus::Failed
            } else if status.and_then(|s| s.active).unwrap_or(0) > 0 {
                JobStatus::Running
            } else {
                JobStatus::Pending
            };

            let duration = status
                .and_then(|s| {
                    let start = s.start_time.as_ref()?;
                    let end = s.completion_time.as_ref().map(|t| t.0).unwrap_or_else(Utc::now);
                    let dur = end.signed_duration_since(start.0);
                    Some(format!("{}s", dur.num_seconds()))
                })
                .unwrap_or_else(|| "-".to_string());

            let owner = meta.owner_references.as_ref()
                .and_then(|owners| owners.first())
                .map(|o| o.name.clone());

            JobInfo {
                name: meta.name.clone().unwrap_or_default(),
                namespace: meta.namespace.clone().unwrap_or_default(),
                completions,
                duration,
                age: format_age(meta.creation_timestamp.as_ref()),
                status: job_status,
                owner,
            }
        })
        .collect())
}

pub async fn delete_job(client: &Client, namespace: &str, name: &str) -> Result<()> {
    let jobs: Api<Job> = Api::namespaced(client.clone(), namespace);
    jobs.delete(name, &DeleteParams::default())
        .await
        .context("Failed to delete job")?;
    Ok(())
}

// CronJob operations

pub async fn list_cronjobs(client: &Client, namespace: Option<&str>) -> Result<Vec<CronJobInfo>> {
    let cronjobs: Api<CronJob> = match namespace {
        Some(ns) => Api::namespaced(client.clone(), ns),
        None => Api::all(client.clone()),
    };

    let list = cronjobs
        .list(&ListParams::default())
        .await
        .context("Failed to list cronjobs")?;

    Ok(list
        .items
        .into_iter()
        .map(|cj| {
            let meta = &cj.metadata;
            let spec = cj.spec.as_ref();
            let status = cj.status.as_ref();

            let last_schedule = status
                .and_then(|s| s.last_schedule_time.as_ref())
                .map(|t| format_age(Some(t)));

            CronJobInfo {
                name: meta.name.clone().unwrap_or_default(),
                namespace: meta.namespace.clone().unwrap_or_default(),
                schedule: spec.map(|s| s.schedule.clone()).unwrap_or_default(),
                suspend: spec.and_then(|s| s.suspend).unwrap_or(false),
                active: status.and_then(|s| s.active.as_ref()).map(|a| a.len() as i32).unwrap_or(0),
                last_schedule,
                age: format_age(meta.creation_timestamp.as_ref()),
            }
        })
        .collect())
}

pub async fn trigger_cronjob(client: &Client, namespace: &str, cronjob_name: &str) -> Result<String> {
    let cronjobs: Api<CronJob> = Api::namespaced(client.clone(), namespace);
    let jobs: Api<Job> = Api::namespaced(client.clone(), namespace);

    let cronjob = cronjobs
        .get(cronjob_name)
        .await
        .context("Failed to get cronjob")?;

    let job_template = cronjob
        .spec
        .as_ref()
        .map(|s| &s.job_template)
        .context("CronJob has no job template")?;

    let job_name = format!("{}-manual-{}", cronjob_name, chrono::Utc::now().timestamp());

    let job = Job {
        metadata: ObjectMeta {
            name: Some(job_name.clone()),
            namespace: Some(namespace.to_string()),
            labels: job_template.metadata.as_ref().and_then(|m| m.labels.clone()),
            annotations: job_template.metadata.as_ref().and_then(|m| m.annotations.clone()),
            owner_references: Some(vec![OwnerReference {
                api_version: "batch/v1".to_string(),
                kind: "CronJob".to_string(),
                name: cronjob_name.to_string(),
                uid: cronjob.metadata.uid.clone().unwrap_or_default(),
                controller: Some(true),
                block_owner_deletion: Some(true),
            }]),
            ..Default::default()
        },
        spec: job_template.spec.clone(),
        status: None,
    };

    jobs.create(&PostParams::default(), &job)
        .await
        .context("Failed to create job from cronjob")?;

    Ok(job_name)
}

pub async fn suspend_cronjob(client: &Client, namespace: &str, name: &str, suspend: bool) -> Result<()> {
    let cronjobs: Api<CronJob> = Api::namespaced(client.clone(), namespace);

    let patch = serde_json::json!({
        "spec": {
            "suspend": suspend
        }
    });

    cronjobs
        .patch(name, &PatchParams::default(), &Patch::Merge(&patch))
        .await
        .context("Failed to update cronjob suspend status")?;

    Ok(())
}

pub async fn get_cronjob_history(client: &Client, namespace: &str, cronjob_name: &str) -> Result<Vec<JobInfo>> {
    let jobs = list_jobs(client, Some(namespace)).await?;

    Ok(jobs
        .into_iter()
        .filter(|j| j.owner.as_ref().map(|o| o == cronjob_name).unwrap_or(false))
        .collect())
}
