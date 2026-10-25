<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from '../lib/i18n';
  import { diag } from '../lib/diag.svelte';

  let pairingData = $state<{ url: string; qr_svg: string } | null>(null);

  onMount(async () => {
    try {
      const res = await fetch('/api/pairing');
      if (res.ok) {
        pairingData = (await res.json()) as { url: string; qr_svg: string };
      }
      // Solo resultado y forma: nunca la URL de emparejamiento ni el SVG del QR.
      diag('ui.fetch.result', { kind: 'pairing-get', ok: res.ok, status: res.status, rendered: pairingData !== null });
    } catch {
      pairingData = null;
      diag('ui.fetch.result', { kind: 'pairing-get', ok: false, status: 0, rendered: false });
    }
  });
</script>

{#if pairingData}
  <div class="pairing-box">
    <h4>{t('settings-pairing-qr')}</h4>
    <div class="qr-wrap" aria-label={t('settings-pairing-qr')}>
      {@html pairingData.qr_svg}
    </div>
    <code class="pairing-url">{pairingData.url}</code>
  </div>
{/if}

<style>
  .pairing-box {
    margin-top: 0;
    padding: 0.85rem 1rem;
    background: var(--hud-surface-card, rgba(20, 25, 35, 0.5));
    border: 1px solid var(--hud-border-subtle, rgba(255, 255, 255, 0.08));
    border-radius: 6px;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.6rem;
  }

  .pairing-box h4 {
    margin: 0;
    color: var(--hud-primary, #ff7100);
    font-size: 0.88rem;
    font-weight: 700;
    letter-spacing: 0.03em;
  }

  .qr-wrap {
    background: #ffffff;
    padding: 0.5rem;
    border-radius: 4px;
    max-width: 200px;
  }

  .qr-wrap :global(svg) {
    display: block;
    width: 160px;
    height: 160px;
  }

  .pairing-url {
    font-family: var(--font-mono);
    font-size: 0.8rem;
    color: var(--hud-cyan, #00d4b2);
    word-break: break-all;
  }
</style>
