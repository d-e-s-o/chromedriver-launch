// Copyright (C) 2025 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

//! End-to-end tests for `chromedriver-launch`.

use chromedriver_launch::Chromedriver;

use fantoccini::wd::Capabilities;
use fantoccini::ClientBuilder;

use serde_json::json;

use tempfile::tempdir;


/// Check that we can launch a Chromedriver instance and wait for it
/// to service webdriver requests.
#[tokio::test]
async fn chromedriver_launching() {
  let chromedriver_args = [
    "--disable-background-networking",
    "--disable-background-timer-throttling",
    "--disable-backgrounding-occluded-windows",
    "--disable-breakpad",
    "--disable-client-side-phishing-detection",
    "--disable-component-extensions-with-background-pages",
    "--disable-default-apps",
    "--disable-dev-shm-usage",
    "--disable-extensions",
    "--disable-features=TranslateUI",
    "--disable-hang-monitor",
    "--disable-ipc-flooding-protection",
    "--disable-popup-blocking",
    "--disable-prompt-on-repost",
    "--disable-renderer-backgrounding",
    "--disable-sync",
    "--no-first-run",
    "--password-store=basic",
    "--use-mock-keychain",
    "--disable-blink-features",
    "--disable-blink-features=AutomationControlled",
    "--mute-audio",
    "--incognito",
    "--headless",
    // NB: This flag is required in some CI environments.
    "--no-sandbox",
  ];

  let chromedriver = Chromedriver::launch().unwrap();
  let webdriver_url = format!("http://{}", chromedriver.socket_addr());
  let data_dir = tempdir().unwrap();
  let mut args = Vec::from(chromedriver_args);
  let data_dir_arg = format!("--user-data-dir={}", data_dir.path().display());
  let () = args.push(&data_dir_arg);

  let opts = json!({"args": args});
  let mut capabilities = Capabilities::new();
  let _val = capabilities.insert("goog:chromeOptions".to_string(), opts);

  let client = ClientBuilder::native()
    .capabilities(capabilities)
    .connect(&webdriver_url)
    .await
    .unwrap();

  let () = client
    .goto("https://en.wikipedia.org/wiki/Foobar")
    .await
    .unwrap();
  let url = client.current_url().await.unwrap();
  assert_eq!(url.as_ref(), "https://en.wikipedia.org/wiki/Foobar");
}
