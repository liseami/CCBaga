// [INPUT]: 用户选定的国家地区编号。
// [OUTPUT]: 含 Chicago 的显式地区预设、有效区域解析与目标地区匹配；机器语言/脚本不作为国家的替代。
// [POS]: 快捷操作的唯一地区真相源；选择地区不代表实际网络出口已改变。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use serde::Serialize;
#[derive(Serialize)]
pub struct Region {
    pub id: &'static str,
    pub label: &'static str,
    pub country: &'static str,
    pub language: &'static str,
    pub locale: &'static str,
    pub timezone: &'static str,
}
const REGIONS: &[Region] = &[
    Region {
        id: "us-west",
        country: "US",
        label: "美国 · 洛杉矶",
        language: "en-US",
        locale: "en_US",
        timezone: "America/Los_Angeles",
    },
    Region {
        id: "us-east",
        country: "US",
        label: "美国 · 纽约",
        language: "en-US",
        locale: "en_US",
        timezone: "America/New_York",
    },
    Region {
        id: "us-central",
        country: "US",
        label: "美国 · 芝加哥",
        language: "en-US",
        locale: "en_US",
        timezone: "America/Chicago",
    },
    Region {
        id: "ca",
        country: "CA",
        label: "加拿大 · 多伦多",
        language: "en-CA",
        locale: "en_CA",
        timezone: "America/Toronto",
    },
    Region {
        id: "gb",
        country: "GB",
        label: "英国 · 伦敦",
        language: "en-GB",
        locale: "en_GB",
        timezone: "Europe/London",
    },
    Region {
        id: "fr",
        country: "FR",
        label: "法国 · 巴黎",
        language: "fr",
        locale: "fr_FR",
        timezone: "Europe/Paris",
    },
    Region {
        id: "de",
        country: "DE",
        label: "德国 · 柏林",
        language: "de",
        locale: "de_DE",
        timezone: "Europe/Berlin",
    },
    Region {
        id: "jp",
        country: "JP",
        label: "日本 · 东京",
        language: "ja",
        locale: "ja_JP",
        timezone: "Asia/Tokyo",
    },
    Region {
        id: "kr",
        country: "KR",
        label: "韩国 · 首尔",
        language: "ko",
        locale: "ko_KR",
        timezone: "Asia/Seoul",
    },
    Region {
        id: "sg",
        country: "SG",
        label: "新加坡",
        language: "en-SG",
        locale: "en_SG",
        timezone: "Asia/Singapore",
    },
    Region {
        id: "tw",
        country: "TW",
        label: "台湾 · 台北",
        language: "zh-Hant",
        locale: "zh_TW",
        timezone: "Asia/Taipei",
    },
    Region {
        id: "hk",
        country: "HK",
        label: "香港",
        language: "zh-Hant",
        locale: "zh_HK",
        timezone: "Asia/Hong_Kong",
    },
    Region {
        id: "cn",
        country: "CN",
        label: "中国 · 上海",
        language: "zh-Hans",
        locale: "zh_CN",
        timezone: "Asia/Shanghai",
    },
    Region {
        id: "au",
        country: "AU",
        label: "澳大利亚 · 悉尼",
        language: "en-AU",
        locale: "en_AU",
        timezone: "Australia/Sydney",
    },
];
pub fn find(id: &str) -> Result<&'static Region, String> {
    REGIONS
        .iter()
        .find(|r| r.id == id)
        .ok_or_else(|| "请选择支持的国家地区。".into())
}
#[tauri::command]
pub fn get_regions() -> &'static [Region] {
    REGIONS
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn effective_country_is_explicit_and_region_overrides_win() {
        for locale in [
            "zh_TW",
            "zh-Hans-TW",
            "zh_CN@rg=twzzzz",
            "zh-CN-u-rg-twzzzz",
            "zh_CN.UTF-8@rg=twzzzz",
        ] {
            assert_eq!(locale_country(locale).as_deref(), Some("TW"), "{locale}");
        }
        assert_eq!(locale_country("en_US.UTF-8").as_deref(), Some("US"));
        assert_eq!(locale_country("zh_Hant_HK").as_deref(), Some("HK"));
        for locale in [
            "zh-Hans",
            "zh",
            "C",
            "",
            "zh_CN@rg=invalid",
            "zh-CN-u-rg-invalid",
            "zh-CN-u-rg",
            "@rg=twzzzz",
            "C@rg=twzzzz",
            "invalid!@rg=twzzzz",
            "zh--CN@rg=twzzzz",
        ] {
            assert_eq!(locale_country(locale), None, "{locale}");
        }
        assert_eq!(locale_country("zh-CN-x-rg-twzzzz").as_deref(), Some("CN"));
        assert_eq!(locale_country("zh-CN-x-u-rg-twzzzz").as_deref(), Some("CN"));
        assert_eq!(
            locale_country("zh-CN-a-test-u-rg-twzzzz").as_deref(),
            Some("TW")
        );
        assert_eq!(
            matching_country("TW", "Asia/Taipei").map(|r| r.id),
            Some("tw")
        );
        assert!(matching_country("CN", "Asia/Taipei").is_none());
        assert!(matching_country("TW", "Asia/Shanghai").is_none());
        assert!(matching("zh-Hans", "Asia/Taipei").is_none());
    }
    #[test]
    fn catalog_has_unique_ids_and_valid_native_timezones() {
        let machine = crate::browser::config::machine().unwrap();
        for (i, r) in REGIONS.iter().enumerate() {
            assert!(!REGIONS[..i].iter().any(|other| other.id == r.id));
            assert!(machine.timezones.iter().any(|zone| zone == r.timezone));
        }
        assert!(find("us; touch /tmp/injected").is_err());
    }
    #[test]
    fn catalog_includes_chicago_without_equating_distinct_iana_zones() {
        assert_eq!(REGIONS.len(), 14);
        let chicago = find("us-central").unwrap();
        assert_eq!(
            (
                chicago.country,
                chicago.language,
                chicago.locale,
                chicago.timezone
            ),
            ("US", "en-US", "en_US", "America/Chicago")
        );
        assert_eq!(
            matching("en-US", "America/Chicago").map(|r| r.id),
            Some(chicago.id)
        );
        assert_eq!(
            matching_country("US", "America/Chicago").map(|r| r.id),
            Some(chicago.id)
        );
        assert!(matching_country("CA", "America/Chicago").is_none());
        assert!(matching("en-CA", "America/Chicago").is_none());
        assert!(matching("en-US", "America/Phoenix").is_none());
        assert!(matching("en-US", "US/Pacific").is_none());
        assert!(matching("de", "Europe/Paris").is_none());
        assert!(matching_country("DE", "Europe/Paris").is_none());
    }
}

// 通过既有语言与时区识别目标地区；自定义组合无法识别时保持未核验。
pub fn matching(language: &str, timezone: &str) -> Option<&'static Region> {
    REGIONS
        .iter()
        .find(|r| r.language == language && r.timezone == timezone)
}

/// Extract an explicit ISO country from ICU/POSIX or BCP 47 locale identifiers.
/// Region overrides take precedence; language/script alone never imply a country.
pub fn locale_country(locale: &str) -> Option<String> {
    fn country(value: &str) -> Option<String> {
        (value.len() == 2 && value.bytes().all(|c| c.is_ascii_alphabetic()))
            .then(|| value.to_ascii_uppercase())
    }
    fn override_country(value: &str) -> Option<String> {
        let value = value.to_ascii_lowercase();
        value.strip_suffix("zzzz").and_then(country)
    }
    let (base, keywords) = locale.split_once('@').unwrap_or((locale, ""));
    let base = base.split('.').next().unwrap_or("");
    let parts: Vec<_> = base.split(['-', '_']).collect();
    let language = *parts.first()?;
    if !(2..=8).contains(&language.len())
        || !language.bytes().all(|c| c.is_ascii_alphabetic())
        || parts
            .iter()
            .any(|p| p.is_empty() || p.len() > 8 || !p.bytes().all(|c| c.is_ascii_alphanumeric()))
    {
        return None;
    }
    let mut index = 1;
    if parts
        .get(index)
        .is_some_and(|p| p.len() == 4 && p.bytes().all(|c| c.is_ascii_alphabetic()))
    {
        index += 1;
    }
    let base_country = parts.get(index).and_then(|p| country(p));
    if base_country.is_some() {
        index += 1;
    }
    for keyword in keywords.split(';') {
        if let Some((key, value)) = keyword.split_once('=') {
            if key.eq_ignore_ascii_case("rg") {
                return override_country(value);
            }
        }
    }
    // Stop at private use; an x-u-rg sequence is not a Unicode extension.
    while index < parts.len() {
        let part = parts[index];
        if part.eq_ignore_ascii_case("x") {
            break;
        }
        if part.eq_ignore_ascii_case("u") {
            let extension: Vec<_> = parts[index + 1..]
                .iter()
                .take_while(|p| p.len() != 1)
                .copied()
                .collect();
            for (offset, key) in extension.iter().enumerate() {
                if key.eq_ignore_ascii_case("rg") {
                    return extension
                        .get(offset + 1)
                        .and_then(|value| override_country(value));
                }
            }
            break;
        }
        index += 1;
    }
    base_country
}

/// Follow the machine's effective country and timezone, independently of language/script.
/// A conflicting/unsupported country or timezone is still unverified.
pub fn matching_country(country: &str, timezone: &str) -> Option<&'static Region> {
    REGIONS
        .iter()
        .find(|r| r.country == country && r.timezone == timezone)
}
