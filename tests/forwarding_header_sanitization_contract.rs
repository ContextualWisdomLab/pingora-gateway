use pingora::prelude::RequestHeader;

#[test]
fn pingora_remove_header_removes_all_case_insensitive_duplicate_values() {
    let mut request =
        RequestHeader::build("GET", b"/", None).expect("fixture request must be valid");

    request
        .append_header(
            "X-Forwarded-Client-Cert",
            "By=spiffe://attacker-a;URI=spiffe://attacker-a/client",
        )
        .expect("first fixture XFCC value must be valid");
    request
        .append_header(
            "x-forwarded-client-cert",
            "By=spiffe://attacker-b;URI=spiffe://attacker-b/client",
        )
        .expect("second fixture XFCC value must be valid");

    assert_eq!(
        request
            .headers
            .get_all("x-forwarded-client-cert")
            .iter()
            .count(),
        2,
        "fixture must contain duplicate case-insensitive XFCC values before sanitization",
    );

    request.remove_header("X-Forwarded-Client-Cert");

    assert_eq!(
        request
            .headers
            .get_all("x-forwarded-client-cert")
            .iter()
            .count(),
        0,
        "pinned Pingora remove_header must remove every case-insensitive duplicate value",
    );
}
