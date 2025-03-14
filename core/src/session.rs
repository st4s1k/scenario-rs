use std::{net::TcpStream, path::Path};

use crate::scenario::{credentials::Credentials, server::Server};

pub trait Channel {
    fn exec(&mut self, command: &str) -> Result<(), ssh2::Error>;
    fn read_to_string(&mut self, output: &mut String) -> Result<usize, ssh2::Error>;
    fn exit_status(&self) -> Result<i32, ssh2::Error>;
}

pub trait Sftp {
    fn create(&self, path: &Path) -> Result<Box<dyn Write>, ssh2::Error>;
}

pub trait Write {
    fn write_all(&mut self, buf: &[u8]) -> Result<(), ssh2::Error>;
}

pub struct Session {
    inner: SessionType,
}

enum SessionType {
    Real(ssh2::Session),
    Mock,
}

impl Session {
    pub fn new(server: &Server, credentials: &Credentials) -> Result<Self, ssh2::Error> {
        if cfg!(debug_assertions) {
            Self::create_mock_session(server, credentials)
        } else {
            Self::create_real_session(server, credentials)
        }
    }

    pub fn channel_session(&self) -> Result<Box<dyn Channel>, ssh2::Error> {
        match &self.inner {
            SessionType::Real(real_session) => real_session
                .channel_session()
                .map(|ch| Box::new(ch) as Box<dyn Channel>),
            SessionType::Mock => {
                std::thread::sleep(std::time::Duration::from_millis(100));
                Ok(Box::new(mock::MockChannel))
            }
        }
    }

    pub fn sftp(&self) -> Result<Box<dyn Sftp>, ssh2::Error> {
        match &self.inner {
            SessionType::Real(real_session) => real_session
                .sftp()
                .map(|sftp| Box::new(sftp) as Box<dyn Sftp>),
            SessionType::Mock => {
                std::thread::sleep(std::time::Duration::from_millis(100));
                Ok(Box::new(mock::MockSftp))
            }
        }
    }

    fn create_real_session(
        server: &Server,
        credentials: &Credentials,
    ) -> Result<Session, ssh2::Error> {
        let host = &server.host;
        let port = &server.port;

        let tcp = TcpStream::connect(&format!("{host}:{port}"))
            .map_err(|_| ssh2::Error::from_errno(ssh2::ErrorCode::Session(libc::EIO)))?;

        let mut real_session = ssh2::Session::new()?;
        real_session.set_tcp_stream(tcp);
        real_session.handshake()?;

        let username = &credentials.username;
        let password = &credentials.password.as_deref();

        match password {
            Some(pwd) => real_session.userauth_password(username, pwd)?,
            None => real_session.userauth_agent(username)?,
        }

        Ok(Session {
            inner: SessionType::Real(real_session),
        })
    }

    fn create_mock_session(
        server: &Server,
        credentials: &Credentials,
    ) -> Result<Session, ssh2::Error> {
        let host = &server.host;
        let port = &server.port;
        let username = &credentials.username;
        let password = &credentials.password.as_deref();

        println!(
            "Connecting to {host}:{port} as {username} with password {}",
            password.unwrap_or("<ssh-agent>")
        );

        std::thread::sleep(std::time::Duration::from_millis(100));

        Ok(Session {
            inner: SessionType::Mock,
        })
    }
}

impl Channel for ssh2::Channel {
    fn exec(&mut self, command: &str) -> Result<(), ssh2::Error> {
        self.exec(command)
    }

    fn read_to_string(&mut self, output: &mut String) -> Result<usize, ssh2::Error> {
        use std::io::Read;
        let mut buf = Vec::new();
        match self.read_to_end(&mut buf) {
            Ok(size) => {
                if let Ok(s) = String::from_utf8(buf) {
                    output.push_str(&s);
                    Ok(size)
                } else {
                    Err(ssh2::Error::from_errno(ssh2::ErrorCode::Session(libc::EIO)))
                }
            }
            Err(_) => Err(ssh2::Error::from_errno(ssh2::ErrorCode::Session(libc::EIO))),
        }
    }

    fn exit_status(&self) -> Result<i32, ssh2::Error> {
        self.exit_status()
    }
}

impl Sftp for ssh2::Sftp {
    fn create(&self, path: &Path) -> Result<Box<dyn Write>, ssh2::Error> {
        self.create(path)
            .map(|file| Box::new(SshFileWrapper(file)) as Box<dyn Write>)
    }
}

struct SshFileWrapper(ssh2::File);

impl Write for SshFileWrapper {
    fn write_all(&mut self, buf: &[u8]) -> Result<(), ssh2::Error> {
        use std::io::Write;
        self.0
            .write_all(buf)
            .map_err(|_| ssh2::Error::from_errno(ssh2::ErrorCode::Session(libc::EIO)))
    }
}

pub mod mock {
    use super::*;

    pub struct MockChannel;

    impl Channel for MockChannel {
        fn exec(&mut self, _command: &str) -> Result<(), ssh2::Error> {
            std::thread::sleep(std::time::Duration::from_millis(100));
            Ok(())
        }

        fn read_to_string(&mut self, output: &mut String) -> Result<usize, ssh2::Error> {
            let mock_output = "Mock command output\nLine 1\nLine 2\nLine 3\n";
            output.push_str(mock_output);
            std::thread::sleep(std::time::Duration::from_millis(100));
            Ok(mock_output.len())
        }

        fn exit_status(&self) -> Result<i32, ssh2::Error> {
            std::thread::sleep(std::time::Duration::from_millis(100));
            Ok(0)
        }
    }

    pub struct MockSftp;

    impl Sftp for MockSftp {
        fn create(&self, _path: &Path) -> Result<Box<dyn Write>, ssh2::Error> {
            std::thread::sleep(std::time::Duration::from_millis(100));
            Ok(Box::new(MockFile))
        }
    }

    pub struct MockFile;

    impl Write for MockFile {
        fn write_all(&mut self, _buf: &[u8]) -> Result<(), ssh2::Error> {
            std::thread::sleep(std::time::Duration::from_millis(100));
            Ok(())
        }
    }
}
