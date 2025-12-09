use std::env;
use std::time::Duration;

use anyhow::{Result, anyhow};
use opentelemetry::KeyValue;
use opentelemetry::metrics::MeterProvider;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
use scraper::{Html, Selector};

#[tokio::main]
async fn main() -> Result<()> {
    let packages = packages_from_env()?;

    for package in packages {
        let records = get_downloads(&package).await?;
        export_metrics(&package, &records)?;
    }

    Ok(())
}

fn packages_from_env() -> Result<Vec<String>> {
    let raw = env::var("PACKAGES")
        .map_err(|_| anyhow!("Set PACKAGES with comma-separated package names"))?;

    let packages: Vec<String> = raw
        .split(',')
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .map(|p| p.to_string())
        .collect();

    if packages.is_empty() {
        return Err(anyhow!("No package names found in PACKAGES"));
    }

    Ok(packages)
}

async fn get_downloads(package_name: &str) -> Result<Vec<Record>> {
    let url = format!(
        "https://www.npmjs.com/package/{}?activeTab=versions",
        package_name
    );

    let html = reqwest::Client::new()
        .get(url)
        .header(
            "User-Agent",
            "github.com/toof-jp/npm-download-stats-otel-exporter",
        )
        .send()
        .await?
        .text()
        .await?;

    parse_html(&html)
}

fn parse_html(html: &str) -> Result<Vec<Record>> {
    let document = Html::parse_document(html);
    let tr_selector =
        Selector::parse("#tabpanel-versions > div > table:nth-child(5) > tbody > tr").unwrap();
    let version_selector = Selector::parse("td:nth-child(1) > a").unwrap();
    let downloads_selector = Selector::parse("td:nth-child(2)").unwrap();

    document
        .select(&tr_selector)
        .map(|tr| {
            let version = tr
                .select(&version_selector)
                .next()
                .ok_or_else(|| anyhow!("version cell not found"))?
                .inner_html();

            let downloads_text = tr
                .select(&downloads_selector)
                .next()
                .ok_or_else(|| anyhow!("downloads cell not found"))?
                .inner_html();

            let downloads = downloads_text
                .replace(",", "")
                .parse::<u64>()
                .map_err(|e| anyhow!("failed to parse downloads '{downloads_text}': {e}"))?;

            Ok(Record { version, downloads })
        })
        .collect()
}

fn export_metrics(package: &str, records: &[Record]) -> Result<()> {
    let endpoint = env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4317".to_string());

    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()?;

    let reader = PeriodicReader::builder(exporter)
        .with_interval(Duration::from_secs(5))
        .build();

    let resource = Resource::builder()
        .with_attributes(vec![KeyValue::new(
            "service.name",
            "npm-download-stats-otel-exporter",
        )])
        .build();

    let provider = SdkMeterProvider::builder()
        .with_resource(resource)
        .with_reader(reader)
        .build();

    let meter = provider.meter("npm-download-stats-otel-exporter");
    let gauge = meter
        .u64_gauge("npm.package.downloads")
        .with_description("NPM downloads per package version")
        .build();

    let package_attr = KeyValue::new("package", package.to_string());
    for record in records {
        gauge.record(
            record.downloads,
            &[
                package_attr.clone(),
                KeyValue::new("version", record.version.clone()),
            ],
        );
    }

    provider
        .force_flush()
        .map_err(|e| anyhow!("OTLP metrics flush failed: {e}"))?;
    provider
        .shutdown()
        .map_err(|e| anyhow!("OTLP metrics shutdown failed: {e}"))?;

    Ok(())
}

#[derive(Debug, Clone)]
struct Record {
    version: String,
    downloads: u64,
}
