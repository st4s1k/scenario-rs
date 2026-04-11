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
    // Given & When
    let config = ScenarioConfig::try_from(example_config("example-scenario.toml"));

    // Then
    assert!(config.is_ok(), "failed to load example-scenario.toml: {:?}", config.err());
}

#[test]
fn load_deploy_scenario() {
    // Given & When
    let config = ScenarioConfig::try_from(example_config("deploy-scenario.toml"));

    // Then
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
    // Given & When
    let config = ScenarioConfig::try_from(test_scenario("password-auth/all-succeed.toml"));

    // Then
    assert!(config.is_ok(), "failed: {:?}", config.err());
}

#[test]
fn load_test_scenario_sftp_then_sudo() {
    // Given & When
    let config = ScenarioConfig::try_from(test_scenario("password-auth/sftp-then-sudo.toml"));

    // Then
    assert!(config.is_ok(), "failed: {:?}", config.err());
}

#[test]
fn load_test_scenario_only_sftp() {
    // Given & When
    let config = ScenarioConfig::try_from(test_scenario("password-auth/only-sftp-steps.toml"));

    // Then
    assert!(config.is_ok(), "failed: {:?}", config.err());
}

#[test]
fn load_test_scenario_only_sudo() {
    // Given & When
    let config = ScenarioConfig::try_from(test_scenario("password-auth/only-sudo-steps.toml"));

    // Then
    assert!(config.is_ok(), "failed: {:?}", config.err());
}

#[test]
fn load_test_scenario_empty_steps() {
    // Given & When
    let config = ScenarioConfig::try_from(test_scenario("password-auth/empty-steps.toml"));

    // Then
    assert!(config.is_ok(), "failed: {:?}", config.err());
}

#[test]
fn load_test_scenario_with_on_fail() {
    // Given & When
    let config = ScenarioConfig::try_from(test_scenario("password-auth/fail-with-on-fail-succeed.toml"));

    // Then
    assert!(config.is_ok(), "failed: {:?}", config.err());
}

#[test]
fn load_test_scenario_many_on_fail() {
    // Given & When
    let config = ScenarioConfig::try_from(test_scenario("password-auth/many-on-fail-steps.toml"));

    // Then
    assert!(config.is_ok(), "failed: {:?}", config.err());
}

#[test]
fn load_key_auth_scenario_only_sudo() {
    // Given & When
    let config = ScenarioConfig::try_from(test_scenario("key-auth/only-sudo-steps.toml"));

    // Then
    assert!(config.is_ok(), "failed: {:?}", config.err());
    let config = config.unwrap();
    assert_eq!(config.credentials.username, "test_user");
    assert!(config.credentials.password.is_none());
    assert!(config.credentials.private_key.is_some());
}

#[test]
fn load_key_auth_scenario_sftp_then_sudo() {
    // Given & When
    let config = ScenarioConfig::try_from(test_scenario("key-auth/sftp-then-sudo.toml"));

    // Then
    assert!(config.is_ok(), "failed: {:?}", config.err());
}

#[test]
fn load_agent_auth_scenario_only_sudo() {
    // Given & When
    let config = ScenarioConfig::try_from(test_scenario("agent-auth/only-sudo-steps.toml"));

    // Then
    assert!(config.is_ok(), "failed: {:?}", config.err());
    let config = config.unwrap();
    assert_eq!(config.credentials.username, "test_user");
    assert!(config.credentials.password.is_none());
    assert!(config.credentials.private_key.is_none());
}

#[test]
fn load_agent_auth_scenario_sftp_then_sudo() {
    // Given & When
    let config = ScenarioConfig::try_from(test_scenario("agent-auth/sftp-then-sudo.toml"));

    // Then
    assert!(config.is_ok(), "failed: {:?}", config.err());
}

#[test]
fn missing_file_returns_cannot_open_config() {
    // Given & When
    let result = ScenarioConfig::try_from(PathBuf::from("nonexistent.toml"));

    // Then
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

[steps.s1]
task = "t"

[tasks.remote_sudo.t]
command = "echo hi"
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

[steps.s1]
task = "t"

[tasks.remote_sudo.t]
command = "echo hi"
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
fn missing_steps_returns_error() {
    // Given
    let dir = std::env::temp_dir().join("scenario_rs_test_missing_steps");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("no-steps.toml");
    std::fs::write(
        &path,
        r#"
[credentials]
username = "user"

[server]
host = "localhost"

[tasks.remote_sudo.t]
command = "echo hi"
"#,
    )
    .unwrap();

    // When
    let result = ScenarioConfig::try_from(path.clone());

    // Then
    assert!(
        matches!(result, Err(ScenarioConfigError::MissingSteps)),
        "expected MissingSteps, got: {:?}",
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

[steps.s1]
task = "t"
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

#[test]
fn load_config_with_private_key_from_parent() {
    // Given
    let dir = std::env::temp_dir().join("scenario_rs_test_load_key_parent");
    std::fs::create_dir_all(&dir).unwrap();

    let parent_path = dir.join("server-key.toml");
    std::fs::write(
        &parent_path,
        r#"
[credentials]
username = "key_user"
private_key = "./my_key"

[server]
host = "localhost"
port = 2222
"#,
    )
    .unwrap();

    let child_path = dir.join("child.toml");
    std::fs::write(
        &child_path,
        r#"
parent = "./server-key.toml"

[steps.s1]
task = "t"

[tasks.remote_sudo.t]
command = "echo hi"
"#,
    )
    .unwrap();

    // When
    let config = ScenarioConfig::try_from(child_path.clone()).unwrap();

    // Then
    assert_eq!(config.credentials.username, "key_user");
    assert!(config.credentials.password.is_none());
    assert!(config.credentials.private_key.is_some());
    let key = config.credentials.private_key.unwrap();
    assert!(key.contains("my_key"), "expected path containing 'my_key', got: {key}");

    let _ = std::fs::remove_file(&child_path);
    let _ = std::fs::remove_file(&parent_path);
    let _ = std::fs::remove_dir(&dir);
}

#[test]
fn private_key_relative_path_resolved_to_config_dir() {
    // Given
    let dir = std::env::temp_dir().join("scenario_rs_test_key_resolve");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("key-scenario.toml");
    std::fs::write(
        &path,
        r#"
[credentials]
username = "user"
private_key = "./keys/id_ed25519"

[server]
host = "localhost"

[steps]

[tasks]
"#,
    )
    .unwrap();

    // When
    let config = ScenarioConfig::try_from(path.clone()).unwrap();

    // Then
    let key = config.credentials.private_key.unwrap();
    let expected_suffix = dir.join("keys").join("id_ed25519");
    assert_eq!(
        std::path::PathBuf::from(&key),
        expected_suffix,
        "expected key path resolved to {:?}, got: {key}",
        expected_suffix
    );
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir(&dir);
}

#[test]
fn private_key_absolute_path_stays_unchanged() {
    // Given
    let dir = std::env::temp_dir().join("scenario_rs_test_key_abs");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("key-abs.toml");
    let abs_key = if cfg!(windows) {
        "C:\\\\Users\\\\user\\\\.ssh\\\\id_ed25519"
    } else {
        "/home/user/.ssh/id_ed25519"
    };
    std::fs::write(
        &path,
        format!(
            r#"
[credentials]
username = "user"
private_key = "{abs_key}"

[server]
host = "localhost"

[steps]

[tasks]
"#
        ),
    )
    .unwrap();

    // When
    let config = ScenarioConfig::try_from(path.clone()).unwrap();

    // Then
    let key = config.credentials.private_key.unwrap();
    let expected = abs_key.replace("\\\\", "\\");
    assert_eq!(key, expected);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir(&dir);
}

#[test]
fn private_key_inherited_from_parent() {
    // Given
    let dir = std::env::temp_dir().join("scenario_rs_test_key_inherit");
    std::fs::create_dir_all(&dir).unwrap();

    let parent_path = dir.join("parent.toml");
    std::fs::write(
        &parent_path,
        r#"
[credentials]
username = "user"
private_key = "./parent_key"

[server]
host = "localhost"

[steps]

[tasks]
"#,
    )
    .unwrap();

    let child_path = dir.join("child.toml");
    std::fs::write(
        &child_path,
        r#"
parent = "./parent.toml"

[server]
host = "override-host"
"#,
    )
    .unwrap();

    // When
    let config = ScenarioConfig::try_from(child_path.clone()).unwrap();

    // Then
    assert!(config.credentials.private_key.is_some());
    let key = config.credentials.private_key.unwrap();
    assert!(key.contains("parent_key"));
    assert_eq!(config.server.host, "override-host");

    let _ = std::fs::remove_file(&child_path);
    let _ = std::fs::remove_file(&parent_path);
    let _ = std::fs::remove_dir(&dir);
}
