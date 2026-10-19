//! The native event loop, window and drawing callbacks have one OS-thread owner.
use std::{marker::PhantomData, rc::Rc, thread::ThreadId};

pub(crate) struct GraphicsThread {
    id: ThreadId,
    // Neither this token nor an application containing it can enter a worker.
    _not_send_or_sync: PhantomData<Rc<()>>,
}
impl GraphicsThread {
    pub fn enter() -> std::io::Result<Self> {
        #[cfg(target_os = "linux")]
        {
            let task = std::fs::read_link("/proc/thread-self")?;
            let leader = std::process::id().to_string();
            if task.file_name() != Some(std::ffi::OsStr::new(&leader)) {
                return Err(std::io::Error::other(
                    "SSK window/event/render loop must start on the process main thread",
                ));
            }
        }
        let owner = Self {
            id: std::thread::current().id(),
            _not_send_or_sync: PhantomData,
        };
        tracing::info!(name:"ssk.client.graphics.thread_owner",thread_id=?owner.id,
            "Window, event loop and renderer share the process main thread");
        Ok(owner)
    }
    pub fn assert_current(&self) {
        assert_eq!(
            self.id,
            std::thread::current().id(),
            "graphics thread changed"
        );
    }
}
impl Drop for GraphicsThread {
    fn drop(&mut self) {
        self.assert_current();
    }
}
