use scenario_rs_core::config::scenario::ScenarioConfig;
use scenario_rs_core::scenario::errors::ScenarioConfigError;
use std::path::PathBuf;

fn example_config(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("example_configs")
        .join(name)
}

fn test_scenario(name: &str) -> PathBuf {
    example_config(&format!("test-scenarios/{name}"))
}

#[test]
fn load_example_scenario() {
    // Given & When & Then
    let config = ScenarioConfig::try_from(example_config("example-scenario.toml"));
    assert!(config.is_ok(), "failed to load example-scenario.toml: {:?}", config.err());
}

#[test]
fn load_deploy_scenario() {
    // Given & When & Then
    let config = ScenarioConfig::try_from(example_config("deploy-scenario.toml"));
    assert!(config.is_ok(), "failed to load deploy-scenario.toml: {:?}", config.err());
}

#[test]
fn load_child_with_parent_inheritance() {
    // Given & When
    let config = ScenarioConfig::try_from(example_config("deploy-service-one.toml"));

    // Then
    assert!(config.is_ok(), "failed to load deploy-service-one.toml: {:?}", config.err());
    let config = config.unwrap();
    assert_eq!(config.server.host, "service-one.example.com");
    assert_eq!(config.server.port, Some(23));
    assert_eq!(config.credentials.username, "my_username_service_one");
}

#[test]
fn load_second_child_with_parent_inheritance() {
    // Given & When
    let config = ScenarioConfig::try_from(example_config("deploy-service-two.toml"));

    // Then
    assert!(config.is_ok(), "failed to load deploy-service-two.toml: {:?}", config.err());
    let config = config.unwrap();
    assert_eq!(config.server.host, "service-two.example.com");
}

#[test]
fn load_test_scenario_all_succeed() {
    // Given & When & Then
    let config = ScenarioConfig::try_from(test_scenario("all-succeed.toml"));
    assert!(config.is_ok(), "failed: {:?}", config.err());
}

#[test]
fn load_test_scenario_sftp_then_sudo() {
    // Given & When & Then
    let config = ScenarioConfig::try_from(test_scenario("sftp-then-sudo.toml"));
    assert!(config.is_ok(), "failed: {:?}", config.err());
}

#[test]
fn load_test_scenario_only_sftp() {
    // Given & When & Then
    let config = ScenarioConfig::try_from(test_scenario("only-sftp-steps.toml"));
    assert!(config.is_ok(), "failed: {:?}", config.err());
}

#[test]
fn load_test_scenario_only_sudo() {
    // Given & When & Then
    let config = ScenarioConfig::try_from(test_scenario("only-sudo-steps.toml"));
    assert!(config.is_ok(), "failed: {:?}", config.err());
}

#[test]
fn load_test_scenario_empty_steps() {
    // Given & When & Then
    let config = ScenarioConfig::try_from(test_scenario("empty-steps.toml"));
    assert!(config.is_ok(), "failed: {:?}", config.err());
}

#[test]
fn load_test_scenario_with_on_fail() {
    // Given & When & Then
    let config = ScenarioConfig::try_from(test_scenario("fail-with-on-fail-succeed.toml"));
    assert!(config.is_ok(), "failed: {:?}", config.err());
}

#[test]
fn load_test_scenario_many_on_fail() {
    // Given & When & Then
    let config = ScenarioConfig::try_from(test_scenario("many-on-fail-steps.toml"));
    assert!(config.is_ok(), "failed: {:?}", config.err());
}

#[test]
fn missing_file_returns_cannot_open_config() {
    // Given & When & Then
    let result = ScenarioConfig::try_from(PathBuf::from("nonexistent.toml"));
    assert!(
        matches!(result, Err(ScenarioConfigError::CannotOpenConfig(_))),
        "expected CannotOpenConfig, got: {:?}",
        result
    );
}

#[test]
fn invalid_toml_returns_cannot_read_config() {
    // Given
    let dir = std::env::temp_dir().join("scenario_rs_test_invalid_toml");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("bad.toml");
    std::fs::write(&path, "this is not valid [[[toml content").unwrap();

    // When
    let result = ScenarioConfig::try_from(path.clone());

    // Then
    assert!(
        matches!(result, Err(ScenarioConfigError::CannotReadConfig(_))),
        "expected CannotReadConfig, got: {:?}",
        result
    );
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir(&dir);
}

#[test]
fn missing_credentials_returns_error() {
    // Given
    let dir = std::env::temp_dir().join("scenario_rs_test_missing_creds");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("no-creds.toml");
    std::fs::write(
        &path,
        r#"
[server]
host = "localhost"

[execute]
steps = [{ task = "t" }]

[tasks.t]
type = "RemoteSudo"
description = "d"
command = "echo hi"
error_message = "e"
"#,
    )
    .unwrap();

    // When
    let result = ScenarioConfig::try_from(path.clone());

    // Then
    assert!(
        matches!(result, Err(ScenarioConfigError::MissingCredentials)),
        "expected MissingCredentials, got: {:?}",
        result
    );
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir(&dir);
}

#[test]
fn missing_server_returns_error() {
    // Given
    let dir = std::env::temp_dir().join("scenario_rs_test_missing_server");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("no-server.toml");
    std::fs::write(
        &path,
        r#"
[credentials]
username = "user"

[execute]
steps = [{ task = "t" }]

[tasks.t]
type = "RemoteSudo"
description = "d"
command = "echo hi"
error_message = "e"
"#,
    )
    .unwrap();

    // When
    let result = ScenarioConfig::try_from(path.clone());

    // Then
    assert!(
        matches!(result, Err(ScenarioConfigError::MissingServer)),
        "expected MissingServer, got: {:?}",
        result
    );
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir(&dir);
}

#[test]
fn missing_execute_returns_error() {
    // Given
    let dir = std::env::temp_dir().join("scenario_rs_test_missing_execute");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("no-exec.toml");
    std::fs::write(
        &path,
        r#"
[credentials]
username = "user"

[server]
host = "localhost"

[tasks.t]
type = "RemoteSudo"
description = "d"
command = "echo hi"
error_message = "e"
"#,
    )
    .unwrap();

    // When
    let result = ScenarioConfig::try_from(path.clone());

    // Then
    assert!(
        matches!(result, Err(ScenarioConfigError::MissingExecute)),
        "expected MissingExecute, got: {:?}",
        result
    );
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir(&dir);
}

#[test]
fn missing_tasks_returns_error() {
    // Given
    let dir = std::env::temp_dir().join("scenario_rs_test_missing_tasks");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("no-tasks.toml");
    std::fs::write(
        &path,
        r#"
[credentials]
username = "user"

[server]
host = "localhost"

[execute]
steps = [{ task = "t" }]
"#,
    )
    .unwrap();

    // When
    let result = ScenarioConfig::try_from(path.clone());

    // Then
    assert!(
        matches!(result, Err(ScenarioConfigError::MissingTasks)),
        "expected MissingTasks, got: {:?}",
        result
    );
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir(&dir);
}

#[test]
fn circular_parent_returns_error() {
    // Given
    let dir = std::env::temp_dir().join("scenario_rs_test_circular");
    std::fs::create_dir_all(&dir).unwrap();
    let a_path = dir.join("a.toml");
    let b_path = dir.join("b.toml");
    let b_toml_path = b_path.display().to_string().replace('\\', "/");
    let a_toml_path = a_path.display().to_string().replace('\\', "/");
    std::fs::write(&a_path, format!("parent = \"{b_toml_path}\"")).unwrap();
    std::fs::write(&b_path, format!("parent = \"{a_toml_path}\"")).unwrap();

    // When
    let result = ScenarioConfig::try_from(a_path.clone());

    // Then
    assert!(
        matches!(result, Err(ScenarioConfigError::CircularDependency(_))),
        "expected CircularDependency, got: {:?}",
        result
    );
    let _ = std::fs::remove_file(&a_path);
    let _ = std::fs::remove_file(&b_path);
    let _ = std::fs::remove_dir(&dir);
}

#[test]
fn parent_not_found_returns_error() {
    // Given
    let dir = std::env::temp_dir().join("scenario_rs_test_parent_not_found");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("child.toml");
    std::fs::write(&path, "parent = \"./nonexistent-parent.toml\"").unwrap();

    // When
    let result = ScenarioConfig::try_from(path.clone());

    // Then
    assert!(
        matches!(result, Err(ScenarioConfigError::ParentConfigNotFound(_))),
        "expected ParentConfigNotFound, got: {:?}",
        result
    );
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir(&dir);
}
