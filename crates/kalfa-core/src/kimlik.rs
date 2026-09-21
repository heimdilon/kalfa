//! Oturum, tur, araç çağrısı ve onay işlemi kimlikleri.
//!
//! Araç çağrısı kimliği (`CagriId`) modelin verdiği `tool_call.id` değil,
//! Kalfa'nın ürettiği UUID'dir: model kimlikleri tekrar edebilir ve onay
//! mührü benzersiz bir kimliğe bağlanmalıdır.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! kimlik {
    ($(#[$belge:meta])* $ad:ident) => {
        $(#[$belge])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $ad(pub Uuid);

        impl $ad {
            pub fn yeni() -> Self {
                Self(Uuid::new_v4())
            }
        }

        impl std::fmt::Display for $ad {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

kimlik!(/// Bir oturumun kimliği.
    OturumId);
kimlik!(/// Bir kullanıcı mesajıyla başlayan turun kimliği.
    TurId);
kimlik!(/// Kalfa'nın bir araç çağrısına verdiği kimlik.
    CagriId);
kimlik!(/// Tek bir onay kartında gruplanan işlemin kimliği.
    IslemId);
