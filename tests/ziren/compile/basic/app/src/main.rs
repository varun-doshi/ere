#![no_main]
zkm_zkvm::entrypoint!(main);

pub fn main() {
    let n = zkm_zkvm::io::read::<u32>();

    let a: u32 = 0;
    let b: u32 = 1;

    zkm_zkvm::io::commit(&(n * 2));
}
