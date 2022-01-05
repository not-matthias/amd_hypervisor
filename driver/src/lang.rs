use crate::{dbg_break, KeBugCheck, MANUALLY_INITIATED_CRASH};
use core::panic::PanicInfo;

#[no_mangle]
#[allow(bad_style)]
static _fltused: i32 = 0;

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    log::info!("Panic handler called: {:?}", _info);

    dbg_break!();

    unsafe { KeBugCheck(MANUALLY_INITIATED_CRASH) };
}

#[lang = "eh_personality"]
extern "C" fn eh_personality() {}

#[no_mangle]
extern "C" fn __CxxFrameHandler3() {}
