//! Uyumluluk testinin örnek crate'i.

pub fn onaltilik(veri: &[u8]) -> String {
    hex::encode(veri)
}

#[cfg(test)]
mod tests {
    #[test]
    fn onaltilik_yazar() {
        assert_eq!(super::onaltilik(b"kalfa"), "6b616c6661");
    }
}
