use serde::Deserialize;
#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Product { pub name:String,pub version:String,pub repository_url:String,pub defaults:Defaults,pub onboarding:Onboarding }
#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Defaults {pub interface_percent:u16,pub mini_player_density:String,pub appearance:Look}
#[derive(Deserialize)]
pub struct Look {pub mode:String,pub source:String,pub accent:String,pub saturation:u16,pub tint:u16}
#[derive(Deserialize)]
pub struct Onboarding {pub version:u32}
pub fn config()->&'static Product {static VALUE:std::sync::OnceLock<Product>=std::sync::OnceLock::new();VALUE.get_or_init(||serde_json::from_str(include_str!("../../product.config.json")).expect("Validated product.config.json"))}
pub fn provider_user_agent()->&'static str {static VALUE:std::sync::OnceLock<String>=std::sync::OnceLock::new();VALUE.get_or_init(||format!("{}/{} ({})",config().name,config().version,config().repository_url))}
#[cfg(test)]mod tests {#[test]fn shared_defaults_are_valid(){let p=super::config();assert!(!p.name.is_empty());assert!(p.repository_url.starts_with("https://"));assert_eq!(p.onboarding.version,1);assert!([80,100,125,150].contains(&p.defaults.interface_percent));}}
