use ntapi::ntexapi::{NtQuerySystemInformation, SystemProcessInformation};

fn main() {
    let test = unsafe { NtQuerySystemInformation(SystemProcessInformation, 0 as _, 0, 0 as _) };
}
