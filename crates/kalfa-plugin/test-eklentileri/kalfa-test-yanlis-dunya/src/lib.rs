//! Sözleşmeye uymayan dünya (aşama 4 tasarımı §16.1): `calistir` farklı
//! imzalı. Yükleme `InstancePre` adımında düşmeli.

wit_bindgen::generate!({
    inline: r#"
        package kalfa:plugin@0.1.0;

        interface arac {
          record arac-tanimi { ad: string, aciklama: string, parametre-semasi: string }
          araclari-listele: func() -> list<arac-tanimi>;
          calistir: func(ad: string) -> string;
        }

        world eklenti {
          export arac;
        }
    "#,
});

use exports::kalfa::plugin::arac::{AracTanimi, Guest};

struct YanlisDunya;

impl Guest for YanlisDunya {
    fn araclari_listele() -> Vec<AracTanimi> {
        vec![AracTanimi {
            ad: "yanlis".into(),
            aciklama: "İmzası sözleşmeye uymaz.".into(),
            parametre_semasi: r#"{"type":"object"}"#.into(),
        }]
    }

    fn calistir(ad: String) -> String {
        ad
    }
}

export!(YanlisDunya);
