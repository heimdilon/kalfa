import { mount } from 'svelte';
import '../stiller';
import Sartlar from './Sartlar.svelte';

const kok = document.getElementById('kok');
if (!kok) throw new Error('#kok bulunamadı');
mount(Sartlar, { target: kok });
