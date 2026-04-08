# Test Scenarios

Test scenario configs for manually testing the GUI execution flow against a real SSH server.

## Setup

Start the SSH server (requires `docker restart` so the KEX config takes effect):

```bash
cd example_configs/test-scenarios
docker compose up -d
docker restart test-ssh
```

This starts an OpenSSH server on `localhost:2222`.
The container automatically creates directories needed by SFTP scenarios and installs the test public key via `init-dirs.sh`.

## Directory Structure

```
test-scenarios/
  docker-compose.yml, init-dirs.sh                    # Docker infra
  password-auth/                                      # Scenarios using password auth
    server.toml                                       # Parent: test_user / test_pass
    *.toml                                            # 10 scenario configs
  key-auth/                                           # Scenarios using private key auth
    server.toml                                       # Parent: test_user + private key
    test_key, test_key.pub                            # ED25519 keypair
    *.toml                                            # Scenario configs
  agent-auth/                                         # Scenarios using SSH agent auth
    server.toml                                       # Parent: test_user (no password/key)
    *.toml                                            # Scenario configs
```

## Password Auth Scenarios (`password-auth/`)

All inherit from `password-auth/server.toml` (credentials: `test_user` / `test_pass`).

| File | What it tests |
|------|--------------|
| `all-succeed.toml` | SftpCopy + RemoteSudo steps, all pass (requires selecting a local file) |
| `empty-steps.toml` | Empty steps list — "No steps to display" |
| `fail-with-on-fail-fail.toml` | Step fails, on-fail step also fails |
| `fail-with-on-fail-succeed.toml` | Step 1 passes, Step 2 fails with on-fail steps that succeed |
| `many-on-fail-steps.toml` | Step fails with 4 on-fail recovery steps |
| `multi-step-mid-fail.toml` | 5 steps, failure at step 3, steps 4-5 stay pending |
| `only-sftp-steps.toml` | 3 SftpCopy steps (requires selecting local files) |
| `only-sudo-steps.toml` | 3 RemoteSudo steps (`df -h`, `free -m`, service list) |
| `sftp-then-sudo.toml` | SftpCopy then RemoteSudo (requires selecting a local file) |
| `single-sudo-fail.toml` | Single step with unresolved variable — fails immediately |

## Key Auth Scenarios (`key-auth/`)

All inherit from `key-auth/server.toml` (credentials: `test_user` + private key `test_key`).

| File | What it tests |
|------|--------------|
| `only-sudo-steps.toml` | 3 RemoteSudo steps with key-based auth |
| `sftp-then-sudo.toml` | SftpCopy + RemoteSudo with key-based auth (requires selecting a local file) |

## Agent Auth Scenarios (`agent-auth/`)

All inherit from `agent-auth/server.toml` (credentials: `test_user`, no password or key — uses SSH agent).

Requires `ssh-agent` with the test key loaded:

```bash
eval $(ssh-agent)
ssh-add example_configs/test-scenarios/key-auth/test_key
```

| File | What it tests |
|------|---------------|
| `only-sudo-steps.toml` | 3 RemoteSudo steps with agent-based auth |
| `sftp-then-sudo.toml` | SftpCopy + RemoteSudo with agent-based auth (requires selecting a local file) |

## SFTP Scenarios

Scenarios that use SftpCopy require selecting local files to upload via the variable prompts. Any file will work.

## Teardown

```bash
docker compose down
```
