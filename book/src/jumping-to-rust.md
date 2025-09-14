# Jumping to Rust

In this chapter, we will do necessary setup that prepares the system for calling
a normal Rust function. Specifically, we will set up the stack and zero any data
that our Rust code expects to start zeroed.

## Setting up the Stack

First, we set up the stack. What actually the stack is is out of scope of this
book, but it should suffice for now to say that stack is a data structure in
memory where functions place data they need to store for a while (by pushing
them onto the stack) and which they then retrieve when they need them again
(by popping them off the stack).

It is the responsibility of the kernel to set up the stack for use by its own
functions (and later on, to set up stacks for programs that will run on the
operating system using the kernel). Let's set it up then!

The stack, being a data structure, needs place to "grow" - as data will be
pushed onto it, it will increase in size. It is customary (and assumed by `rustc`
when copiling code) that the stack grows "downwards", i.e. from larger addresses
to smaller addresses.

## A Nice Place for the Stack

Since the stack grows downwards, it would not be a bad idea to place it just
below the RAM top, that is at (almost) the highest available RAM address. Most
SBCs have relatively small RAMs, and since our kernel will not be too complex,
we will assume a 512 MiB large RAM. Bare metal developers usually don't place
the stack at the extreme end of RAM (for good reasons) and leave a few unused
bytes as a buffer zone (some systems may even use the reserved buffer for some
useful data - not important to us in this tutorial). For the purpose of this
tutorial, we will go ahead with a buffer 64 kiB large, which in turn gives us
an address of `0x1FFF0000` (512 MiB - 64 kiB buffer).

## Meet `sp`, the Stack Pointer

On 64 bit ARM, the position of the stack is stored in a special register, `sp` -
the `s`tack `p`ointer. We would like to set it to point to `0x1FFF0000`. To
achieve this, we add `mov sp, #0x1FFF0000` at the beginnging of our inline
assembly.

```rust
// main.rs
// ...

#[unsafe(naked)]
#[unsafe(no_mangle)]
pub extern "C" fn main() {
    core::arch::naked_asm!(
        "mov sp, #0x1FFF0000",
        "1:",
        "wfe",
        "b 1b"
    );
}

// ...  
```

## Zero BSS

In general, ELF files have four very important symbol sections:

- `.text`  - contains executable machine code instructions
- `.data`  - contains (mutable) global variables that star initialized to a
             non-zero value
- `.rodata`- same as `.data` but for constants (`ro` = readonly)
- `.bss`   - same as `.data` but for uninitialized or zero-initialized data[^1]

It is customary to set all data in `.bss` to zero (even for variables that were
not explicitly initialized to zero in source code). You could think that the
linker or the compiler would handle this automatically, but that is actually not
the case. The `.bss` is special in the sense that no actual data is included in
the binary, so there is no data to be initialized to zero by either the compiler
or the linker. The reason for not storing data is entirely practical - it would
be a collosal waste of space to include a bunch of zeros in the binary.

For these reasons, it is normally up to the loader to zero the `.bss` section
at program startup. But this is only possible if the program to be executed
includes headers that tell the loader where in memory `.bss` will be located -
information that is included in the ELF headers but not included in raw binary
format, which is the format expected by the RPi firmware. So, even though we
have been feeding Qemu our kernel in the ELF format, in the near future when we
finally run our kernel on real hardware, there will be no loader that would zero
`.bss` for us before our kernel starts. This means it is up to us to zero the
`.bss` section manually as part of our startup code.

To pass the memory addreses of `.bss` start and end to our assembly code, we set
two more symbols in our linker script and use linker script keywords to obtain
the addresses of interest:

```ld
ENTRY(start)

SECTIONS {}

bss_start = ADDR(.bss);
bss_end = bss_start + SIZEOF(.bss);
``` 

We then write a loop that will implement the following pseudocode:

```text
let x0 = __bss_start

while x0 != __bss_end:
    *x0 = 0
    x0 = x0 + 8 bytes  // 8, because 8 bytes is the size of a pointer on
                       // 64-bit arch.     
```

Which translates into following assembly:

```asm
  ldr  x0, =bss_start
  ldr  x1, =bss_end
1:
  cmp  x0, x1
  b.eq 1f
  str  xzr, [x0], #8
  b    1b
1:
; code after the zero .bss loop
```

The lines of assembly above do the following:

`ldr  x0, =bss_start` - loads the value of `bss_start` into reg. `x0`
`ldr  x1, =bss_end`   - loads the value of `bss_end` into reg. `x1`
`cmp x0, x1`          - compares registers `x0` and `x1`
`b.eq 1f`             - if the result of previous comparison is "values equal",
                        branch to label 1 in the forward direction
`str xzr, [x0], #8`   - store the value of register `xzr` (a utility register,
                        always set to zero) at the memory address pointed to `x0`,
                        then, increment `x0` by 8 bytes
                      - you can think of `[x0]` as a dereference of `x0`                     
`b 1b`                - branch to label 1 in the backward direction

Which we inline in our rust setup code like this:

```rust
// main.rs
// ...

#[unsafe(naked)]
#[unsafe(no_mangle)]
pub extern "C" fn main() {
    core::arch::naked_asm!(
        // setup the stack pointer
        "mov sp, #0x1FFF0000",
        // zero the .bss section
        "  ldr  x0, =bss_start",
        "  ldr  x1, =bss_end",
        "1:",
        "  cmp  x0, x1",
        "  b.eq 1f",
        "  str  xzr, [x0], #8",
        "  b    1b",
        // parking loop
        "1:",
        "  wfe",
        "  b 1b"
    );
}

// ...  
```

Running with `cargo run` yields this output:

```text
----------------
IN: start
0x00000000:  b27033ff  mov      sp, #0x1fff0000
0x00000004:  58000120  ldr      x0, #0x28
0x00000008:  58000141  ldr      x1, #0x30
0x0000000c:  eb01001f  cmp      x0, x1
0x00000010:  54000060  b.eq     #0x1c

```

... four times

If you noticed that the `ldr` operations seem to use "weird addresses" (like
`#0x28` when setting `x0`) - don't worry. The linker actually placed the values
of symbols we defined in the linker script at those places in memory, so the
`ldr x0, #0x28` actually loads the value stored at `0x80028`, that is, the value
of `bss_start` we set in the linker script. 

## Four Times

At this point you can notice one significant issue with the code - we actually
have a race condition! Qemu starts all four cores with and sets them to execute
our kernel, and all four kernels try to zero `.bss` data stored in RAM. This is
because even though each processor has its own set of registers, they all share
the same RAM and thus the same memory space.

Right now, the race condition is fairly innocent - all the cores do is they write
zeros to the same memory address. But in the future, when our kernel becomes more
complex, we would most certainly run into a real race condition that would corrupt
the state of our kernel and lead to incorrect behavior or, if we are lucky,
a crash. This is not to mention running the kernel on all four cores is not
desired at all - we want our kernel to execute on a single core, and perhaps
eventually delegete some tasks to other cores, if we desired so.

To fix this problem, we choose a single core for the execution of our kernel.
Each core has its own id, (ranging from 0 to 3 for a 4-core system). Since we
would like to keep our kernel as portable as possible, let's choose the core 0,
since every processor is guaranteed to have at least a single core. Then, we
will adjust our setup code to check the core it's executed on, then proceed only
if the core id is 0, otherwise jump to the parking loop.

To get the ID of the core, we have to read from a special register `MPIDR_EL1`
that contains various data about "core affinity" (besides the core's ID, it
contains information about higher level groupings of the core, such as the core's
cluster in a multi-cluster system, etc.). To read the value of `MPIDR_EL1` into
some register, we have to use a special instruction `mrs`, mask out only the bits
that contain the core ID (we are not interested in the higher level groupings)
and continue only on the core with ID 0, otherwise we jump straight to the parking
loop:

```asm
mrs	x0, MPIDR_EL1 ; read core affinity data into x0
and x0, x0, 0b11  ; bitwise and: x0 = x0 | 0b11
cmp x0, 0         ; compare x0 with 0
b.ne 2f           ; if not equal, branch to the parking loop, whose label we
                  ; have to change to 2
```

Which we put at the start of our startup code in `main`:

 ```rust
// main.rs
// ...

#[unsafe(naked)]
#[unsafe(no_mangle)]
pub extern "C" fn main() {
    core::arch::naked_asm!(
        // check core ID, proceed only on core 0
        "mrs x0, MPIDR_EL1",
        "and x0, x0, 0b11",
        "cmp x0, 0",
        "b.ne 2f",
        // setup the stack pointer
        "mov sp, #0x1FFF0000",
        // zero the .bss section
        "ldr  x0, =bss_start",
        "ldr  x1, =bss_end",
        "1:",
        "cmp  x0, x1",
        "b.eq 2f",
        "str  xzr, [x0], #8",
        "b    1b",
        // parking loop
        "2:",
        "wfe",
        "b 2b"
    );
}
// ...
```

Having the parking loop at the end is now becoming a little awkward, so let's
rearrange it and place it just after the core ID check:

 ```rust
// main.rs
// ...

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
        "mov sp, #0x1FFF0000",
        // zero the .bss section
        "ldr  x0, =bss_start",
        "ldr  x1, =bss_end",
        "1:",
        "cmp  x0, x1",
        "b.eq 1f",
        "str  xzr, [x0], #8",
        "b    1b",
        "1:",
        "nop" // no operation just yet...
    );
}
// ...
```

Running this in Qemu, you will be able to see that the stack and `.bss` setup
code is ran only once - the other cores jump straight to the parking loop.

## Jumping to Rust

We now finally have everything in place to jump to our first "normal" Rust
function and leave the world of assembly.

Before we do that, we do a quick rename of `main` to `start`, so that we can use
`main` as a name for a "normal" Rust function and tuck the startup assembly code
into its own module "start":

 ```rust
// main.rs
#![no_main]
#![no_std]

use core::panic::PanicInfo;

mod start;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    unimplemented!()
}
```

```rust
// start.rs

#[unsafe(naked)]
#[unsafe(no_mangle)]
pub extern "C" fn start() {    // notice the function name changed to start
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
        "mov sp, #0x1FFF0000",
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
        "nop" // no operation just yet...
    );
}    
```

We now create a very simple `fn main` that will immediately `panic!`:

 ```rust
// main.rs
#![no_main]
#![no_std]

use core::panic::PanicInfo;

mod start;

fn main() -> ! {
    panic!();
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    unimplemented!()
}
```

...and then jump to it from our `start` function:

```rust
// start.rs

use super::main;

#[unsafe(naked)]
#[unsafe(no_mangle)]
pub extern "C" fn start() {
    core::arch::naked_asm!(
    
        // ...

        "b.eq 1f",
        "str  xzr, [x0], #8",
        "b    1b",
        "1:",
        "b {}", sym main
    );
}    
```

The `sym main` is a special argument to the special `format!` macro Rust uses
for formatting inline assembly. It means we pass whatever symbol `main` is
assigned during compilation (remember, `rustc` magles symbol names by default)
to the `b` instruction in the inline assembly.

## Back in the Safe Waters Again

Running this yields following Qemu output (the core check and the parking loop
of idle cores are left out for brevity):

```text
----------------
IN: start
0x00000058:  d53800a0  mrs      x0, mpidr_el1
0x0000005c:  92400400  and      x0, x0, #3
0x00000060:  f100001f  cmp      x0, #0
0x00000064:  54000060  b.eq     #0x70

----------------
IN: start
0x00000058:  d53800a0  mrs      x0, mpidr_el1
0x0000005c:  92400400  and      x0, x0, #3
0x00000060:  f100001f  cmp      x0, #0
0x00000064:  54000060  b.eq     #0x70

----------------
IN: start
0x00000068:  d503205f  wfe
0x0000006c:  17ffffff  b        #0x68

----------------
IN: start
0x00000058:  d53800a0  mrs      x0, mpidr_el1
0x0000005c:  92400400  and      x0, x0, #3
0x00000060:  f100001f  cmp      x0, #0
0x00000064:  54000060  b.eq     #0x70

----------------
IN: start
0x00000058:  d53800a0  mrs      x0, mpidr_el1
0x0000005c:  92400400  and      x0, x0, #3
0x00000060:  f100001f  cmp      x0, #0
0x00000064:  54000060  b.eq     #0x70

----------------
IN: start
0x00000068:  d503205f  wfe
0x0000006c:  17ffffff  b        #0x68

----------------
IN: start
0x00000070:  b27033ff  mov      sp, #0x1fff0000
0x00000074:  580000e0  ldr      x0, #0x90
0x00000078:  58000101  ldr      x1, #0x98
0x0000007c:  eb01001f  cmp      x0, x1
0x00000080:  54000060  b.eq     #0x8c

----------------
IN: start
0x0000008c:  1400000a  b        #0xb4

----------------
IN: _ZN6ferros4main17h66b48a6dfdde69deE
0x000000b4:  d503201f  nop
0x000000b8:  10fffac0  adr      x0, #0x10
0x000000bc:  94000001  bl       #0xc0

----------------
IN: _ZN6ferros4main19panic_cold_explicit17hb892d9c16d9d0380E
0x000000c0:  94000009  bl       #0xe4

----------------
IN: _ZN4core9panicking14panic_explicit17h80c39b8a630a2655E
0x000000e4:  d10143ff  sub      sp, sp, #0x50
0x000000e8:  a9047bfd  stp      x29, x30, [sp, #0x40]
0x000000ec:  910103fd  add      x29, sp, #0x40
0x000000f0:  d503201f  nop
0x000000f4:  10fffaa8  adr      x8, #0x48
0x000000f8:  d503201f  nop
0x000000fc:  10002cc9  adr      x9, #0x694
0x00000134:  aa0003e1  mov      x1, x0
0x00000138:  00000148  udf      #0x148

----------------
IN: _ZN4core3fmt9Formatter3pad17hdc1fc7a515466962E
0x00000200:  00000058  udf      #0x58

```

As an interesting aside, notice the mangled symbols for `main` and the `panic`
chain - you should be able to visually parse out the original function names
from the mangled ones.

Besides that, there are two more important things to notice here:

- The code jumps from our `start` to our `main`, then to a chain of panic
  hadnlers that eventually loop on themselves (the `unimplemented!` call in our
  panic handler is actually wrapping another `panic!` in itself).
- The execution ends with a `udf` = undefined instruction, signalling something
  went wrong and the core ended up in a situtation it doesn't know what to do
  about.

The corrupted state at the end of execution is the result of infinite calls to
`panic!` within our current panic handling implementation. We can fix it by
changing our panic handler to instead enter an infinite `wfe` loop, now without
direct use of inline assembly:

```rust
// main.rs
#![no_main]
#![no_std]

use aarch64_cpu::asm;
use core::panic::PanicInfo;

mod start;

fn main() -> ! {
    panic!();
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        asm::wfe();
    }
}
```

Here, we have used an assembly wrapper from the `aarch64_cpu`, a crate providin
low level access to AArch 64 processor functionality.

Now, running the kernel will result with the same chain of functions as before,
with the single difference in the eventual execution result - we no longer end
up with an `udf` but instead park the core in an `wfe` loop.

Finally, we make one last cosmetic change - to keep our `main` modules neat and
clean, we move the panic handler into its own submodule:

```rust
// main.rs
#![no_main]
#![no_std]

mod panic_handler;
mod start;

fn main() -> ! {
    panic!();
}
```

```rust
// panic_handler.rs
use aarch64_cpu::asm;
use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        asm::wfe();
    }
}
```

## On to debugging!

Congratulations on getting through to the end of chapter 2! Our kernel is
properly set up for executing compiled Rust code, and from now on we will mostly
move away from assembly and finally start writing some Rust! You can check out
and cross-reference the source code we built together in the branch
[chapter-2](https://github.com/matej-almasi/rust-sbc-os-book/tree/chapter-2).
 
In the next chapter, we will resurrect `println!()` which will alow us some
primitive form of debugging, and talk about debugging our kernel in general.
See you!

[^1]: `.bss` stands for "block starting symbol" - a rather historical name...
