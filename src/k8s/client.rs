use anyhow::{Context, Result};
use kube::{config::Kubeconfig, Client, Config};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct K8sClient {
    inner: Arc<RwLock<ClientState>>,
}

struct ClientState {
    client: Option<Client>,
    current_context: Option<String>,
    kubeconfig: Option<Kubeconfig>,
}

#[derive(Clone, Debug)]
pub struct ContextInfo {
    pub name: String,
    pub cluster: String,
    pub user: String,
    pub namespace: Option<String>,
}

impl K8sClient {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(ClientState {
                client: None,
                current_context: None,
                kubeconfig: None,
            })),
        }
    }

    pub async fn initialize(&self) -> Result<()> {
        let kubeconfig = Kubeconfig::read().context("Failed to read kubeconfig")?;
        let current_context = kubeconfig.current_context.clone();

        let config = Config::from_kubeconfig(&kube::config::KubeConfigOptions {
            context: current_context.clone(),
            ..Default::default()
        })
        .await
        .context("Failed to create config from kubeconfig")?;

        let client = Client::try_from(config).context("Failed to create Kubernetes client")?;

        let mut state = self.inner.write().await;
        state.client = Some(client);
        state.current_context = current_context;
        state.kubeconfig = Some(kubeconfig);

        Ok(())
    }

    pub async fn get_client(&self) -> Option<Client> {
        self.inner.read().await.client.clone()
    }

    pub async fn get_current_context(&self) -> Option<String> {
        self.inner.read().await.current_context.clone()
    }

    pub async fn list_contexts(&self) -> Vec<ContextInfo> {
        let state = self.inner.read().await;
        let Some(kubeconfig) = &state.kubeconfig else {
            return vec![];
        };

        kubeconfig
            .contexts
            .iter()
            .filter_map(|ctx| {
                let context = ctx.context.as_ref()?;
                Some(ContextInfo {
                    name: ctx.name.clone(),
                    cluster: context.cluster.clone(),
                    user: context.user.clone().unwrap_or_default(),
                    namespace: context.namespace.clone(),
                })
            })
            .collect()
    }

    pub async fn switch_context(&self, context_name: &str) -> Result<()> {
        let config = Config::from_kubeconfig(&kube::config::KubeConfigOptions {
            context: Some(context_name.to_string()),
            ..Default::default()
        })
        .await
        .context("Failed to create config for new context")?;

        let client = Client::try_from(config).context("Failed to create client for new context")?;

        let mut state = self.inner.write().await;
        state.client = Some(client);
        state.current_context = Some(context_name.to_string());

        Ok(())
    }

    pub async fn list_namespaces(&self) -> Result<Vec<String>> {
        use k8s_openapi::api::core::v1::Namespace;
        use kube::api::{Api, ListParams};

        let client = self
            .get_client()
            .await
            .context("No Kubernetes client available")?;
        let namespaces: Api<Namespace> = Api::all(client);
        let ns_list = namespaces
            .list(&ListParams::default())
            .await
            .context("Failed to list namespaces")?;

        Ok(ns_list
            .items
            .into_iter()
            .filter_map(|ns| ns.metadata.name)
            .collect())
    }
}

impl Default for K8sClient {
    fn default() -> Self {
        Self::new()
    }
}
