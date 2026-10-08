//! Live RFC 10023 tests against SIDN's published fixtures.
#![cfg(feature = "forsale")]

use domain_check_lib::{CheckConfig, DomainChecker};

/// SIDN rate-limits bursts of RDAP queries. The for-sale lookup only runs on
/// TAKEN results, so skip (rather than flake) when example.nl isn't confirmed.
/// example.nl is registered, so anything other than "taken" means the RDAP
/// answer was lost (a 429, or a WHOIS fallback that misread the response).
fn unconfirmed(available: Option<bool>) -> bool {
    if available != Some(false) {
        eprintln!("skipped: .nl registry did not confirm example.nl as taken (rate limited?)");
        return true;
    }
    false
}

#[tokio::test]
async fn example_nl_is_for_sale() {
    let checker = DomainChecker::with_config(CheckConfig::default().with_for_sale(true));
    assert!(
        checker.for_sale_enabled(),
        "system resolver should be available in CI"
    );
    let available = match checker.check_domain("example.nl").await {
        Ok(result) => (result.available, Some(result)),
        Err(_) => (None, None),
    };
    if unconfirmed(available.0) {
        return;
    }
    let result = available.1.unwrap();
    assert_eq!(result.available, Some(false));
    let fs = result
        .for_sale
        .expect("example.nl publishes _for-sale records");
    assert!(fs.prices.iter().any(|p| p.to_string() == "EUR 100000000"));
    assert!(fs
        .uris
        .iter()
        .any(|u| u.value == "https://example.nl/for-sale.txt" && u.trusted));
    assert!(!fs.codes.is_empty());
    assert!(fs
        .texts
        .iter()
        .all(|t| !t.contains("not part of the ForSale record")));
}

#[tokio::test]
async fn taken_domain_without_record_is_not_for_sale() {
    let checker = DomainChecker::with_config(CheckConfig::default().with_for_sale(true));
    let result = checker.check_domain("google.com").await.unwrap();
    assert_eq!(result.available, Some(false));
    assert!(result.for_sale.is_none());
}

#[tokio::test]
async fn disabled_by_default() {
    // Holds whether or not the registry answers: no flag, no lookup.
    if let Ok(result) = DomainChecker::new().check_domain("example.nl").await {
        assert!(result.for_sale.is_none());
    }
}

#[tokio::test]
async fn batch_and_stream_paths_annotate() {
    use futures_util::StreamExt;
    let checker = DomainChecker::with_config(CheckConfig::default().with_for_sale(true));
    let domains = vec!["example.nl".to_string()];
    // A 429 surfaces as Err; treat it like any other unconfirmed answer.
    if let Ok(batch) = checker.check_domains(&domains).await {
        if !unconfirmed(batch[0].available) {
            assert!(batch[0].for_sale.is_some());
        }
    }
    let streamed: Vec<_> = checker.check_domains_stream(&domains).collect().await;
    if let Ok(streamed) = &streamed[0] {
        if !unconfirmed(streamed.available) {
            assert!(streamed.for_sale.is_some());
        }
    }
}
