#![no_std]
#![no_main]

use sqisign_verify::{Error, Level, Level1, Level3, Level5, PublicKey, Signature};

ckb_std::entry!(main);
ckb_std::default_alloc!();

fn verify<L: Level>(msg: &[u8], sig: &[u8], pk: &[u8]) -> Result<(), Error> {
    let pk = PublicKey::<L>::from_bytes(pk)?;
    pk.verify(msg, &Signature::from_bytes(sig)?)
}

fn verify_args() -> Result<(), Error> {
    let [msg, sig, pk] = ckb_std::env::argv() else {
        return Err(Error::InvalidLength);
    };
    let decode =
        |arg: &ckb_std::env::Arg| hex::decode(arg.to_bytes()).map_err(|_| Error::MalformedInput);
    let msg = decode(msg)?;
    let sig = decode(sig)?;
    let pk = decode(pk)?;
    match pk.len() {
        n if n == Level1::PARAMS.publickey_bytes => verify::<Level1>(&msg, &sig, &pk),
        n if n == Level3::PARAMS.publickey_bytes => verify::<Level3>(&msg, &sig, &pk),
        n if n == Level5::PARAMS.publickey_bytes => verify::<Level5>(&msg, &sig, &pk),
        _ => Err(Error::InvalidLength),
    }
}

fn main() -> i8 {
    match verify_args() {
        Ok(()) => 0,
        Err(Error::InvalidSignature) => 1,
        Err(_) => 2,
    }
}
