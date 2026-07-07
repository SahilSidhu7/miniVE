// Requires a running Docker daemon: cargo test -- --ignored
use bollard::Docker;

#[tokio::test]
#[ignore]
async fn env_lifecycle_against_real_docker() {
    let docker = Docker::connect_with_local_defaults().unwrap();
    docker.ping().await.expect("docker must be running for this test");
    // Pull tiny image, create+start container the way env_manager does, exec echo, tear down.
    use bollard::container::{Config, CreateContainerOptions, RemoveContainerOptions, StartContainerOptions};
    use bollard::exec::{CreateExecOptions, StartExecResults};
    use bollard::image::CreateImageOptions;
    use futures_util::StreamExt;

    let mut pull = docker.create_image(
        Some(CreateImageOptions { from_image: "alpine:3.20".to_string(), ..Default::default() }),
        None,
        None,
    );
    while let Some(i) = pull.next().await { i.unwrap(); }

    let name = "minive-inttest";
    let _ = docker.remove_container(name, Some(RemoveContainerOptions { force: true, ..Default::default() })).await;
    docker
        .create_container(
            Some(CreateContainerOptions { name: name.to_string(), platform: None }),
            Config::<String> {
                image: Some("alpine:3.20".into()),
                cmd: Some(vec!["sleep".into(), "infinity".into()]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    docker.start_container(name, None::<StartContainerOptions<String>>).await.unwrap();

    let exec = docker
        .create_exec(name, CreateExecOptions::<String> {
            attach_stdout: Some(true),
            cmd: Some(vec!["echo".into(), "hi".into()]),
            ..Default::default()
        })
        .await
        .unwrap();
    let mut out = String::new();
    if let StartExecResults::Attached { mut output, .. } = docker.start_exec(&exec.id, None).await.unwrap() {
        while let Some(Ok(msg)) = output.next().await {
            out.push_str(&String::from_utf8_lossy(&msg.into_bytes()));
        }
    }
    assert!(out.contains("hi"));

    docker.remove_container(name, Some(RemoveContainerOptions { force: true, ..Default::default() })).await.unwrap();
}
