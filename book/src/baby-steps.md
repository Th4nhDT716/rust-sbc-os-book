# Chapter 1 - Baby Steps

Every adventure starts with a step. Let's start our adventure!

## Setting Up the Project

Every *Rust* adventure starts with `cargo new`. Go ahead and open your favourite
directory (mine is `Projects`) and open the terminal. Pick a name for your
kernel (I decided to go with `ferros`) and run `cargo new <kernel_name>`.

## No Main, No Standards

Since we are creating an Operating System, and since an Operating System,
unlike a typical application doesn't have an Operating System to run on top of
('duh...), we have to give up the comfort of `std` and even the comfort of the
`main` function.

This is is, among other reasons, mainly because `std` relies
on an operating system for much of its functionality, and/or happily uses "high
level" concepts like the heap for dynamic data allocation, which often are not
desirable or available in bare metal environments. `main`, on the other hand,
is not avaialble because Rust's `main` secretly does runtime setup for your
program (like making available arguments passed to your application when it
is invoked).

To tell `rustc` we won't be using `std` or `main`, we put two declarative
macros to our `main.rs` (which can happily stay named `main.rs`):

```rust
#![no_main]
#![no_std]
 
// ... rest of main.rs
```

This has the unfortunate effect of losing `println!` (which is part of `std`),
which immediately causes our code to not compile, since `cargo new` scaffolded
our project with a single `println` in our `main`. So, with some sorrow,
we delete the call to `println!`:

```rust
#![no_main]
#![no_std]

fn main() {
}
```

## More Trouble Without `std`

In addition to that, we suffered another loss - the default panic handler
(the function invoked to print the panic message) is no longer available too.
Fortunately, it is not too difficult to write one ourselves:

```rust
// main.rs
use core::panic::PanicInfo;

// ...

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    unimplemented!()
}  
```

`PanicInfo` is a data structure contained in the `core` module (the part of
`std` that is always available, on all targets, including bare metal) that
contains some useful info we might want to use later, when our panic handler
becomes more mature.

## Even More Trouble

Our troubles that started when we dropped `std` have not yet ended. If you
use `rust-analyzer` (as you should, btw), you will notice that it reports to
us an error "can't find crate for `test`" - this is because the scaffolding
needed for `cargo test` to build and execute any test we might have had written
is also dependent on... `std`. This means we need to tell there will be no
tests (at least for now) for our package. We announce this to `cargo` (and
`rust-analyzer` by adding the following entries to `Cargo.toml`:

```toml
[[bin]]
name = "ferros"
test = false
bench = false
```

The `bench` entry is required as benchmark tests rely on the `test` scaffolding
and thus also need to be explicitly turned off.

At this point, running `cargo check` gives us a feasible build and
`rust-analyzer` reports no more errors.

## Setting the Target

At this point, we should decide what kind of Single Board Computer are we
actually building our kernel for. Since we are not very decisive and since
we would like to defer such a decision until later (say, until a SBC arrives
at our doorstep...) and since [QEMU](https://www.qemu.org/) is a small miracle,
we decide to only make a much smaller decision - what kind of CPU architecture
do we want to support?

There is no right or wrong answer, but after some deliberation[^1] we decide to
go on with 64-bit ARM (AArch64). ARM is a particularly good choice because:

- ARM Assembly is (relatively) simple[^2]
- ARM boards are readily available in good quality
- ARM boards have good emulation support in QEMU

All that being said, each chapter will have an appendix containing
modifications required to run our code on a RISC-V machine.

With that decision out of the way, we promise ourselves to write code as
generic as possible (so we will have the ability to choose a SBC later) and
pick *some* SBC board that we will emulate in QEMU. Since most of you will
probably grab a Raspberry Pi and since Raspberry Pi is a very decent choice
anyway[^1] we will use it as the target of QEMU emulation.

## Back to Code

Having decided the target architecture, we shall now focus on telling `rustc`
to actually compile our kernel *for* that architecture. We consult
the [Cargo Book](https://doc.rust-lang.org/cargo/) and learn that we should
create a config file `.cargo/config.toml`:

```sh
mkdir .cargo
touch .cargo/config.toml
```

and configure cargo with the desired target:

```toml
[build]
target = [?????]
```

But what should our `target` be? Obviously, we want to target bare metal
AArch64, but that gives us two options: `aarch64-unknown-none` and
`aarch64-unknown-none-softfloat` - but which is the one we need?

## Floating

The difference between the two target variants comes down to whether or not our
kernel assumes the availability of hardware floating point unit (FPU). For the
purposes of developing a kernel, we will want to stay away from FP alltogether
and thus not make any assumption as to whether an FPU will or will not be
available. Therefore, we will pick the `-softfloat` option, which simply
means that any FP operations would be done by software emulation instead
of an FPU use.

Thus, our `.cargo/config.toml` will look like this:

```toml
[build]
target = ["aarch64-unknown-none-softfloat"]
```

## But Where To Start

Even though we fixed all of the compiler errors that haunted us so far, running
`cargo check` gives us a somewhat disconcerting warning:

```
warning: function `main` is never used
 --> src/main.rs:6:4
  |
6 | fn main() {}
  |    ^^^^
  |
```

As you can remember, we actually told `rustc` that there will be `#[no_main]`,
so our `fn main` actually *is* unused - it is not invoked by anything in our
program, and `cargo` doesn't automatically make it an entry point of our program.

On bare metal, it is up to *us* to manually configure the binary being built
with an entry point.

## Linkin Time

As a quick refresher, once `rustc` (and then LLVM, under the hood) does its job
compiling our source code into actual instructions for the processor, it ends
up with a bunch of "object" (`.o`) files that it needs to wire up together to
form the resulting binary executable (or binary library, if we were building
a `lib` crate).

For this, it calls a special program - the linker, which stitches all the objects
together, makes sures all symbols are defined (what that means will be described
in a short while) and sets a bunch of crucial metadata for programs that will
eventually use the executable (this will be especially important in the next
chapter).

To give a brief overview of what linker does; every function (and every global
constant...) is labeled by a *symbol*. A function definition (`fn foo() {/*...*/}`)
defines the symbol, and calls to other functions are actually calls to the
symbols that represent them (so, `let x = foo(y, z)` internally is a call to
label `foo`, which is in this case expected to be a function).

Usually, the configuration `rustc` passes to the linker and the linker's default
settings are more than enough to create a viable binary without any input from
us, the developers. That being said, the linker offers us a way to configure its
behavior, in case we need such control.

The way to configure the linker for linking of a specific binary is through
a [linker script](https://wiki.osdev.org/Linker_Scripts) - a simple file that,
among other things, allows as to tell the linker where in the resulting binary
to place different parts of the program and where is the *ENTRY* of its execution.

Let's write one ourselves, and let's try to tell the linker that `main` is the
symbol that denotes the entry of our program.

```sh
touch kernel.ld
```

The name of the linker script doesn't matter much, but it is customary to give
it a `.ld` extension and name it sensibly, thus the name `kernel.ld`.

We place the following line inside the script:

```ld
/* kernel.ld */
ENTRY(main)
```

`ENTRY` is a keyword that does what it sounds like - tells the linker that
symbol `main` is the *ENTRY* of our program.

## External Help

Now, we didn't quite get rid of the compiler warning, because 1. `cargo` doesn't
really know that there is some linker script (and so doesn't `rustc`) and 2.
even if `cargo` knew we have this linker script in place, `cargo` can't really
read it and understand that `main` defined in `main.rs` is now "used" by
the linker.

We will fix problem 2. first. We can't quite teach `cargo` to understand the
linker script, but we can tell `cargo` that *something* outside our crate will
use `fn main`, ridding us of the warning we encountered above. We achieve this
by adding `pub extern "C"` before declaring `fn main`:

```rust
// main.rs
// ...

pub extern "C" fn main() {}

// ...
```

`extern` here means `main` should be a symbol available to `extern`al users -
in this case that means us when we write our linker script. The `"C"` part tells
the compiler that `main` shall adher to C language calling conventions. It is
not very important for us right now, and we could have used a different calling
convention if we desired so (we could happily use `"Rust"`, for example).

## No Mangle

There is just one more thing we need to take care of - name mangling. By default,
`rustc` "mangles" (adds lots of not very readable characters to) the names of
our functions, which is true for `main` as well. To disable this for our `main`
function (so that the linker will be able to find symbol `main` when it looks
for the `ENTRY(main)` we defined above), we need to put `#[unsafe(no_mangle)]`
attribute to our main:

```rust
// main.rs
// ...

#[unsafe(no_mangle)]
pub extern "C" fn main() {}  

// ...
``` 


## Building

We turn our attention to problem 1. mentioned above – how do we tell `cargo` to
use our linker script? One good way we can achieve this is to create a [build
script](https://doc.rust-lang.org/cargo/reference/build-scripts.html) (for some
time, the last script we are making, I promise) named `build.rs`.

In the root of our project:

```sh
touch build.rs
```

`cargo` automatically picks up a `build.rs` file, provided it exists in the place
as `Cargo.toml` and executes before building a crate using `cargo build`. There
are many great uses for the build script, but for now, we will suffice with writing
the following lines in the script:

```rust
// build.rs

fn main() {
    println!("cargo:rustc-link-search={}", env!("CARGO_MANIFEST_DIR"));
    println!("cargo:rustc-link-arg=--script=kernel.ld");
}
```

The two lines inside the scripts `main` are read by `cargo`, which in turn is
told to pass `link-search` and `link-arg` as parameters to `rustc` when it is
invoked to compile our kernel. `link-search={CARGO_MANIFEST_DIR}` tells `rustc`
to tell the linker to look for a linker script in the directory where
`Cargo.toml` lives (as we created it there) and `link-arg=--script=kernel.ld`
tells `rustc` to tell the linker that it should use `kernel.ld` as its linker
script.

There is one small issue with `build.rs` as it stands however. When we build
a Rust crate and call `cargo build` without any changes to `Cargo.toml` or
our actual source code, `cargo` is smart enough to skip the entire build process,
knowing there is nothing that could affect the resulting binary, which has
previously been built.

We would like to tell `cargo` to treat changes to `build.rs` and `kernel.ld`
as changes that affect the resulting binary (i.e. to treat them as it treats
`Cargo.toml` or `*.rs` files in `src`). This is possible by adding the following
lines to `build.rs`:


```rust
// build.rs

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=kernel.ld");

    // ...
}
```

If you have run `cargo build` before adding those two lines, make sure to run
`cargo clean` before your next call to `cargo`, as `cargo` wouldn't know that
it should rerun `build.rs` when `build.rs` changes until `build.rs` with the
lines above runs for the first time.

## Waiting for Events

Now that we have most of the build infrastructure ready, we can proceed and
actually implement some code! For starters, we should implements something
really small, just to make sure that our code actually is executed when we will
eventually run our kernel. For this, we will implement a parking loop - the
processor will wait for events (what events are doesn't matter right now) and
when an event occurs, it will loop back to waiting again.

To get our kernel up and running, we will have to pull up our sleeves and write
a few lines of 64 bit ARM assembly. Fortunately, we can write inline assembly in
`.rs` files and ARM assembly is not too complicated (at least, not for simple
purposes like ours).

There are a few ways we can write inline assembly in Rust. Right now, we want to
make use of Rust [*naked functions*](https://blog.rust-lang.org/2025/06/26/Rust-1.88.0/#naked-functions) -
functions that consist only of inline assembly and for which `rustc` doesn't
automatically generate function prologues and epilogues (small bits of assembly
boilerplate at the beginning and end of a function that do some setup and
teardown for the function) as we are actually going to implement this setup and
teardown ourselves (in the next chapter - in fact, this setup will be the sole
objective of the next chapter).

We create a naked function by adding a `#[unsafe(naked)]` attribute before the
function declaration (`unsafe` is there precisely because it is up to us to
do the setup and teardown properly - any mistake could corrupt program state
and crash it) and including a single `core::arch::naked_asm!()` call in the `fn`:

```rust
// main.rs
// ...

#[unsafe(naked)]
pub extern "C" fn main() {
    core::arch::naked_asm!("");
}

// ...
```

To implement the parking loop itself, we write the following lines of assembly:

```rust
// main.rs
// ...

#[unsafe(naked)]
pub extern "C" fn main() {
    core::arch::naked_asm!(
        "1:",
        "   wfe",
        "   b 1b"
    );
}

// ...
```

The lines of assembly above do the following:

`1:`   - declares a label (symbol) that we can reference from other assembly code
         by its number (in this case, the number is `1`)
`wfe`  - is an instruction to `w`ait `f`or `e`vents, as mentioned above
`b 1b` - is a `b`ranc instruction - an instruction to jump `b`ack to instruction
         labeled with `1`, (if we wanted to jump to a hypothetical instruction
         labeled by `1` in the `f`oraward direction, we would use `b 1f`)

As you can see, this is indeed an (infinite) wait-for-event loop.

## Running the Kernel

With our parking loop in place, it is finally time to run our code. Since we
don't have too much functionality yet, we will make do with emulating the
Raspberry Pi with QEMU. If you haven't done so yet, now is the time to install
`qemu-system-aarch64` which is capable of emulating the whole RPi device.
 
We first build our kernel using `cargo build`. Then, we invoke qemu like this
(**don't forget to change the name of your project!**):

```sh
qemu-system-aarch64 -machine raspi4b \
                    -d in_asm        \
                    -display none    \
                    -kernel target/aarch64-unknown-none-softfloat/debug/ferros
```

This tells qemu to emulate Raspberry Pi 4B, print out executed ARM assembly,
don't use any display output (as our kernel doesn't support any display output...)
and use the file `target/.../debug/ferros` as the kernel for the emulated
machine.

You should see output similar to this, 4 times:

```text
----------------
IN: main
0x00210120:  d503205f  wfe
0x00210124:  17ffffff  b        #0x210120
```

what you see are the four cores, parked, waiting for events.

> There is an important note to be made here - the binary produced by `cargo` is
> in [ELF](https://wiki.osdev.org/ELF) file format, which is noramlly used for
> application executed on running on UNIX-like systems. This file format wouldn't
> normally be executable as a kernel on a real Raspberry Pi - for that we will
> need to turn it into "pure binary" - strip all the headers and sections with
> debuginfo, etc. For now, we will happily continue using ELF, until our first
> attempt to flash and run our kernel on real hardware later in the book.

Since we are going to use `qemu` like this a lot in this book and the command
above is a little annoying to type every time, and since we really like
`cargo run` we know and love from application development, we are going to set
up a custom runner that will actually invoke qemu in the correct way. In our
`.cargo/config.toml`:

```toml
# .cargo/config.toml
# ...

[target.aarch64-unknown-none-softfloat]
runner = """\
  qemu-system-aarch64 -machine raspi4b \
                      -d in_asm \
                      -display none \
                      -kernel
"""
    
```

Now, `cargo run` will actually invoke `qemu` with our kernel, rebuilding it when
necessary.

## Congrats!

Congrats reading all the way here. I hope you had fun and learned new things!
You can check out and cross-reference the source code we built together in
the branch [chapter-1](https://github.com/matej-almasi/rust-sbc-os-book/tree/chapter-1).
 
Now, let's move on and continue on our kernel journey...


[^1]: Ok, I admit. This book *is* an adaptation of a tutorial for Raspberry Pi,
which is an ARM system. But for now, let's pretend we *were* making a decision.

[^2]: Don't worry, there won't be much assembly written and all 18 lines of it
will be properly explained. We need to resort to asm because we have to do some
prep work before the first line of Rust code can be executed in our kernel. 
