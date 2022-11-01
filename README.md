# amd_hypervisor

 AMD Hypervisor written writh Rust. 
 
 ## Features
 
 - [Easily define your own vmexit handlers](https://github.com/stars/not-matthias/lists/hypervisor)
    - MSR (read/write)
    - Cpuid
    - Rdtsc
    - and all the other vmexits
- [NPT Hooking](https://github.com/not-matthias/amd_hypervisor/blob/main/driver/src/handlers/npf.rs) 
- Memory safe and blazingly fast :rocket:
 
 ## Example
 
 See [driver/](./driver) for a reference implementation. Notes on how to write a kernel driver can be found [here](https://not-matthias.github.io/posts/kernel-driver-with-rust/).
 
 ## References
 
 - AMD Manual
 - Intel Manual
 - [SimpleSvm](https://github.com/tandasat/SimpleSvm)
 - Other projects from [this list](https://github.com/stars/not-matthias/lists/hypervisor)
