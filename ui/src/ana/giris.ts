import { mount } from 'svelte';
import '../stiller';
import Ana from './Ana.svelte';

const kok = document.getElementById('kok');
if (!kok) throw new Error('#kok bulunamadı');
mount(Ana, { target: kok });
