// Bileşen vitrini: yalnızca geliştirme sunucusunda (`npm run dev`,
// http://localhost:5173/vitrin.html). Derlemeye girmez.
import { mount } from 'svelte';
import '../stiller';
import Vitrin from './Vitrin.svelte';

const kok = document.getElementById('kok');
if (!kok) throw new Error('#kok bulunamadı');
mount(Vitrin, { target: kok });
