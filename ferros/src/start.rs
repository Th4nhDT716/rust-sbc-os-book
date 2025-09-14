use super::main;

#[unsafe(naked)]
#[unsafe(no_mangle)]
pub extern "C" fn start() {
    core::arch::naked_asm!(
        // check core ID, proceed only on core 0
        "mrs x0, MPIDR_EL1",
        "and x0, x0, 0b11",
        "cmp x0, 0",
        "b.eq 2f", // if this is core 1, jump to stack pointer setup
        // otherwise, fall into the infinite parking loop
        "1:",
        "wfe",
        "b 1b",
        // setup the stack pointer
        "2:",
        " mov sp, #0x1FFF0000",
        // zero the .bss section
        "ldr  x0, =bss_start",
        "ldr  x1, =bss_end",
        "1:",
        "cmp  x0, x1",
        "b.eq 1f",
        "str  xzr, [x0], #8",
        "b    1b",
        // jump to Rust main!
        "1:",
        "b {}", sym main
    );
}
