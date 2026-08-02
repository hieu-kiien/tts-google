use regex::Regex;
use std::sync::LazyLock;
use tracing::info;

static ABBR_REGEXES: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    vec![
        (Regex::new(r"\bTP\.HCM\b").unwrap(), "Thành phố Hồ Chí Minh"),
        (Regex::new(r"\bTp\.HCM\b").unwrap(), "Thành phố Hồ Chí Minh"),
        (Regex::new(r"\bTPHCM\b").unwrap(), "Thành phố Hồ Chí Minh"),
        (Regex::new(r"\bHN\b").unwrap(), "Hà Nội"),
        (Regex::new(r"\bTHPT\b").unwrap(), "Trung học phổ thông"),
        (Regex::new(r"\bTHCS\b").unwrap(), "Trung học cơ sở"),
        (Regex::new(r"\bĐH\b").unwrap(), "Đại học"),
        (Regex::new(r"\bUBND\b").unwrap(), "Ủy ban nhân dân"),
        (Regex::new(r"\bBCH\b").unwrap(), "Ban chấp hành"),
        (Regex::new(r"\bBTC\b").unwrap(), "Ban tổ chức"),
        (Regex::new(r"\bv\.v\.\.\.").unwrap(), "vân vân"),
        (Regex::new(r"\bv\.v\b").unwrap(), "vân vân"),
        (Regex::new(r"\bvv\.\b").unwrap(), "vân vân"),
        (Regex::new(r"\bKBT\b").unwrap(), "ki-lô-bắt"),
        (Regex::new(r"\bMB\b").unwrap(), "mê-ga-bắt"),
        (Regex::new(r"\bGB\b").unwrap(), "gi-ga-bắt"),
    ]
});

static UNIT_REGEXES: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    vec![
        (
            Regex::new(r"(\d+(?:[\.,]\d+)?)\s*km/h").unwrap(),
            "$1 ki-lô-mét trên giờ",
        ),
        (
            Regex::new(r"(\d+(?:[\.,]\d+)?)\s*m/s").unwrap(),
            "$1 mét trên giây",
        ),
        (
            Regex::new(r"(?i)(\d+(?:[\.,]\d+)?)\s*[^\w\s]?\s*C\b").unwrap(),
            "$1 độ C",
        ),
        (
            Regex::new(r"(\d+(?:[\.,]\d+)?)\s*cm\b").unwrap(),
            "$1 xen-ti-mét",
        ),
        (
            Regex::new(r"(\d+(?:[\.,]\d+)?)\s*mm\b").unwrap(),
            "$1 mi-li-mét",
        ),
        (
            Regex::new(r"(\d+(?:[\.,]\d+)?)\s*kg\b").unwrap(),
            "$1 ki-lô-gam",
        ),
        (Regex::new(r"(\d+(?:[\.,]\d+)?)\s*g\b").unwrap(), "$1 gam"),
        (
            Regex::new(r"(\d+(?:[\.,]\d+)?)\s*m²").unwrap(),
            "$1 mét vuông",
        ),
        (
            Regex::new(r"(\d+(?:[\.,]\d+)?)\s*m³").unwrap(),
            "$1 mét khối",
        ),
    ]
});

static CURRENCY_VND: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\d+(?:[\.,]\d+)*)\s*(?:VND|VNĐ|\bđ\b|đồng\b)").unwrap());
static CURRENCY_USD_1: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\$\s*(\d+(?:[\.,]\d+)*)").unwrap());
static CURRENCY_USD_2: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\d+(?:[\.,]\d+)*)\s*USD").unwrap());
static CURRENCY_EUR_1: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"€\s*(\d+(?:[\.,]\d+)*)").unwrap());
static CURRENCY_EUR_2: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\d+(?:[\.,]\d+)*)\s*EUR").unwrap());

static DATE_1: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(\d{1,2})[/.-](\d{1,2})[/.-](\d{4})\b").unwrap());
static DATE_2: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\btháng\s*(\d{1,2})[/.-](\d{4})\b").unwrap());

static TIME_1: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(\d{1,2})h(\d{1,2})p?\b").unwrap());
static TIME_2: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b(\d{1,2})h\b").unwrap());

static PERCENTAGE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\d+(?:[\.,]\d+)?)\s*%").unwrap());

static ROMAN_REGEXES: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    vec![
        (Regex::new(r"\bXXI\b").unwrap(), "21"),
        (Regex::new(r"\bXX\b").unwrap(), "20"),
        (Regex::new(r"\bXIX\b").unwrap(), "19"),
        (Regex::new(r"\bXVIII\b").unwrap(), "18"),
        (Regex::new(r"\bXVII\b").unwrap(), "17"),
        (Regex::new(r"\bXVI\b").unwrap(), "16"),
        (Regex::new(r"\bXV\b").unwrap(), "15"),
        (Regex::new(r"\bXIV\b").unwrap(), "14"),
        (Regex::new(r"\bXIII\b").unwrap(), "13"),
        (Regex::new(r"\bXII\b").unwrap(), "12"),
        (Regex::new(r"\bXI\b").unwrap(), "11"),
        (Regex::new(r"\bX\b").unwrap(), "10"),
        (Regex::new(r"\bIX\b").unwrap(), "9"),
        (Regex::new(r"\bVIII\b").unwrap(), "8"),
        (Regex::new(r"\bVII\b").unwrap(), "7"),
        (Regex::new(r"\bVI\b").unwrap(), "6"),
        (Regex::new(r"\bV\b").unwrap(), "5"),
        (Regex::new(r"\bIV\b").unwrap(), "4"),
        (Regex::new(r"\bIII\b").unwrap(), "3"),
        (Regex::new(r"\bII\b").unwrap(), "2"),
        (Regex::new(r"\bI\b").unwrap(), "1"),
    ]
});

pub struct VietnameseNormalizer;

impl VietnameseNormalizer {
    pub fn normalize(text: &str) -> String {
        info!("Running Vietnamese Text Normalizer");
        let mut result = text.to_string();

        // 1. Abbreviation normalization (e.g. TP.HCM -> Thành phố Hồ Chí Minh, v.v. -> vân vân)
        result = Self::normalize_abbreviations(&result);

        // 2. Units & Scientific symbols (e.g. 100km/h -> 100 ki-lô-mét trên giờ, 35°C -> 35 độ C)
        result = Self::normalize_units(&result);

        // 3. Currency normalization (e.g. 1.500.000 VNĐ / 1.500.000đ / $50 / €100)
        result = Self::normalize_currencies(&result);

        // 4. Date normalization (e.g. 24/07/2026 -> ngày 24 tháng 7 năm 2026)
        result = Self::normalize_dates(&result);

        // 5. Time normalization (e.g. 14h30 -> 14 giờ 30 phút)
        result = Self::normalize_times(&result);

        // 6. Percentage normalization (e.g. 85% -> 85 phần trăm)
        result = Self::normalize_percentages(&result);

        // 7. Roman numerals (e.g. thế kỷ XXI -> thế kỷ 21)
        result = Self::normalize_roman_numerals(&result);

        result
    }

    fn normalize_abbreviations(text: &str) -> String {
        let mut res = text.to_string();
        for (re, replacement) in &*ABBR_REGEXES {
            res = re.replace_all(&res, *replacement).to_string();
        }
        res
    }

    fn normalize_units(text: &str) -> String {
        let mut res = text.to_string();
        for (re, replacement) in &*UNIT_REGEXES {
            res = re.replace_all(&res, *replacement).to_string();
        }
        res
    }

    fn normalize_currencies(text: &str) -> String {
        let mut res = text.to_string();
        res = CURRENCY_VND
            .replace_all(&res, "$1 Việt Nam đồng")
            .to_string();
        res = CURRENCY_USD_1.replace_all(&res, "$1 đô la Mỹ").to_string();
        res = CURRENCY_USD_2.replace_all(&res, "$1 đô la Mỹ").to_string();
        res = CURRENCY_EUR_1.replace_all(&res, "$1 ơ-rô").to_string();
        res = CURRENCY_EUR_2.replace_all(&res, "$1 ơ-rô").to_string();
        res
    }

    fn normalize_dates(text: &str) -> String {
        let mut res = text.to_string();
        res = DATE_1
            .replace_all(&res, "ngày $1 tháng $2 năm $3")
            .to_string();
        res = DATE_2.replace_all(&res, "tháng $1 năm $2").to_string();
        res
    }

    fn normalize_times(text: &str) -> String {
        let mut res = text.to_string();
        res = TIME_1.replace_all(&res, "$1 giờ $2 phút").to_string();
        res = TIME_2.replace_all(&res, "$1 giờ").to_string();
        res
    }

    fn normalize_percentages(text: &str) -> String {
        let mut res = text.to_string();
        res = PERCENTAGE.replace_all(&res, "$1 phần trăm").to_string();
        res
    }

    fn normalize_roman_numerals(text: &str) -> String {
        let mut res = text.to_string();
        for (re, arabic) in &*ROMAN_REGEXES {
            res = re.replace_all(&res, *arabic).to_string();
        }
        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_abbreviations() {
        let input = "Học sinh THPT tại TP.HCM gửi đơn lên UBND thành phố v.v...";
        let output = VietnameseNormalizer::normalize(input);
        assert!(output.contains("Trung học phổ thông"));
        assert!(output.contains("Thành phố Hồ Chí Minh"));
        assert!(output.contains("Ủy ban nhân dân"));
        assert!(output.contains("vân vân"));
    }

    #[test]
    fn test_normalize_units() {
        let input = "Xe chạy 60km/h trong thời tiết 35°C và nặng 1500kg.";
        let output = VietnameseNormalizer::normalize(input);
        assert!(output.contains("60 ki-lô-mét trên giờ"));
        assert!(output.contains("35 độ C"));
        assert!(output.contains("1500 ki-lô-gam"));
    }

    #[test]
    fn test_normalize_currencies_and_dates() {
        let input =
            "Giá sản phẩm là 2.500.000 VNĐ, mua vào ngày 24/07/2026 lúc 14h30 với giảm giá 15%.";
        let output = VietnameseNormalizer::normalize(input);
        assert!(output.contains("Việt Nam đồng"));
        assert!(output.contains("ngày 24 tháng 07 năm 2026"));
        assert!(output.contains("14 giờ 30 phút"));
        assert!(output.contains("15 phần trăm"));
    }

    #[test]
    fn test_normalize_usd_and_eur() {
        let input = "Giá $50 hoặc 100 USD hoặc €20 hoặc 30 EUR.";
        let output = VietnameseNormalizer::normalize(input);
        assert!(output.contains("50 đô la Mỹ"));
        assert!(output.contains("100 đô la Mỹ"));
        assert!(output.contains("20 ơ-rô"));
        assert!(output.contains("30 ơ-rô"));
    }

    #[test]
    fn test_normalize_roman_numerals() {
        let input = "Sống ở thế kỷ XXI vào triều đại XIX.";
        let output = VietnameseNormalizer::normalize(input);
        assert!(output.contains("thế kỷ 21"));
        assert!(output.contains("triều đại 19"));
    }

    #[test]
    fn test_normalize_times_short() {
        let input = "Cuộc họp diễn ra lúc 8h sáng.";
        let output = VietnameseNormalizer::normalize(input);
        assert!(output.contains("8 giờ sáng"));
    }
}
