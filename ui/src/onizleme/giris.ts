// Ekran önizlemesi: yalnızca geliştirme sunucusunda (`npm run dev`,
// http://localhost:5173/onizleme.html). Ekranlar sahte arka uçla çalışır;
// derlemeye girmez. `#sartlar`, `#sartlar-anahtar`, `#sartlar-kilitli` şart
// penceresini açar; `#windows` ana pencereyi Windows'taki gibi gösterir;
// `#sinir` durum çubuğunda EVREN'in günlük sınırını dolu gösterir. Ana
// pencerede "eklenti" geçen bir mesaj eklenti onayını ve önizlemesini getirir.
import { mount } from 'svelte';
import '../stiller';
import Ana from '../ana/Ana.svelte';
import Sartlar from '../sartlar/Sartlar.svelte';
import { anaSenaryosu, sartSenaryosu } from './senaryo';

const kok = document.getElementById('kok');
if (!kok) throw new Error('#kok bulunamadı');

const pencere = location.hash.slice(1);
if (pencere.startsWith('sartlar')) {
  document.body.classList.add('kalfa-light');
  sartSenaryosu(pencere).kur();
  mount(Sartlar, { target: kok });
} else {
  anaSenaryosu(pencere === 'windows', pencere === 'sinir').kur();
  mount(Ana, { target: kok });
}
