use std::error::Error;
use std::iter;

use chrono::{DateTime, Utc};
use clap::Parser;
use serde::Deserialize;
use termimad::{Area, MadSkin};
use tokio;


#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[clap(default_value = "google_bigquery_dataset_access")]
    resource: String,
}

impl Args {
    fn get_provider_and_resource(&self) -> Option<(String, String)> {
        let mut sp = self.resource.split("_");
        let provider = sp.next()?;
        let resource = sp
            .zip(iter::repeat("_"))
            .flat_map(|(a,b)| vec![b, a])
            .skip(1)
            .collect();
        Some((provider.to_string(), resource))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Args = Args::parse();
    let (provider, resource) = args.get_provider_and_resource()
        .ok_or("provider not found")?;
    let provider_id = get_provider(provider).await?.id;
    let version_id = get_latest_version(provider_id).await?;
    let resource_docs = get_docs(resource, version_id).await?;
    termimad::print_text(resource_docs.attributes.content.as_str());
    Ok(())
}

async fn get_provider(provider: String) -> Result<Resource, Box<dyn Error>> {
    Ok(reqwest::get(
        format!("https://registry.terraform.io/v2/providers\
                ?filter%5Bnamespace%5D=hashicorp&filter%5Bname%5D={provider}"))
        .await?
        .json::<DataResponse<Vec<Resource>>>()
        .await?
        .data
        .first().ok_or("provider not found")?
        .clone())
}

async fn get_latest_version(provider_id: String) -> Result<String, Box<dyn Error>> {
    Ok(reqwest::get(
        format!("https://registry.terraform.io/v2/providers/{provider_id}/provider-versions"))
        .await?
        .json::<DataResponse<Vec<Resource<VersionAttributes>>>>()
        .await?
        .data
        // Assume order...
        .last().ok_or("No version found...")?
        .id
        .clone())
}

async fn get_docs(resource: String, version_id: String) -> Result<Resource<DocsAttributes>, Box<dyn Error>> {
    let docs_rsp =
        reqwest::get(format!("https://registry.terraform.io/v2/provider-docs\
            ?filter%5Bprovider-version%5D={version_id}\
            &filter%5Bcategory%5D=resources\
            &filter%5Bslug%5D={resource}"))
        .await?
        .json::<DataResponse<Vec<Resource>>>()
        .await?;
    let doc_result = docs_rsp.data.first().ok_or("resource not found")?;
    Ok(reqwest::get("https://registry.terraform.io".to_string() + doc_result.links._self.as_str())
        .await?
        .json::<DataResponse<Resource<DocsAttributes>>>()
        .await?
        .data)
}

#[derive(Deserialize, Debug, Clone)]
struct DataResponse<T> {
    data: T,
}

#[derive(Deserialize, Debug, Clone)]
struct Resource<A = Empty> {
    id: String,
    #[serde(rename(deserialize = "type"))]
    _type: String,
    links: Links,
    attributes: A
}

#[derive(Deserialize, Debug, Clone)]
struct Empty{}

#[derive(Deserialize, Debug, Clone)]
struct VersionAttributes {
    description: String,
    downloads: usize,
    #[serde(rename(deserialize = "published-at"))]
    published_at: DateTime<Utc>,
    tag: String,
    version: String
}

#[derive(Deserialize, Debug, Clone)]
struct DocsAttributes {
    content: String,
    category: String,
    slug: String,
    subcategory: String,
    title: String
}

#[derive(Deserialize, Debug, Clone)]
struct Links {
    #[serde(rename(deserialize = "self"))]
    _self: String
}

#[derive(Deserialize, Debug, Clone)]
struct IdAndType {
    id: String,
    #[serde(rename(deserialize = "type"))]
    type_: String,
}
