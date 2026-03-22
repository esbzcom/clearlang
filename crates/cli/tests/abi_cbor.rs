use serde_cbor::Value;
use std::collections::BTreeMap;

fn hex_bytes(s: &str) -> Vec<u8> {
    let cleaned = s.replace([' ', '\n'], "");
    hex::decode(cleaned).expect("hex")
}

#[test]
fn request_envelope_is_canonical_cbor() {
    let mut map = BTreeMap::new();
    map.insert(
        Value::Text("kind".to_string()),
        Value::Text("query".to_string()),
    );
    map.insert(Value::Text("abi_version".to_string()), Value::Integer(1));
    map.insert(Value::Text("msg".to_string()), Value::Bytes(vec![0x01]));
    map.insert(Value::Text("state".to_string()), Value::Bytes(Vec::new()));

    let value = Value::Map(map);
    let encoded = serde_cbor::to_vec(&value).expect("encode cbor");
    let expected = hex_bytes(
        "a4
         63 6d 73 67 41 01
         64 6b 69 6e 64 65 71 75 65 72 79
         65 73 74 61 74 65 40
         6b 61 62 69 5f 76 65 72 73 69 6f 6e 01",
    );
    assert_eq!(encoded, expected);
}

#[test]
fn error_response_envelope_is_canonical_cbor() {
    let mut error = BTreeMap::new();
    error.insert(
        Value::Text("message".to_string()),
        Value::Text("fail".to_string()),
    );
    error.insert(
        Value::Text("code".to_string()),
        Value::Text("E0".to_string()),
    );
    error.insert(Value::Text("data".to_string()), Value::Bytes(Vec::new()));

    let mut map = BTreeMap::new();
    map.insert(
        Value::Text("status".to_string()),
        Value::Text("err".to_string()),
    );
    map.insert(Value::Text("abi_version".to_string()), Value::Integer(1));
    map.insert(Value::Text("events".to_string()), Value::Array(Vec::new()));
    map.insert(Value::Text("state".to_string()), Value::Bytes(Vec::new()));
    map.insert(Value::Text("data".to_string()), Value::Bytes(Vec::new()));
    map.insert(Value::Text("error".to_string()), Value::Map(error));

    let value = Value::Map(map);
    let encoded = serde_cbor::to_vec(&value).expect("encode cbor");
    let expected = hex_bytes(
        "a6
         64 64 61 74 61 40
         65 65 72 72 6f 72
         a3
         64 63 6f 64 65 62 45 30
         64 64 61 74 61 40
         67 6d 65 73 73 61 67 65 64 66 61 69 6c
         65 73 74 61 74 65 40
         66 65 76 65 6e 74 73 80
         66 73 74 61 74 75 73 63 65 72 72
         6b 61 62 69 5f 76 65 72 73 69 6f 6e 01",
    );
    assert_eq!(encoded, expected);
}
