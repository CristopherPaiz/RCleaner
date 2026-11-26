<script lang="ts">
  // Interruptor de encendido/apagado.
  interface Props {
    checked: boolean;
    label: string;
    disabled?: boolean;
    onchange: (value: boolean) => void;
  }
  let { checked, label, disabled = false, onchange }: Props = $props();
</script>

<button
  class="switch"
  class:on={checked}
  role="switch"
  aria-checked={checked}
  aria-label={label}
  {disabled}
  onclick={() => onchange(!checked)}
>
  <span class="knob"></span>
</button>

<style>
  .switch {
    position: relative; width: 40px; height: 22px; flex-shrink: 0; border-radius: 99px; cursor: pointer; padding: 0;
    border: 1px solid var(--line-2); background: var(--raise-2); transition: background 0.15s, border-color 0.15s;
  }
  .switch.on { background: linear-gradient(135deg, var(--a), var(--b)); border-color: transparent; }
  .knob {
    position: absolute; top: 2px; left: 2px; width: 16px; height: 16px; border-radius: 50%;
    background: var(--text-2); transition: transform 0.15s, background 0.15s;
  }
  .switch.on .knob { transform: translateX(18px); background: #fff; }
  .switch:disabled { opacity: 0.45; cursor: default; }
</style>
