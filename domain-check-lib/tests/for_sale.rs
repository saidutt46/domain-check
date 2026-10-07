//! Live RFC 10023 tests against SIDN's published fixtures.
#![cfg(feature = "forsale")]

use domain_check_lib::{CheckConfig, DomainChecker};

#[tokio::test]
async fn example_nl_is_for_sale() {
    let checker = DomainChecker::with_config(CheckConfig::default().with_for_sale(true));
    assert!(
        checker.for_sale_enabled(),
        "system resolver should be available in CI"
    );
    let result = checker.check_domain("example.nl").await.unwrap();
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
    let result = DomainChecker::new()
        .check_domain("example.nl")
        .await
        .unwrap();
    assert!(result.for_sale.is_none());
}

#[tokio::test]
async fn batch_and_stream_paths_annotate() {
    use futures_util::StreamExt;
    let checker = DomainChecker::with_config(CheckConfig::default().with_for_sale(true));
    let domains = vec!["example.nl".to_string()];
    let batch = checker.check_domains(&domains).await.unwrap();
    assert!(batch[0].for_sale.is_some());
    let streamed: Vec<_> = checker.check_domains_stream(&domains).collect().await;
    assert!(streamed[0].as_ref().unwrap().for_sale.is_some());
}
