#!/bin/bash
# Create directories required by SFTP test scenarios
mkdir -p /home/test_user/uploads
mkdir -p /home/test_user/deploy/config
mkdir -p /home/test_user/deploy/scripts
mkdir -p /etc/myapp

chown -R test_user:test_user /home/test_user/uploads /home/test_user/deploy
chmod 777 /etc/myapp

# Install public key for key-based auth testing
mkdir -p /home/test_user/.ssh
cp /test_key.pub /home/test_user/.ssh/authorized_keys
chown -R test_user:test_user /home/test_user/.ssh
chmod 700 /home/test_user/.ssh
chmod 600 /home/test_user/.ssh/authorized_keys

# Add KEX algorithms compatible with libssh2 (used by ssh2 Rust crate).
# OpenSSH 10+ drops older algorithms by default; libssh2 needs them.
SSHD_CONF="/config/sshd/sshd_config"
if ! grep -q '^KexAlgorithms' "$SSHD_CONF" 2>/dev/null; then
  echo 'KexAlgorithms curve25519-sha256,curve25519-sha256@libssh.org,ecdh-sha2-nistp256,ecdh-sha2-nistp384,ecdh-sha2-nistp521,diffie-hellman-group14-sha256,diffie-hellman-group16-sha512,diffie-hellman-group18-sha512' >> "$SSHD_CONF"
fi
if ! grep -q '^PubkeyAuthentication' "$SSHD_CONF" 2>/dev/null; then
  echo 'PubkeyAuthentication yes' >> "$SSHD_CONF"
fi
