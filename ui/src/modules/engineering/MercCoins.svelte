<script lang="ts">
 import { t } from '../../lib/i18n';
 import { integer } from '../../lib/format';
 import type { MercCoins } from './types';
 /** Total necesario y saldo; un saldo `null` nunca se muestra como cero. */
 let { coins }: { coins: MercCoins | null | undefined } = $props();
</script>
{#if coins && coins.need > 0}
 <p data-testid="merc-coins">{t('engineering-merc-need', { need: integer(coins.need) })} · {coins.have === null ? t('engineering-merc-balance-unknown') : t('engineering-merc-balance', { have: integer(coins.have) })}</p>
{/if}
