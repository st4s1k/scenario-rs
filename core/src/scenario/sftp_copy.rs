use crate::{
    config::SftpCopyConfig,
    scenario::{errors::SftpCopyError, utils::SendEvent, variables::Variables},
    session::Session,
};
use std::{fs::File, io::Read, path::Path, sync::mpsc::Sender};

use super::events::Event;

#[derive(Debug, Clone)]
pub struct SftpCopy {
    pub(crate) source_path: String,
    pub(crate) destination_path: String,
}

impl From<&SftpCopyConfig> for SftpCopy {
    fn from(config: &SftpCopyConfig) -> Self {
        SftpCopy {
            source_path: config.source_path.clone(),
            destination_path: config.destination_path.clone(),
        }
    }
}

impl SftpCopy {
    pub fn source_path(&self) -> &str {
        &self.source_path
    }

    pub fn destination_path(&self) -> &str {
        &self.destination_path
    }

    pub(crate) fn execute(
        &self,
        session: &Session,
        variables: &Variables,
        tx: &Sender<Event>,
    ) -> Result<(), SftpCopyError> {
        let resolved_source = variables
            .resolve_placeholders(&self.source_path)
            .map_err(SftpCopyError::CannotResolveSourcePathPlaceholders)?;
        let resolved_destination = variables
            .resolve_placeholders(&self.destination_path)
            .map_err(SftpCopyError::CannotResolveDestinationPathPlaceholders)?;

        tx.send_event(Event::SftpCopyBefore {
            source: resolved_source.clone(),
            destination: resolved_destination.clone(),
        });

        let mut source_file =
            File::open(&resolved_source).map_err(SftpCopyError::CannotOpenSourceFile)?;

        let sftp = session
            .sftp()
            .map_err(SftpCopyError::CannotOpenChannelAndInitializeSftp)?;

        let mut destination_file = sftp
            .create(Path::new(&resolved_destination))
            .map_err(SftpCopyError::CannotCreateDestinationFile)?;

        let total_bytes = source_file
            .metadata()
            .map_err(SftpCopyError::CannotReadSourceFile)?
            .len();

        let mut current_bytes = 0u64;
        let mut buffer = [0u8; 8192];
        loop {
            let bytes_read = source_file
                .read(&mut buffer)
                .map_err(SftpCopyError::CannotReadSourceFile)?;
            if bytes_read == 0 {
                break;
            }

            destination_file
                .write_all(&buffer[..bytes_read])
                .map_err(SftpCopyError::CannotWriteDestinationFile)?;

            current_bytes += bytes_read as u64;

            tx.send_event(Event::SftpCopyProgress {
                current: current_bytes,
                total: total_bytes,
            });
        }

        tx.send_event(Event::SftpCopyAfter);

        Ok(())
    }
}
