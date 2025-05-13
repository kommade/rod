use super::*;
use rod_derive::RodValidate;

struct Error {
    field: String,
}

#[derive(RodValidate)]
struct Test {
    #[rod(String {
        length: 5,
    })]
    field: String,
}

#[derive(RodValidate)]
enum TestEnum {
    
    Field(
        #[rod(String {
            length: 5,
        })]
        String
    ),
    AnotherField(Test),
}

#[test]
fn test_string_length() {
    let test = Test {
        field: "12345".to_string(),
    };
    assert!(test.validate().is_ok());

    let test = Test {
        field: "1234".to_string(),
    };
    assert!(test.validate().is_err());
}