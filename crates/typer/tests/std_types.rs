use clg_parser::parse;
use clg_typer::{check_with_vcs_with_std, StdTypeInfo, StdTypeMap};

#[test]
fn std_chain_type_in_struct_and_enum_fields_is_allowed() {
    let src = r#"
        struct Wallet { owner: std::eth::Address; }
        enum Holder { One(std::eth::Address) }
        function main() -> Int { 0 }
    "#;
    let ast = parse(src).expect("parse ok");
    let mut std_types = StdTypeMap::new();
    std_types.insert(
        "std::eth::Address".to_string(),
        StdTypeInfo {
            byte_len: 20,
            align: 1,
        },
    );
    check_with_vcs_with_std(&ast, &std_types).expect("type-check ok");
}
