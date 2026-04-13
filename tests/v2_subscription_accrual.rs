//! Priority 5: Subscription accrual characterization.
//!
//! Verifies how the `/dashboard/subscriptions` endpoint classifies active vs.
//! cancelled subscriptions and whether they are windowed to the current period.

mod common;

use common::auth::create_user_and_login;
use common::entities::{create_category, create_subscription};
use common::{V2_BASE, test_client};
use rocket::http::Status;
use rocket::local::asynchronous::Client;
use serde_json::Value;

async fn get_json(client: &Client, url: &str) -> Value {
    let resp = client.get(url.to_string()).dispatch().await;
    assert_eq!(resp.status(), Status::Ok, "GET {url} failed with {}", resp.status());
    serde_json::from_str(&resp.into_string().await.unwrap()).unwrap()
}

async fn dashboard_subs(client: &Client, period_id: &str) -> Value {
    get_json(client, &format!("{V2_BASE}/dashboard/subscriptions?periodId={period_id}")).await
}

// ─────────────────────────────────────────────────────────────────────────────

#[rocket::async_test]
#[ignore = "requires database"]
async fn active_subscription_in_period_appears_in_dashboard() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let cat = create_category(&client, "Streaming Dash", "expense").await;
    let sub_id = create_subscription(&client, "Netflix Dash", &cat, 1_499, "monthly", "2026-03-15").await;

    let period_id = common::entities::create_period(&client, "2026-03-01", "2026-03-31").await;
    let body = dashboard_subs(&client, &period_id).await;

    let subs = body["subscriptions"].as_array().unwrap();
    let ours = subs.iter().find(|s| s["id"] == sub_id.as_str());
    assert!(ours.is_some(), "active sub with in-period charge date must appear");
    let ours = ours.unwrap();
    assert_eq!(ours["billingAmount"], 1_499);
    assert_eq!(ours["nextChargeDate"], "2026-03-15");
    // displayStatus is some valid variant
    let status = ours["displayStatus"].as_str().unwrap();
    assert!(status == "charged" || status == "today" || status == "upcoming", "unexpected status {status}");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn cancelled_subscription_does_not_appear_in_dashboard() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let cat = create_category(&client, "Streaming Cancel", "expense").await;
    let sub_id = create_subscription(&client, "Cancelled Netflix", &cat, 999, "monthly", "2026-03-15").await;

    // Cancel it
    let resp = client.post(format!("{V2_BASE}/subscriptions/{sub_id}/cancel")).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);

    let period_id = common::entities::create_period(&client, "2026-03-01", "2026-03-31").await;
    let body = dashboard_subs(&client, &period_id).await;
    let subs = body["subscriptions"].as_array().unwrap();
    assert!(subs.iter().all(|s| s["id"] != sub_id.as_str()), "cancelled sub must not appear in dashboard");
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn subscription_outside_current_period_does_not_appear() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let cat = create_category(&client, "Streaming Outside", "expense").await;
    let sub_id = create_subscription(&client, "FutureSub", &cat, 500, "monthly", "2026-07-15").await;

    // Period in March — charge date is July, outside the window.
    let period_id = common::entities::create_period(&client, "2026-03-01", "2026-03-31").await;
    let body = dashboard_subs(&client, &period_id).await;
    let subs = body["subscriptions"].as_array().unwrap();
    assert!(
        subs.iter().all(|s| s["id"] != sub_id.as_str()),
        "sub with out-of-period charge date must not appear"
    );
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn multiple_in_period_subscriptions_all_appear() {
    let client = test_client().await;
    create_user_and_login(&client).await;

    let cat = create_category(&client, "Streaming Multi", "expense").await;
    let sub_a = create_subscription(&client, "MultiA", &cat, 100, "monthly", "2026-03-05").await;
    let sub_b = create_subscription(&client, "MultiB", &cat, 200, "monthly", "2026-03-10").await;
    let sub_c = create_subscription(&client, "MultiC", &cat, 300, "monthly", "2026-03-20").await;

    let period_id = common::entities::create_period(&client, "2026-03-01", "2026-03-31").await;
    let body = dashboard_subs(&client, &period_id).await;
    let subs = body["subscriptions"].as_array().unwrap();

    let ids: Vec<&str> = subs.iter().map(|s| s["id"].as_str().unwrap()).collect();
    for expected in [&sub_a, &sub_b, &sub_c] {
        assert!(ids.contains(&expected.as_str()), "{expected} should appear");
    }
    // active_count should be ≥ 3 (characterization: may count all active subs
    // regardless of whether they're windowed to the period).
    assert!(body["activeCount"].as_i64().unwrap() >= 3);
    // monthly_total should be ≥ the sum of the three (100 + 200 + 300 = 600)
    assert!(body["monthlyTotal"].as_i64().unwrap() >= 600);
}

#[rocket::async_test]
#[ignore = "requires database"]
async fn dashboard_subscriptions_requires_period_id() {
    let client = test_client().await;
    create_user_and_login(&client).await;
    let resp = client.get(format!("{V2_BASE}/dashboard/subscriptions")).dispatch().await;
    assert_eq!(resp.status(), Status::BadRequest);
}
