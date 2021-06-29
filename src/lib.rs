use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct File {
    path: PathBuf,
    file: fs::File,
    should_rotate: Arc<AtomicBool>,
    sig_id: signal_hook::SigId,
}

impl File {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, io::Error> {
        let flag = Arc::default();
        let file = Self::do_open(&path)?;

        let sig_id =
            signal_hook::flag::register(signal_hook::consts::signal::SIGHUP, Arc::clone(&flag))?;

        let file = File {
            path: path.as_ref().to_owned(),
            file,
            should_rotate: Arc::clone(&flag),
            sig_id,
        };
        Ok(file)
    }

    fn do_open(path: impl AsRef<Path>) -> Result<fs::File, io::Error> {
        fs::OpenOptions::new().append(true).create(true).open(path)
    }

    fn reopen_if_needed(&mut self) -> Result<(), io::Error> {
        if self.should_rotate.swap(false, Ordering::Relaxed) {
            self.file.sync_all()?;
            self.file = Self::do_open(&self.path)?;
        }
        Ok(())
    }
}

impl Drop for File {
    fn drop(&mut self) {
        signal_hook::low_level::unregister(self.sig_id);
    }
}

impl Write for File {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.reopen_if_needed()?;
        self.file.write(buf)
    }

    fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
        self.reopen_if_needed()?;
        self.file.write_vectored(bufs)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}
