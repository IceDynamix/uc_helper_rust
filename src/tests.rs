use super::*;

#[test]
fn load_settings() {
    let settings = settings::Settings::from_profile("debug");
    assert!(settings.is_ok());
}

/*
    TODO Async tests
    - tetrio
        - request
        - rank parse
    - tenchi
        - refresh
        - get_ranks
*/
