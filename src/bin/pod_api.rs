use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{DeleteParams, PostParams},
    runtime::{conditions::is_pod_running, wait::await_condition},
    Api, Client, ResourceExt,
};
use serde_json::json;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let client = Client::try_default().await?;

    let api_version = client.apiserver_version().await?;
    info!("API version: {:?}", api_version);

    // Manage pods
    let pods: Api<Pod> = Api::default_namespaced(client);

    // Create pod blog
    let p: Pod = serde_json::from_value(json!({
        "apiVersion": "v1",
        "kind": "Pod",
        "metadata": { "name": "blog" },
        "spec": {
            "containers": [{
                "name": "blog",
                "image": "clux/blog:0.1.0"
            }],
        }
    }))?;

    let pp = PostParams::default();
    match pods.create(&pp, &p).await {
        Ok(o) => {
            let name = o.name_any();
            assert_eq!(p.name_any(), name);
            info!("Created {}", name);
        }
        Err(kube::Error::Api(ae)) => assert_eq!(ae.code, 409), // if you skipped delete for exmaple
        Err(e) => return Err(e.into()),
    }

    let establish = await_condition(pods.clone(), "blog", is_pod_running());
    let _ = tokio::time::timeout(std::time::Duration::from_secs(15), establish).await?;

    // Verify we can get it
    info!("Get Pod blog");
    let _p1 = pods.get("blog").await?;
    if let Some(spec) = &_p1.spec {
        info!("Got blog pod with containers: {:?}", spec.containers);
        assert_eq!(spec.containers[0].name, "blog");
    }

    // Delete it
    let dp = DeleteParams::default();
    pods.delete("blog", &dp).await?.map_left(|pdel| {
        assert_eq!(pdel.name_any(), "blog");
        info!("Deleting blog pod started: {:?}", pdel);
    });

    Ok(())
}
