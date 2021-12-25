use x86::bits64::paging::PML4Flags;

fn main() {
    let mut flags = PML4Flags::empty();
    flags.set(PML4Flags::P, true);
    flags.set(PML4Flags::RW, true);
    flags.set(PML4Flags::US, true);
    println!("flags: {:?}", flags);

    let flags = PML4Flags::from_iter([PML4Flags::P, PML4Flags::RW, PML4Flags::US]);
    println!("flags: {:?}", flags);
}