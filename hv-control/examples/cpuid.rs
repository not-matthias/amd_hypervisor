use x86::cpuid::CpuId;
use core::arch::asm;

fn main() {
    CpuId::new().get_hypervisor_info().map(|hv_info| {
        println!("Hypervisor info: {:?}", hv_info);
        println!("Hypervisor info: {:?}", hv_info.identify());
    });

    println!();
    CpuId::new().get_svm_info().map(|svm_info| {
        println!("SVM info: {:?}", svm_info);
        println!("SVM info - has_svm_lock: {:?}", svm_info.has_svm_lock());
    });

    println!();
    CpuId::new().get_extended_processor_and_feature_identifiers().map(|proc_info| {
        println!("Proc info: {:?}", proc_info);
        println!("Proc info - has_svm: {:?}", proc_info.has_svm());
    });

}