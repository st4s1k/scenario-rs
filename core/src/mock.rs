use std::path::Path;

pub trait SessionTrait {
    fn channel_session(&self) -> Result<Box<dyn ChannelTrait>, ssh2::Error>;
    fn sftp(&self) -> Result<Box<dyn SftpTrait>, ssh2::Error>;
}

pub trait ChannelTrait {
    fn exec(&mut self, command: &str) -> Result<(), ssh2::Error>;
    fn read_to_string(&mut self, output: &mut String) -> Result<usize, ssh2::Error>;
    fn exit_status(&self) -> Result<i32, ssh2::Error>;
}

pub trait SftpTrait {
    fn create(&self, path: &Path) -> Result<Box<dyn WriteTrait>, ssh2::Error>;
}

pub trait WriteTrait {
    fn write_all(&mut self, buf: &[u8]) -> Result<(), ssh2::Error>;
}

impl SessionTrait for ssh2::Session {
    fn channel_session(&self) -> Result<Box<dyn ChannelTrait>, ssh2::Error> {
        self.channel_session()
            .map(|ch| Box::new(ch) as Box<dyn ChannelTrait>)
    }

    fn sftp(&self) -> Result<Box<dyn SftpTrait>, ssh2::Error> {
        self.sftp().map(|sftp| Box::new(sftp) as Box<dyn SftpTrait>)
    }
}

impl ChannelTrait for ssh2::Channel {
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

impl SftpTrait for ssh2::Sftp {
    fn create(&self, path: &Path) -> Result<Box<dyn WriteTrait>, ssh2::Error> {
        self.create(path)
            .map(|file| Box::new(SshFileWrapper(file)) as Box<dyn WriteTrait>)
    }
}

struct SshFileWrapper(ssh2::File);

impl WriteTrait for SshFileWrapper {
    fn write_all(&mut self, buf: &[u8]) -> Result<(), ssh2::Error> {
        use std::io::Write;
        self.0
            .write_all(buf)
            .map_err(|_| ssh2::Error::from_errno(ssh2::ErrorCode::Session(libc::EIO)))
    }
}

#[cfg(debug_assertions)]
pub mod debug {
    use super::*;

    pub struct MockSession;

    impl SessionTrait for MockSession {
        fn channel_session(&self) -> Result<Box<dyn ChannelTrait>, ssh2::Error> {
            std::thread::sleep(std::time::Duration::from_millis(100));
            Ok(Box::new(MockChannel))
        }

        fn sftp(&self) -> Result<Box<dyn SftpTrait>, ssh2::Error> {
            std::thread::sleep(std::time::Duration::from_millis(100));
            Ok(Box::new(MockSftp))
        }
    }

    pub struct MockChannel;

    impl ChannelTrait for MockChannel {
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

    impl SftpTrait for MockSftp {
        fn create(&self, _path: &Path) -> Result<Box<dyn WriteTrait>, ssh2::Error> {
            std::thread::sleep(std::time::Duration::from_millis(100));
            Ok(Box::new(MockFile))
        }
    }

    pub struct MockFile;

    impl WriteTrait for MockFile {
        fn write_all(&mut self, _buf: &[u8]) -> Result<(), ssh2::Error> {
            std::thread::sleep(std::time::Duration::from_millis(100));
            Ok(())
        }
    }

    pub fn new_mock_session() -> Box<dyn SessionTrait> {
        std::thread::sleep(std::time::Duration::from_millis(100));
        Box::new(MockSession)
    }
}

#[cfg(not(debug_assertions))]
pub fn get_session(session: &ssh2::Session) -> &dyn SessionTrait {
    session
}

#[cfg(debug_assertions)]
pub fn get_session(_: &ssh2::Session) -> Box<dyn SessionTrait> {
    debug::new_mock_session()
}
