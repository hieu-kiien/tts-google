use regex::Regex;
use tracing::info;

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

        let abbr_map = [
            (r"\bTP\.HCM\b", "Thành phố Hồ Chí Minh"),
            (r"\bTp\.HCM\b", "Thành phố Hồ Chí Minh"),
            (r"\bTPHCM\b", "Thành phố Hồ Chí Minh"),
            (r"\bHN\b", "Hà Nội"),
            (r"\bTHPT\b", "Trung học phổ thông"),
            (r"\bTHCS\b", "Trung học cơ sở"),
            (r"\bĐH\b", "Đại học"),
            (r"\bUBND\b", "Ủy ban nhân dân"),
            (r"\bBCH\b", "Ban chấp hành"),
            (r"\bBTC\b", "Ban tổ chức"),
            (r"\bv\.v\.\.\.", "vân vân"),
            (r"\bv\.v\b", "vân vân"),
            (r"\bvv\.\b", "vân vân"),
            (r"\bKBT\b", "ki-lô-bắt"),
            (r"\bMB\b", "mê-ga-bắt"),
            (r"\bGB\b", "gi-ga-bắt"),
        ];

        for (pattern, replacement) in abbr_map {
            if let Ok(re) = Regex::new(pattern) {
                res = re.replace_all(&res, replacement).to_string();
            }
        }

        res
    }

    fn normalize_units(text: &str) -> String {
        let mut res = text.to_string();

        let unit_map = [
            (r"(\d+(?:[\.,]\d+)?)\s*km/h", "$1 ki-lô-mét trên giờ"),
            (r"(\d+(?:[\.,]\d+)?)\s*m/s", "$1 mét trên giây"),
            (r"(?i)(\d+(?:[\.,]\d+)?)\s*[^\w\s]?\s*C\b", "$1 độ C"),
            (r"(\d+(?:[\.,]\d+)?)\s*cm\b", "$1 xen-ti-mét"),
            (r"(\d+(?:[\.,]\d+)?)\s*mm\b", "$1 mi-li-mét"),
            (r"(\d+(?:[\.,]\d+)?)\s*kg\b", "$1 ki-lô-gam"),
            (r"(\d+(?:[\.,]\d+)?)\s*g\b", "$1 gam"),
            (r"(\d+(?:[\.,]\d+)?)\s*m²", "$1 mét vuông"),
            (r"(\d+(?:[\.,]\d+)?)\s*m³", "$1 mét khối"),
        ];

        for (pattern, replacement) in unit_map {
            if let Ok(re) = Regex::new(pattern) {
                res = re.replace_all(&res, replacement).to_string();
            }
        }

        res
    }



    fn normalize_currencies(text: &str) -> String {
        let mut res = text.to_string();

        // VND / VN\u{0110} / \b\u{0111}\b / đồng
        if let Ok(re) = Regex::new(r"(\d+(?:[\.,]\d+)*)\s*(?:VND|VNĐ|\bđ\b|đồng\b)") {
            res = re.replace_all(&res, "$1 Việt Nam đồng").to_string();
        }


        // USD / $
        if let Ok(re) = Regex::new(r"\$\s*(\d+(?:[\.,]\d+)*)") {
            res = re.replace_all(&res, "$1 đô la Mỹ").to_string();
        }
        if let Ok(re) = Regex::new(r"(\d+(?:[\.,]\d+)*)\s*USD") {
            res = re.replace_all(&res, "$1 đô la Mỹ").to_string();
        }

        // EUR / €
        if let Ok(re) = Regex::new(r"€\s*(\d+(?:[\.,]\d+)*)") {
            res = re.replace_all(&res, "$1 ơ-rô").to_string();
        }
        if let Ok(re) = Regex::new(r"(\d+(?:[\.,]\d+)*)\s*EUR") {
            res = re.replace_all(&res, "$1 ơ-rô").to_string();
        }

        res
    }

    fn normalize_dates(text: &str) -> String {
        let mut res = text.to_string();
        // DD/MM/YYYY or DD-MM-YYYY
        if let Ok(re) = Regex::new(r"\b(\d{1,2})[/.-](\d{1,2})[/.-](\d{4})\b") {
            res = re.replace_all(&res, "ngày $1 tháng $2 năm $3").to_string();
        }
        // MM/YYYY
        if let Ok(re) = Regex::new(r"\btháng\s*(\d{1,2})[/.-](\d{4})\b") {
            res = re.replace_all(&res, "tháng $1 năm $2").to_string();
        }
        res
    }

    fn normalize_times(text: &str) -> String {
        let mut res = text.to_string();
        // 14h30 or 14h30p
        if let Ok(re) = Regex::new(r"\b(\d{1,2})h(\d{1,2})p?\b") {
            res = re.replace_all(&res, "$1 giờ $2 phút").to_string();
        }
        // 14h
        if let Ok(re) = Regex::new(r"\b(\d{1,2})h\b") {
            res = re.replace_all(&res, "$1 giờ").to_string();
        }
        res
    }

    fn normalize_percentages(text: &str) -> String {
        let mut res = text.to_string();
        if let Ok(re) = Regex::new(r"(\d+(?:[\.,]\d+)?)\s*%") {
            res = re.replace_all(&res, "$1 phần trăm").to_string();
        }
        res
    }

    fn normalize_roman_numerals(text: &str) -> String {
        let mut res = text.to_string();
        let map = [
            ("XXI", "21"), ("XX", "20"), ("XIX", "19"), ("XVIII", "18"),
            ("XVII", "17"), ("XVI", "16"), ("XV", "15"), ("XIV", "14"),
            ("XIII", "13"), ("XII", "12"), ("XI", "11"), ("X", "10"),
            ("IX", "9"), ("VIII", "8"), ("VII", "7"), ("VI", "6"),
            ("V", "5"), ("IV", "4"), ("III", "3"), ("II", "2"), ("I", "1"),
        ];

        for (roman, arabic) in map {
            let pattern = format!(r"\b{}\b", roman);
            if let Ok(re) = Regex::new(&pattern) {
                res = re.replace_all(&res, arabic).to_string();
            }
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
        let input = "Giá sản phẩm là 2.500.000 VNĐ, mua vào ngày 24/07/2026 lúc 14h30 với giảm giá 15%.";
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


