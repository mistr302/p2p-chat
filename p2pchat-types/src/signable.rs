use libp2p::identity::{Keypair, ed25519::PublicKey};
use serde::{Deserialize, Serialize};
use serde_json::to_vec;

pub fn sign<T>(value: T, keypair: &Keypair) -> Signed<T>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    let serialized = to_vec(&value).expect("Failed to serialize content");
    let sig = keypair.sign(&serialized).expect("Failed to sign");
    Signed {
        sig,
        pub_key: keypair.to_protobuf_encoding().expect("to work"),
        content: value,
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Signed<T> {
    sig: Vec<u8>,
    pub_key: Vec<u8>,
    content: T,
}
impl<T> Signed<T>
where
    T: Serialize,
{
    pub fn verify(self) -> Option<(T, PublicKey)> {
        let pk = PublicKey::try_from_bytes(&self.pub_key).unwrap();
        let serialized = to_vec(&self.content).expect("Failed to serialize content");
        match pk.verify(&serialized, &self.sig) {
            false => None,
            true => Some((self.content, pk)),
        }
    }
}
