use core::arch::global_asm;

global_asm!(include_str!("vmlaunch.asm"));

extern "C" {
    pub fn launch_vm(host_rsp: u64);
}
