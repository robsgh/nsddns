use anyhow::{anyhow, Context, Result};
use std::{fs, path::PathBuf};

/// Version of the Namesilo public API
const NAMESILO_API_VERSION: u8 = 1;

#[derive(Clone, Debug)]
/// Configuration information for nsddns
pub struct NsddnsConfig {
    /// Domain to modify A records for
    pub domain: String,
    /// Subdomain (or blank if mutating the apex)
    pub subdomain: String,
    /// Namesilo API key for reading/mutating records
    pub api_key: String,
}

#[derive(Clone, Debug)]
/// DNS resource record representation
pub struct NsResourceRecord {
    /// Host for the resource record (domain)
    pub record_host: String,
    /// Value of the resource record
    pub record_value: String,
    /// Namesilo's ID for the resource record
    pub record_id: String,
}

/// Parse the configuration JSON and return a NsddnsConfig struct
pub fn parse_config(cfg: PathBuf) -> Result<NsddnsConfig> {
    let path = cfg.as_path();
    let config_data = fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", cfg.to_string_lossy()))?;

    let config_json = json::parse(&config_data)
        .with_context(|| format!("Failed to parse {} as valid JSON", cfg.to_string_lossy()))?;

    let domain = match config_json["domain"].as_str() {
        Some(domain) => domain.to_owned(),
        None => anyhow::bail!("config missing key: domain"),
    };
    let subdomain = match config_json["subdomain"].as_str() {
        Some(subdomain) => subdomain.to_owned(),
        None => anyhow::bail!("config missing key: subdomain"),
    };
    let api_key = match config_json["api_key"].as_str() {
        Some(api_key) => api_key.to_owned(),
        None => anyhow::bail!("config missing key: api_key"),
    };

    Ok(NsddnsConfig {
        domain,
        subdomain,
        api_key,
    })
}

/// Parse the XML data into a vec of resource records for a namesilo listDns response
fn parse_namesilo_a_records_xml(xml_data: String) -> Result<Vec<NsResourceRecord>> {
    let api_response = roxmltree::Document::parse(&xml_data)?;
    let rrs = api_response
        .descendants()
        .filter(|n| n.has_tag_name("resource_record"));

    let mut resource_records = Vec::new();
    for rr in rrs {
        if !rr
            .descendants()
            .any(|n| n.has_tag_name("type") && n.text() == Some("A"))
        {
            continue;
        }

        let record_host = rr
            .descendants()
            .find(|n| n.has_tag_name("host"))
            .unwrap()
            .text()
            .unwrap()
            .to_owned();
        let record_value = rr
            .descendants()
            .find(|n| n.has_tag_name("value"))
            .unwrap()
            .text()
            .unwrap()
            .to_owned();
        let record_id = rr
            .descendants()
            .find(|n| n.has_tag_name("record_id"))
            .unwrap()
            .text()
            .unwrap()
            .to_owned();

        resource_records.push(NsResourceRecord {
            record_host,
            record_value,
            record_id,
        });
    }

    Ok(resource_records)
}

/// Get the resource record for a domain based on the NsddnsConfig
pub fn get_namesilo_a_record(config: &NsddnsConfig) -> Result<NsResourceRecord> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .get("https://www.namesilo.com/api/dnsListRecords")
        .query(&[("version", NAMESILO_API_VERSION)])
        .query(&[
            ("type", "xml"),
            ("key", config.api_key.as_str()),
            ("domain", config.domain.as_str()),
        ])
        .send()?
        .text()?;

    let resource_records = parse_namesilo_a_records_xml(response)?;

    // an empty subdomain means that we should just use the apex domain
    let host = if config.subdomain.is_empty() {
        config.domain.to_owned()
    } else {
        format!("{}.{}", config.subdomain, config.domain)
    };

    let ns_record = match resource_records
        .into_iter()
        .find(|rr| rr.record_host == host)
    {
        Some(rr) => rr,
        None => {
            anyhow::bail!(
                "No matching host record for '{}' in apex domain '{}'",
                host,
                config.domain
            )
        }
    };

    Ok(ns_record)
}

/// Validate that the namesilo response has a code of 300 (success)
fn validate_reply_code(response_xml: &str) -> Result<()> {
    let api_response = roxmltree::Document::parse(response_xml)?;
    if api_response
        .descendants()
        .find(|n| n.has_tag_name("reply"))
        .is_some_and(|r| {
            r.descendants()
                .any(|c| c.has_tag_name("code") && c.text().unwrap_or_default() == "300")
        })
    {
        return Ok(());
    }

    Err(anyhow!("Namesilo API did not return success (code 300)"))
}

/// Update a namesilo resource record to a new value
pub fn update_namesilo_a_record(
    config: &NsddnsConfig,
    resource_record: &NsResourceRecord,
    new_value: &str,
) -> Result<()> {
    let client = reqwest::blocking::Client::new();
    let response_xml = client
        .get("https://www.namesilo.com/api/dnsUpdateRecord")
        .query(&[("version", NAMESILO_API_VERSION)])
        .query(&[
            ("type", "xml"),
            ("key", config.api_key.as_str()),
            ("domain", config.domain.as_str()),
        ])
        .query(&[
            ("rrhost", config.subdomain.as_str()),
            ("rrvalue", new_value),
            ("rrid", resource_record.record_id.as_str()),
        ])
        .send()?
        .text()?;

    validate_reply_code(&response_xml)
}

/// Get the IP of the executing machine from api.ipify.org
pub fn get_current_ip() -> Result<String> {
    let client = reqwest::blocking::Client::new();
    let response = client.get("https://api.ipify.org").send()?.text()?;
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_xml_no_results() -> Result<()> {
        let xml_data = String::from("<namesilo><reply><resource_record><record_id>a1234</record_id><type>CNAME</type><host>hooo</host><value>woooo</value></resource_record></reply></namesilo>");
        let res = parse_namesilo_a_records_xml(xml_data)?;
        assert!(res.is_empty());
        Ok(())
    }

    #[test]
    fn test_parse_xml_one_record() -> Result<()> {
        let xml_data = String::from("<namesilo><reply><resource_record><record_id>a1234</record_id><type>A</type><host>rob</host><value>1234</value></resource_record></reply></namesilo>");
        let res = parse_namesilo_a_records_xml(xml_data)?;
        assert!(res.len() == 1);

        let rr = res.first().unwrap();
        assert_eq!(rr.record_id, "a1234");
        assert_eq!(rr.record_host, "rob");
        assert_eq!(rr.record_value, "1234");

        Ok(())
    }
}
