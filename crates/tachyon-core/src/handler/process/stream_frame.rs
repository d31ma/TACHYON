use super::ProcessFailure;
use crate::Failure;
use crate::handler::frame::FrameLimits;
use tokio::io::AsyncReadExt;
use tokio::process::ChildStdout;

pub(super) struct StreamFrame;

impl StreamFrame {
    pub(super) async fn read(stdout: &mut ChildStdout) -> Result<Option<Vec<u8>>, Failure> {
        let mut prefix = [0_u8; FrameLimits::PREFIX_BYTES];
        let started = Self::read_first_byte(stdout, &mut prefix).await?;
        Self::read_rest(stdout, prefix, started).await
    }

    async fn read_first_byte(
        stdout: &mut ChildStdout,
        prefix: &mut [u8; FrameLimits::PREFIX_BYTES],
    ) -> Result<bool, Failure> {
        match stdout.read(&mut prefix[..1]).await {
            Ok(0) => Ok(false),
            Ok(_) => Ok(true),
            Err(_) => Err(ProcessFailure::stream(
                "Cannot read a streamed frame prefix.",
            )),
        }
    }

    async fn read_rest(
        stdout: &mut ChildStdout,
        prefix: [u8; FrameLimits::PREFIX_BYTES],
        started: bool,
    ) -> Result<Option<Vec<u8>>, Failure> {
        if !started {
            return Ok(None);
        }
        Self::read_payload(stdout, prefix).await.map(Some)
    }

    async fn read_payload(
        stdout: &mut ChildStdout,
        mut prefix: [u8; FrameLimits::PREFIX_BYTES],
    ) -> Result<Vec<u8>, Failure> {
        stdout.read_exact(&mut prefix[1..]).await.map_err(|_| {
            ProcessFailure::stream("A streamed frame ended before its prefix was complete.")
        })?;
        let length = u32::from_be_bytes(prefix) as usize;
        Self::check_length(length)?;
        let mut payload = vec![0_u8; length];
        stdout.read_exact(&mut payload).await.map_err(|_| {
            ProcessFailure::stream("A streamed frame ended before its payload was complete.")
        })?;
        Ok(payload)
    }

    fn check_length(length: usize) -> Result<(), Failure> {
        if length > FrameLimits::MAX_BYTES {
            return Err(ProcessFailure::stream(
                "A streamed frame exceeds the 16 MiB protocol limit.",
            ));
        }
        Ok(())
    }
}
