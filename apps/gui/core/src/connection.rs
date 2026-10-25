//! A write lock plus an independent shutdown handle. Closing an attachment
//! must not wait behind the socket write it is meant to interrupt.

use std::{
    net::{Shutdown, TcpStream},
    ops::Deref,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

pub(crate) struct Connection {
    writer: Mutex<TcpStream>,
    interrupt: Mutex<TcpStream>,
    closed: AtomicBool,
}

impl Connection {
    pub(crate) fn new(stream: TcpStream) -> Result<Self, String> {
        let interrupt = stream.try_clone().map_err(|e| e.to_string())?;
        Ok(Self {
            writer: Mutex::new(stream),
            interrupt: Mutex::new(interrupt),
            closed: AtomicBool::new(false),
        })
    }

    pub(crate) fn close(&self) {
        self.closed.store(true, Ordering::Release);
        let stream = self
            .interrupt
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ = stream.shutdown(Shutdown::Both);
    }

    /// Rejoin may finish after the last holder has left. Never install that
    /// late connection, or let it survive an already completed close.
    pub(crate) fn replace(&self, stream: TcpStream) -> Result<(), String> {
        let interrupt = stream.try_clone().map_err(|e| e.to_string())?;
        // A reader has lost the old stream. Interrupt any writer still stuck
        // on it before waiting for its lock; close can run concurrently.
        {
            let old = self
                .interrupt
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let _ = old.shutdown(Shutdown::Both);
        }
        let mut writer = self
            .writer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut old = self
            .interrupt
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if self.closed.load(Ordering::Acquire) {
            let _ = stream.shutdown(Shutdown::Both);
            return Err("attachment is closed".into());
        }
        *writer = stream;
        *old = interrupt;
        Ok(())
    }
}

impl Deref for Connection {
    type Target = Mutex<TcpStream>;
    fn deref(&self) -> &Self::Target {
        &self.writer
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.close();
    }
}
