<!-- Ana pencerenin yer tutucusu; ekranlar 1c-3b'de gelir. -->
<script lang="ts">
  import { m } from '../dil/tr';
</script>

<main class="yer-tutucu">
  <h1>{m('uygulama.adi')}</h1>
</main>

<style>
  .yer-tutucu {
    display: grid;
    place-items: center;
    height: 100%;
  }
  h1 {
    font: var(--type-wordmark);
    letter-spacing: var(--tracking-wordmark);
    color: var(--text-title);
  }
</style>
