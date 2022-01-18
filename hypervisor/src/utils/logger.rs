use alloc::format;
use log::{Metadata, Record};
use winapi::km::wdm::DbgPrint;

pub struct KernelLogger;

impl log::Log for KernelLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let message = format!("{} - {}\n\0", record.level(), record.args());

            unsafe { DbgPrint(message.as_ptr()) };
        }
    }

    fn flush(&self) {}
}
