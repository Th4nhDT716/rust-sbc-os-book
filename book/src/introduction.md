# Writing a Rust OS for a Single Board Computer

Hello and welcome to this tutorial on writing a simple operating system kernel
for single board computers (SBCs) in Rust!

This tutorial is aimed at everyone who wants to learn about basic principles
of operating systems and build one hands-on. It will guide you, step by step,
with detailed explanations of what is being implemented and why.

## Who is this tutorial for?

This tutorial doesn't assume any prior OS development knowledge. That being said,
it is expected that you have a basic understanding of what operating systems are
and what is their purpose.

In addition to that, you should have a basic understanding of the Rust programming
language. If you are new to Rust, check out [The Rust Book](https://doc.rust-lang.org/book/)
before continuing with this tutorial.

As operating systems are developed on bare metal, it will be beneficial (but not
required) to first read [The Rust Embedded Book](https://doc.rust-lang.org/embedded-book/) -
it is a fairly quick read and as a benefit, it will guide you through the
setup of basic tools like [GDB](https://sourceware.org/gdb/) or [QEMU](https://www.qemu.org/).

In the first two chapters of the tutorial, we will write a few (18) lines of
AArch64 assembly. No prior knowledge of assembly language is required and every
line will be explained, both in terms what is happening and why.

## How to follow this tutorial?

The tutorial consists of 20 chapters in which we will build a simple OS kernel,
step by step. In general, you should read along this book, which will explain
every line of code we will write, and will often include explanations about the
design decisions we are making, as well as (hopefully) helpful tips and
explanations.

In addition to the text of the book, you can find the source code of the kernel
being built in its [own directory]. The code **will be versioned in git**, each
chapter having its own corresponding branch.

You can go ahead and clone the repository:

```sh
git clone https://github.com/matej-almasi/rust-sbc-os-book.git
```

After finishing a chapter of the book you can switch to the correspondig branch,
compare your code with the code in the branch, then continue reading the next
chapter.

[own directory]: https://github.com/matej-almasi/rust-sbc-os-book/tree/main/ferros

## Attribution

This book is based on [Operating System development tutorials in Rust on the Raspberry Pi](https://github.com/rust-embedded/rust-raspberrypi-OS-tutorials)
by Andre Richter. It is truly a giant on whose shoulders this book stands.

## Help

In case you need any help, find a bug, or encounter any other issue, feel free
to open an issue or contact me directly via email.

## Have fun!
