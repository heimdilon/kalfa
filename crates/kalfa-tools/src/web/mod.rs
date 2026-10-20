//! Kalfa web getirme (`web_getir`), izin/mühür, HTML/metin işleme ve
//! kaynaklı arama doğrulama modülü (Yetenek-05: Web, Araştırma ve Güncel Bilgi).

pub mod ag;
pub mod arac;
pub mod arama;
pub mod html;
pub mod izin;
pub mod kaynak;
pub mod metin;
pub mod sikistirma;

pub use ag::{
    AZAMI_YONLENDIRME, AdresRedGerekcesi, BAGLANTI_ZAMAN_ASIMI, Cozucu, SistemCozucu,
    TOPLAM_ZAMAN_ASIMI, TestFixtureIzni, WebGetirHatasi, WebHamYanit, WebIstemci, YonlendirmeKaydi,
    adres_denetle, naif_url_on_kontrolu, url_dogrula,
};
pub use arac::{WEB_ARAC_ADI, WebGetirAraci};
pub use arama::{
    AramaHatasi, AramaSaglayiciYaniti, AramaSonucOgesi, DogrulanmisAramaSonucu, KaynakliIddia,
    KaynakliYanitHatasi, OkunanKaynakSayfa, arama_kaynaklarini_oku, arama_sonuclarini_dogrula,
    kaynakli_yanit_dogrula, mcp_arac_sonucunu_coz, mcp_arama_json_coz,
};
pub use html::{
    HtmlBaglantisi, HtmlBelgesi, HtmlBolumu, html_cozumle, html_meta_karakter_kumesi_bul,
    html_varliklarini_coz,
};
pub use izin::{
    WEB_AG_UYARISI, WEB_ILK_HOST_UYARISI, WEB_SIFRESIZ_HTTP_UYARISI, WEB_YONLENDIRME_UYARISI,
    WebIzinDefteri, WebTarifi, YonlendirmeIzni, web_karari, web_muhur_ozeti,
};
pub use kaynak::{WebSayfaOzeti, dis_icerik_zarfla, url_maskele, url_nesnesi_maskele};
pub use metin::{
    AZAMI_CIKTI_BAYT, EN_AZ_CIKTI_BAYT, IcerikBicimi, ImlecDurumu, IslenmisWebIcerigi,
    baytlari_metne_coz, icerik_ozeti_hesapla, icerik_turu_kabul_edilir_mi, ikili_mi, imlec_coz,
    imlec_olustur, yaniti_metne_donustur,
};
pub use sikistirma::{
    AZAMI_ACILMIS_GOVDE_BAYT, AkisAcici, SikistirmaBicimi, SikistirmaHatasi, ac_sinirli,
    gzip_bomba_uret, gzip_paketle, sikistirma_bicimi_bul,
};
