use crate::{
    config::SftpCopyConfig,
    scenario::{
        errors::SftpCopyError,
        lifecycle::SftpCopyLifecycle,
        variables::Variables,
    },
};
use indicatif::ProgressBar;
use ssh2::Session;
use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};

pub struct SftpCopy {
    pub(crate) source_path: String,
    pub(crate) destination_path: String,
}

impl TryFrom<(&SftpCopyConfig, &Variables)> for SftpCopy {
    type Error = SftpCopyError;

    fn try_from((config, variables): (&SftpCopyConfig, &Variables)) -> Result<Self, Self::Error> {
        let source_path = variables.resolve_placeholders(&config.source_path)
            .map_err(SftpCopyError::CannotResolveSourcePathPlaceholders)?;
        let destination_path = variables.resolve_placeholders(&config.destination_path)
            .map_err(SftpCopyError::CannotResolveDestinationPathPlaceholders)?;

        Ok(SftpCopy { source_path, destination_path })
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
        lifecycle: &mut SftpCopyLifecycle,
    ) -> Result<(), SftpCopyError> {
        (lifecycle.before)(&self);

        let sftp = session.sftp()
            .map_err(SftpCopyError::CannotOpenChannelAndInitializeSftp)?;

        let mut source_file = File::open(&self.source_path)
            .map_err(SftpCopyError::CannotOpenSourceFile)?;
        let mut destination_file = sftp.create(Path::new(&self.destination_path))
            .map_err(SftpCopyError::CannotCreateDestinationFile)?;

        let pb = ProgressBar::hidden();

        (lifecycle.files_ready)(&source_file, &mut destination_file, &pb);

        let mut copy_buffer = Vec::new();

        source_file.read_to_end(&mut copy_buffer)
            .map_err(SftpCopyError::CannotReadSourceFile)?;

        pb.wrap_write(destination_file).write_all(&copy_buffer)
            .map_err(SftpCopyError::CannotWriteDestinationFile)?;

        pb.finish();

        (lifecycle.after)();

        Ok(())
    }
}