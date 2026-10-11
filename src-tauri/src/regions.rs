// [INPUT]: 用户选定的国家地区编号。
// [OUTPUT]: 共用的地区、语言、区域格式与代表时区目录、白名单查询及目标地区匹配。
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
    fn catalog_has_unique_ids_and_valid_native_timezones() {
        let machine = crate::browser::config::machine().unwrap();
        for (i, r) in REGIONS.iter().enumerate() {
            assert!(!REGIONS[..i].iter().any(|other| other.id == r.id));
            assert!(machine.timezones.iter().any(|zone| zone == r.timezone));
        }
        assert!(find("us; touch /tmp/injected").is_err());
    }
}

// 通过既有语言与时区识别目标地区；自定义组合无法识别时保持未核验。
pub fn matching(language: &str, timezone: &str) -> Option<&'static Region> {
    REGIONS
        .iter()
        .find(|r| r.language == language && r.timezone == timezone)
}
