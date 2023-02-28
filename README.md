# visgra_project
A visualization of sound as a texture in a pseudo-3D environment

## Installation

Assuming you have git and cargo installed:

1. clone repo and change into the directory
2. `$ cargo build --release`

## Running it

`$ cargo run --release` OR `$ ./target/release/visgra_project(.exe)`

## Requirements

- Fairly strong CPU, as most of the rendering is software based and there are a lot of threads that need to communicate with each other.
- Default Sound input and output configured in OS (Windows, Linux and macOS should work but are not tested)
- a .wav file with 32-bit float samples in the ´./music´ directory. "The vampire.wav" provided copyright me (i think, not sure how covers work)
