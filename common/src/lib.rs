extern crate serde;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct AddressResponse {
    pub ip: String,
}
